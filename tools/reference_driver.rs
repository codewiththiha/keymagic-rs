//! Reference-only driver compiled against the pinned upstream `KeyMagic` 3.

#![forbid(unsafe_code)]

use keymagic_core::engine::{ActionType, ModifierState};
use keymagic_core::km2::Km2Loader;
use keymagic_core::{KeyInput, KeyMagicEngine};
use std::env;
use std::error::Error;
use std::fmt::Write as _;
use std::path::Path;

fn hex(text: &str) -> Result<String, std::fmt::Error> {
    let mut result = String::new();
    for byte in text.as_bytes() {
        write!(result, "{byte:02x}")?;
    }
    Ok(result)
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut args = env::args().skip(1);
    let keyboard = args.next().ok_or("missing keyboard")?;
    let file = kms2km2::compile_kms_file(Path::new(&keyboard))?;
    let mut bytes = Vec::new();
    kms2km2::binary::Km2Writer::new(&mut bytes).write_km2_file(&file)?;
    let keyboard = Km2Loader::load(&bytes)?;
    let mut engine = KeyMagicEngine::new(keyboard)?;
    for key in args {
        let input = if let Some(value) = key.strip_prefix("c:") {
            let mut chars = value.chars();
            let character = chars.next().ok_or("missing character")?;
            if chars.next().is_some() {
                return Err("expected a single character".into());
            }
            KeyInput::from_char(character)
        } else if let Some(value) = key.strip_prefix("v:") {
            KeyInput::from_vk(value.parse()?, ModifierState::default())
        } else if let Some(value) = key.strip_prefix("k:") {
            let (code, text) = value.split_once(':').ok_or("missing key code")?;
            let mut chars = text.chars();
            let character = chars.next().ok_or("missing character")?;
            if chars.next().is_some() {
                return Err("expected a single character".into());
            }
            KeyInput::new(code.parse()?, ModifierState::default(), Some(character))
        } else {
            return Err("unknown input event".into());
        };
        let result = engine.process_key(input)?;
        let (action, count, text) = match result.action {
            ActionType::None => ("none", 0, String::new()),
            ActionType::Insert(text) => ("insert", 0, text),
            ActionType::BackspaceDelete(count) => ("delete", count, String::new()),
            ActionType::BackspaceDeleteAndInsert(count, text) => ("replace", count, text),
        };
        println!(
            "CORPUS\t{action}\t{count}\t{}\t{}\t{}",
            hex(&text)?,
            hex(&result.composing_text)?,
            result.is_processed
        );
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    run()
}
