//! Replays every corpus scenario against the upstream-verified expectations.

mod common;

use keymagic_engine::{Engine, KeyEvent, Modifiers, Output, VirtualKey};
use serde::Deserialize;
use std::error::Error;
use std::fmt::Debug;
use std::fs;
use std::sync::Arc;

/// Number of scenarios in `corpus/cases`; a change means the corpus changed.
const EXPECTED_CASES: usize = 165;

#[derive(Deserialize)]
struct CorpusFile {
    case: Vec<Case>,
}

#[derive(Deserialize)]
struct Case {
    name: String,
    keyboard: String,
    events: Vec<Event>,
}

#[derive(Deserialize)]
struct Event {
    vk: Option<u16>,
    key: Option<String>,
    #[serde(default)]
    modifiers: Vec<String>,
    #[serde(default)]
    reset: bool,
    set_composing: Option<String>,
    #[serde(default)]
    preview: bool,
    action: Option<String>,
    delete: Option<usize>,
    text: Option<String>,
    composing: Option<String>,
    processed: Option<bool>,
    engine_composing: Option<String>,
}

/// Fields recorded for one event, shaped like the reference driver output.
struct Observed {
    action: String,
    delete: usize,
    text: String,
    composing: String,
    processed: bool,
    engine_composing: String,
}

fn load_cases() -> Result<Vec<Case>, Box<dyn Error>> {
    let mut cases = Vec::new();
    for entry in fs::read_dir(common::corpus_dir().join("cases"))? {
        let path = entry?.path();
        if path.extension().is_some_and(|ext| ext == "toml") {
            let file: CorpusFile = toml::from_str(&fs::read_to_string(&path)?)?;
            cases.extend(file.case);
        }
    }
    Ok(cases)
}

fn modifier_flags(names: &[String]) -> Result<Modifiers, Box<dyn Error>> {
    let mut mods = Modifiers::default();
    for name in names {
        match name.as_str() {
            "shift" => mods.shift = true,
            "ctrl" => mods.ctrl = true,
            "alt" => mods.alt = true,
            "caps_lock" => mods.caps_lock = true,
            other => return Err(format!("unknown modifier {other}").into()),
        }
    }
    Ok(mods)
}

fn key_event(event: &Event) -> Result<KeyEvent, Box<dyn Error>> {
    let ch = match event.key.as_deref() {
        Some(text) => {
            let mut chars = text.chars();
            match (chars.next(), chars.next()) {
                (Some(ch), None) => Some(ch),
                _ => return Err(format!("expected one character in {text:?}").into()),
            }
        }
        None => None,
    };
    Ok(KeyEvent {
        key: event.vk.and_then(VirtualKey::from_u16),
        ch,
        mods: modifier_flags(&event.modifiers)?,
    })
}

fn observe_output(output: &Output, engine: &Engine) -> Observed {
    let (action, delete, text) = common::action_fields(&output.action);
    Observed {
        action: action.to_owned(),
        delete,
        text,
        composing: output.composing.clone(),
        processed: output.consumed,
        engine_composing: engine.composing().to_owned(),
    }
}

fn run_event(engine: &mut Engine, event: &Event) -> Result<Observed, Box<dyn Error>> {
    if event.reset {
        engine.reset();
    } else if let Some(text) = &event.set_composing {
        engine.set_composing(text);
    } else {
        let input = key_event(event)?;
        let output = if event.preview {
            engine.peek(input)
        } else {
            engine.process(input)
        };
        return Ok(observe_output(&output, engine));
    }
    let composing = engine.composing();
    Ok(Observed {
        action: "control".to_owned(),
        delete: 0,
        text: String::new(),
        composing: composing.to_owned(),
        processed: false,
        engine_composing: composing.to_owned(),
    })
}

fn expect<T: PartialEq + Debug>(errors: &mut Vec<String>, field: &str, want: Option<&T>, got: &T) {
    if let Some(want) = want.filter(|want| *want != got) {
        errors.push(format!("{field}: expected {want:?}, got {got:?}"));
    }
}

fn compare(event: &Event, observed: &Observed) -> Vec<String> {
    let mut errors = Vec::new();
    expect(
        &mut errors,
        "action",
        event.action.as_ref(),
        &observed.action,
    );
    expect(
        &mut errors,
        "delete",
        event.delete.as_ref(),
        &observed.delete,
    );
    expect(&mut errors, "text", event.text.as_ref(), &observed.text);
    expect(
        &mut errors,
        "composing",
        event.composing.as_ref(),
        &observed.composing,
    );
    expect(
        &mut errors,
        "processed",
        event.processed.as_ref(),
        &observed.processed,
    );
    expect(
        &mut errors,
        "engine_composing",
        event.engine_composing.as_ref(),
        &observed.engine_composing,
    );
    errors
}

fn run_case(case: &Case) -> Result<Vec<String>, Box<dyn Error>> {
    let keyboard = keymagic_km2::parse(&fs::read(common::keyboard_path(&case.keyboard)?)?)?;
    let mut engine = Engine::new(Arc::new(keyboard))?;
    let mut failures = Vec::new();
    for (index, event) in case.events.iter().enumerate() {
        let observed = run_event(&mut engine, event)?;
        for error in compare(event, &observed) {
            failures.push(format!("{} event {}: {error}", case.name, index + 1));
        }
    }
    Ok(failures)
}

#[test]
#[ignore = "needs KMS keyboards compiled by the pinned upstream compiler"]
fn every_corpus_scenario_matches_upstream() -> Result<(), Box<dyn Error>> {
    let cases = load_cases()?;
    assert_eq!(cases.len(), EXPECTED_CASES);
    let mut failures = Vec::new();
    for case in &cases {
        failures.extend(run_case(case)?);
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    Ok(())
}
