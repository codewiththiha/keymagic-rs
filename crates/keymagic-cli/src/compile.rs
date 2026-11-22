//! Compile, check, format and dump commands.

use crate::json::Json;
use crate::resolver::DirResolver;
use keymagic_kms::{Diagnostic, Severity};
use std::fs;
use std::path::Path;
use std::process::ExitCode;

/// Compiles `input` to KM2 and reports any warnings.
pub fn compile(input: &Path, output: Option<&Path>, json: bool) -> Result<ExitCode, String> {
    let src = read_source(input)?;
    let name = display(input);
    let resolver = DirResolver::beside(input);
    let keyboard = match keymagic_kms::compile(&src, &resolver) {
        Ok(keyboard) => keyboard,
        Err(errors) => {
            emit_errors(&errors, &src, &name, json);
            return Ok(ExitCode::FAILURE);
        }
    };
    let warnings = keymagic_kms::check(&src, &resolver);
    let bytes =
        keymagic_km2::write(&keyboard).map_err(|error| format!("cannot encode KM2: {error}"))?;
    let target = output.map_or_else(|| input.with_extension("km2"), Path::to_path_buf);
    write_file(&target, &bytes)?;
    let target_name = display(&target);
    if json {
        let body = Json::Object(vec![
            ("output", Json::Str(target_name)),
            ("bytes", Json::Number(bytes.len())),
            ("warnings", diagnostics_array(&warnings, &name)),
        ]);
        println!("{}", body.render());
    } else {
        print_text(&warnings, &src, &name);
        println!("wrote {target_name} ({} bytes)", bytes.len());
    }
    Ok(ExitCode::SUCCESS)
}

/// Prints every diagnostic for `input`; fails when one is an error.
pub fn check(input: &Path, json: bool) -> Result<ExitCode, String> {
    let src = read_source(input)?;
    let name = display(input);
    let diagnostics = keymagic_kms::check(&src, &DirResolver::beside(input));
    let failed = diagnostics
        .iter()
        .any(|diagnostic| matches!(diagnostic.severity, Severity::Error));
    if json {
        println!("{}", diagnostics_array(&diagnostics, &name).render());
    } else {
        print_text(&diagnostics, &src, &name);
        if !failed {
            println!("ok: {name}");
        }
    }
    Ok(if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    })
}

/// Prints, rewrites, or checks the canonical form of a file.
pub fn fmt(input: &Path, write: bool, check: bool, json: bool) -> Result<ExitCode, String> {
    let src = read_source(input)?;
    let name = display(input);
    let formatted = match keymagic_kms::format(&src) {
        Ok(text) => text,
        Err(errors) => {
            emit_errors(&errors, &src, &name, json);
            return Ok(ExitCode::FAILURE);
        }
    };
    let changed = formatted != src;
    if write && changed {
        write_file(input, formatted.as_bytes())?;
    }
    if json {
        let shown = if write {
            Json::Null
        } else {
            Json::Str(formatted)
        };
        let body = Json::Object(vec![
            ("path", Json::Str(name)),
            ("changed", Json::Bool(changed)),
            ("formatted", shown),
        ]);
        println!("{}", body.render());
    } else if write {
        if changed {
            println!("formatted {name}");
        }
    } else if check {
        if changed {
            eprintln!("not formatted: {name}");
        } else {
            println!("ok: {name}");
        }
    } else {
        print!("{formatted}");
    }
    Ok(if check && changed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    })
}

/// Summarizes a KM2 file: its info, rule and string counts, and options.
pub fn dump(input: &Path, json: bool) -> Result<ExitCode, String> {
    let bytes =
        fs::read(input).map_err(|error| format!("cannot read {}: {error}", input.display()))?;
    let keyboard =
        keymagic_km2::parse(&bytes).map_err(|error| format!("{}: {error}", input.display()))?;
    let info = &keyboard.info;
    let options = &keyboard.options;
    if json {
        let body = Json::Object(vec![
            ("name", Json::opt(info.name())),
            ("description", Json::opt(info.description())),
            ("font_family", Json::opt(info.font_family())),
            ("hotkey", Json::opt(info.hotkey())),
            ("rules", Json::Number(keyboard.rules.len())),
            ("strings", Json::Number(keyboard.strings.len())),
            (
                "options",
                Json::Object(vec![
                    ("track_caps", Json::Bool(options.track_caps)),
                    ("auto_bksp", Json::Bool(options.auto_bksp)),
                    ("eat", Json::Bool(options.eat)),
                    ("pos_based", Json::Bool(options.pos_based)),
                    ("right_alt", Json::Bool(options.right_alt)),
                ]),
            ),
        ]);
        println!("{}", body.render());
    } else {
        println!("name: {}", or_dash(info.name()));
        println!("description: {}", or_dash(info.description()));
        println!("font: {}", or_dash(info.font_family()));
        println!("hotkey: {}", or_dash(info.hotkey()));
        println!("rules: {}", keyboard.rules.len());
        println!("strings: {}", keyboard.strings.len());
        println!(
            "options: track_caps={} auto_bksp={} eat={} pos_based={} right_alt={}",
            options.track_caps,
            options.auto_bksp,
            options.eat,
            options.pos_based,
            options.right_alt
        );
    }
    Ok(ExitCode::SUCCESS)
}

fn or_dash(value: Option<&str>) -> &str {
    value.unwrap_or("-")
}

fn read_source(path: &Path) -> Result<String, String> {
    fs::read_to_string(path).map_err(|error| format!("cannot read {}: {error}", path.display()))
}

fn write_file(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent().filter(|dir| !dir.as_os_str().is_empty()) {
        fs::create_dir_all(parent)
            .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
    }
    fs::write(path, bytes).map_err(|error| format!("cannot write {}: {error}", path.display()))
}

fn display(path: &Path) -> String {
    path.display().to_string()
}

fn diagnostic_json(diagnostic: &Diagnostic, name: &str) -> Json {
    let severity = match diagnostic.severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
    };
    let file = if diagnostic.file.is_empty() {
        name
    } else {
        diagnostic.file.as_str()
    };
    Json::Object(vec![
        ("severity", Json::Str(severity.to_owned())),
        ("file", Json::Str(file.to_owned())),
        ("start", Json::Number(diagnostic.span.start)),
        ("end", Json::Number(diagnostic.span.end)),
        ("message", Json::Str(diagnostic.message.clone())),
        ("help", Json::opt(diagnostic.help.as_deref())),
    ])
}

fn diagnostics_array(diagnostics: &[Diagnostic], name: &str) -> Json {
    Json::Array(
        diagnostics
            .iter()
            .map(|diagnostic| diagnostic_json(diagnostic, name))
            .collect(),
    )
}

fn emit_errors(errors: &[Diagnostic], src: &str, name: &str, json: bool) {
    if json {
        println!("{}", diagnostics_array(errors, name).render());
    } else {
        print_text(errors, src, name);
    }
}

fn print_text(diagnostics: &[Diagnostic], src: &str, name: &str) {
    for diagnostic in diagnostics {
        eprint!("{}", diagnostic.render(src, name));
    }
}
