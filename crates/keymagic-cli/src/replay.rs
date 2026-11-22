//! Replays corpus cases through the engine and prints a result table.

use crate::resolver::DirResolver;
use crate::style;
use keymagic_engine::{Action, Engine, KeyEvent, Modifiers, Output, VirtualKey};
use crate::json::Json;
use keymagic_kms::Keyboard;
use std::fmt::Debug;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;
use toml::{Table, Value};

struct Case {
    name: String,
    keyboard: String,
    events: Vec<Event>,
}

struct Event {
    vk: Option<u16>,
    key: Option<String>,
    modifiers: Vec<String>,
    reset: bool,
    set_composing: Option<String>,
    preview: bool,
    action: Option<String>,
    delete: Option<usize>,
    text: Option<String>,
    composing: Option<String>,
    processed: Option<bool>,
    engine_composing: Option<String>,
}

/// What one event produced, in the fields the corpus records.
struct Observed {
    action: String,
    delete: usize,
    text: String,
    composing: String,
    processed: bool,
    engine_composing: String,
}

struct Outcome {
    name: String,
    failures: Vec<String>,
}

/// Replays every case in `cases_dir`. Keyboard paths resolve from its parent.
pub fn run(cases_dir: &Path, json: bool) -> Result<ExitCode, String> {
    let cases = load_cases(cases_dir)?;
    if cases.is_empty() {
        return Err(format!("no cases found in {}", cases_dir.display()));
    }
    let root = corpus_root(cases_dir)?;
    let outcomes: Vec<Outcome> = cases.iter().map(|case| replay(case, &root)).collect();
    let failed = outcomes
        .iter()
        .filter(|outcome| !outcome.failures.is_empty())
        .count();
    let passed = outcomes.len() - failed;
    if json {
        let body = Json::Object(vec![
            ("passed", Json::Number(passed)),
            ("failed", Json::Number(failed)),
            ("cases", Json::Array(outcomes.iter().map(outcome_json).collect())),
        ]);
        println!("{}", body.render());
    } else {
        print_table(&outcomes);
        println!("{passed} passed, {failed} failed");
    }
    Ok(if failed == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

/// Loads a KM2 file, or compiles a KMS file in process.
pub fn load_keyboard(path: &Path) -> Result<Keyboard, String> {
    match path.extension().and_then(|ext| ext.to_str()) {
        Some("km2") => {
            let bytes = fs::read(path)
                .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
            keymagic_km2::parse(&bytes).map_err(|error| format!("{}: {error}", path.display()))
        }
        Some("kms") => {
            let src = fs::read_to_string(path)
                .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
            keymagic_kms::compile(&src, &DirResolver::beside(path)).map_err(|errors| {
                let first = errors.first().map_or_else(
                    || "no diagnostics".to_owned(),
                    |error| error.message.clone(),
                );
                format!("{}: {first}", path.display())
            })
        }
        _ => Err(format!("unsupported keyboard file {}", path.display())),
    }
}

fn load_cases(dir: &Path) -> Result<Vec<Case>, String> {
    let entries =
        fs::read_dir(dir).map_err(|error| format!("cannot read {}: {error}", dir.display()))?;
    let mut paths = Vec::new();
    for entry in entries {
        let path = entry.map_err(|error| error.to_string())?.path();
        if path.extension().is_some_and(|ext| ext == "toml") {
            paths.push(path);
        }
    }
    paths.sort();
    let mut cases = Vec::new();
    for path in paths {
        let text = fs::read_to_string(&path)
            .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
        let table: Table =
            toml::from_str(&text).map_err(|error| format!("{}: {error}", path.display()))?;
        let entries: &[Value] = match table.get("case") {
            Some(Value::Array(items)) => items.as_slice(),
            _ => &[],
        };
        for entry in entries {
            let case = parse_case(entry).map_err(|error| format!("{}: {error}", path.display()))?;
            cases.push(case);
        }
    }
    Ok(cases)
}

fn parse_case(value: &Value) -> Result<Case, String> {
    let table = value.as_table().ok_or("a case must be a table")?;
    let events = match table.get("events") {
        Some(Value::Array(items)) => items
            .iter()
            .map(parse_event)
            .collect::<Result<Vec<_>, _>>()?,
        Some(_) => return Err("events must be an array".to_owned()),
        None => Vec::new(),
    };
    Ok(Case {
        name: required_text(table, "name")?,
        keyboard: required_text(table, "keyboard")?,
        events,
    })
}

fn parse_event(value: &Value) -> Result<Event, String> {
    let table = value.as_table().ok_or("an event must be a table")?;
    let vk = match number_field(table, "vk")? {
        Some(number) => Some(u16::try_from(number).map_err(|_| "vk is out of range")?),
        None => None,
    };
    let delete = match number_field(table, "delete")? {
        Some(number) => Some(usize::try_from(number).map_err(|_| "delete is negative")?),
        None => None,
    };
    Ok(Event {
        vk,
        key: text_field(table, "key")?,
        modifiers: list_field(table, "modifiers")?,
        reset: bool_field(table, "reset")?.unwrap_or(false),
        set_composing: text_field(table, "set_composing")?,
        preview: bool_field(table, "preview")?.unwrap_or(false),
        action: text_field(table, "action")?,
        delete,
        text: text_field(table, "text")?,
        composing: text_field(table, "composing")?,
        processed: bool_field(table, "processed")?,
        engine_composing: text_field(table, "engine_composing")?,
    })
}

fn required_text(table: &Table, key: &str) -> Result<String, String> {
    text_field(table, key)?.ok_or_else(|| format!("missing {key}"))
}

fn text_field(table: &Table, key: &str) -> Result<Option<String>, String> {
    match table.get(key) {
        None => Ok(None),
        Some(Value::String(text)) => Ok(Some(text.clone())),
        Some(_) => Err(format!("{key} must be a string")),
    }
}

fn number_field(table: &Table, key: &str) -> Result<Option<i64>, String> {
    match table.get(key) {
        None => Ok(None),
        Some(Value::Integer(number)) => Ok(Some(*number)),
        Some(_) => Err(format!("{key} must be an integer")),
    }
}

fn bool_field(table: &Table, key: &str) -> Result<Option<bool>, String> {
    match table.get(key) {
        None => Ok(None),
        Some(Value::Boolean(flag)) => Ok(Some(*flag)),
        Some(_) => Err(format!("{key} must be true or false")),
    }
}

fn list_field(table: &Table, key: &str) -> Result<Vec<String>, String> {
    match table.get(key) {
        None => Ok(Vec::new()),
        Some(Value::Array(items)) => items
            .iter()
            .map(|item| match item {
                Value::String(text) => Ok(text.clone()),
                _ => Err(format!("{key} must list strings")),
            })
            .collect(),
        Some(_) => Err(format!("{key} must be an array")),
    }
}

fn corpus_root(cases_dir: &Path) -> Result<PathBuf, String> {
    let absolute = fs::canonicalize(cases_dir)
        .map_err(|error| format!("cannot open {}: {error}", cases_dir.display()))?;
    absolute
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| format!("{} has no parent directory", cases_dir.display()))
}

fn replay(case: &Case, root: &Path) -> Outcome {
    let failures = run_case(case, root).unwrap_or_else(|error| vec![error]);
    Outcome {
        name: case.name.clone(),
        failures,
    }
}

fn run_case(case: &Case, root: &Path) -> Result<Vec<String>, String> {
    let keyboard = load_keyboard(&root.join(&case.keyboard))?;
    let mut engine = Engine::new(Arc::new(keyboard)).map_err(|error| error.to_string())?;
    let mut failures = Vec::new();
    for (index, event) in case.events.iter().enumerate() {
        let observed = run_event(&mut engine, event)?;
        for error in compare(event, &observed) {
            failures.push(format!("event {}: {error}", index + 1));
        }
    }
    Ok(failures)
}

fn run_event(engine: &mut Engine, event: &Event) -> Result<Observed, String> {
    if event.reset {
        engine.reset();
        return Ok(control(engine.composing()));
    }
    if let Some(text) = &event.set_composing {
        engine.set_composing(text);
        return Ok(control(engine.composing()));
    }
    let input = key_event(event)?;
    let output = if event.preview {
        engine.peek(input)
    } else {
        engine.process(input)
    };
    Ok(observe(&output, engine))
}

fn control(composing: &str) -> Observed {
    Observed {
        action: "control".to_owned(),
        delete: 0,
        text: String::new(),
        composing: composing.to_owned(),
        processed: false,
        engine_composing: composing.to_owned(),
    }
}

fn observe(output: &Output, engine: &Engine) -> Observed {
    let (action, delete, text) = action_fields(&output.action);
    Observed {
        action: action.to_owned(),
        delete,
        text,
        composing: output.composing.clone(),
        processed: output.consumed,
        engine_composing: engine.composing().to_owned(),
    }
}

fn action_fields(action: &Action) -> (&'static str, usize, String) {
    match action {
        Action::None => ("none", 0, String::new()),
        Action::Insert(text) => ("insert", 0, text.clone()),
        Action::Delete(count) => ("delete", *count, String::new()),
        Action::Replace { delete, insert } => ("replace", *delete, insert.clone()),
    }
}

fn key_event(event: &Event) -> Result<KeyEvent, String> {
    let ch = match event.key.as_deref() {
        None => None,
        Some(text) => {
            let mut chars = text.chars();
            match (chars.next(), chars.next()) {
                (Some(ch), None) => Some(ch),
                _ => return Err(format!("expected one character in {text:?}")),
            }
        }
    };
    Ok(KeyEvent {
        key: event.vk.and_then(VirtualKey::from_u16),
        ch,
        mods: modifier_flags(&event.modifiers)?,
    })
}

fn modifier_flags(names: &[String]) -> Result<Modifiers, String> {
    let mut mods = Modifiers::default();
    for name in names {
        match name.as_str() {
            "shift" => mods.shift = true,
            "ctrl" => mods.ctrl = true,
            "alt" => mods.alt = true,
            "caps_lock" => mods.caps_lock = true,
            other => return Err(format!("unknown modifier {other}")),
        }
    }
    Ok(mods)
}

fn compare(event: &Event, observed: &Observed) -> Vec<String> {
    [
        field("action", event.action.as_ref(), &observed.action),
        field("delete", event.delete.as_ref(), &observed.delete),
        field("text", event.text.as_ref(), &observed.text),
        field("composing", event.composing.as_ref(), &observed.composing),
        field("processed", event.processed.as_ref(), &observed.processed),
        field(
            "engine_composing",
            event.engine_composing.as_ref(),
            &observed.engine_composing,
        ),
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn field<T: PartialEq + Debug>(name: &str, want: Option<&T>, got: &T) -> Option<String> {
    want.filter(|want| *want != got)
        .map(|want| format!("{name}: expected {want:?}, got {got:?}"))
}

fn outcome_json(outcome: &Outcome) -> Json {
    Json::Object(vec![
        ("name", Json::Str(outcome.name.clone())),
        ("passed", Json::Bool(outcome.failures.is_empty())),
        (
            "failures",
            Json::Array(
                outcome
                    .failures
                    .iter()
                    .map(|failure| Json::Str(failure.clone()))
                    .collect(),
            ),
        ),
    ])
}

fn print_table(outcomes: &[Outcome]) {
    let width = outcomes
        .iter()
        .map(|outcome| outcome.name.chars().count())
        .max()
        .unwrap_or(4)
        .max(4);
    println!("{:<width$}  RESULT", "CASE");
    for outcome in outcomes {
        let verdict = if outcome.failures.is_empty() {
            style::pass("PASS")
        } else {
            style::fail("FAIL")
        };
        println!("{:<width$}  {verdict}", outcome.name);
        for failure in &outcome.failures {
            println!("    {failure}");
        }
    }
}
