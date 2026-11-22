//! Types each stdin line into a keyboard, one character at a time.

use crate::json::Json;
use crate::replay::load_keyboard;
use keymagic_engine::{Action, Engine, KeyEvent, Modifiers};
use std::io::{self, BufRead};
use std::path::Path;
use std::process::ExitCode;
use std::sync::Arc;

/// Types each stdin line; the line `:reset` clears the composing text.
pub fn run(keyboard: &Path, json: bool) -> Result<ExitCode, String> {
    let keyboard = load_keyboard(keyboard)?;
    let mut engine = Engine::new(Arc::new(keyboard)).map_err(|error| error.to_string())?;
    let mut host = String::new();
    for line in io::stdin().lock().lines() {
        let line = line.map_err(|error| format!("cannot read stdin: {error}"))?;
        if line == ":reset" {
            engine.reset();
        } else {
            for ch in line.chars() {
                let input = KeyEvent {
                    key: None,
                    ch: Some(ch),
                    mods: Modifiers::default(),
                };
                let output = engine.process(input);
                if output.consumed {
                    apply(&mut host, &output.action);
                } else {
                    host.push(ch);
                }
            }
        }
        if json {
            let body = Json::Object(vec![
                ("input", Json::Str(line.clone())),
                ("text", Json::Str(host.clone())),
                ("composing", Json::Str(engine.composing().to_owned())),
            ]);
            println!("{}", body.render());
        } else {
            println!("text: {host}");
            println!("composing: {}", engine.composing());
        }
    }
    Ok(ExitCode::SUCCESS)
}

fn apply(host: &mut String, action: &Action) {
    match action {
        Action::None => {}
        Action::Insert(text) => host.push_str(text),
        Action::Delete(count) => remove_last(host, *count),
        Action::Replace { delete, insert } => {
            remove_last(host, *delete);
            host.push_str(insert);
        }
    }
}

fn remove_last(host: &mut String, count: usize) {
    for _ in 0..count {
        host.pop();
    }
}
