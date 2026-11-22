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
    let path = Path::new(&keyboard);
    let bytes = if path.extension().is_some_and(|ext| ext == "km2") {
        std::fs::read(path)?
    } else {
        let file = kms2km2::compile_kms_file(path)?;
        let mut bytes = Vec::new();
        kms2km2::binary::Km2Writer::new(&mut bytes).write_km2_file(&file)?;
        bytes
    };
    let keyboard = Km2Loader::load(&bytes)?;
    let mut engine = KeyMagicEngine::new(keyboard)?;
    for key in args {
        if key == "r:" || key.starts_with("s:") {
            if key == "r:" {
                engine.reset();
            } else if let Some(text) = key.strip_prefix("s:") {
                engine.set_composing_text(text.to_string());
            }
            let composing = hex(engine.composing_text())?;
            println!("CORPUS\tcontrol\t0\t\t{composing}\tfalse\t{composing}");
            continue;
        }
        let (preview, key) = key
            .strip_prefix("t:")
            .map_or((false, key.as_str()), |value| (true, value));
        let input = if let Some(value) = key.strip_prefix("c:") {
            let mut chars = value.chars();
            let character = chars.next().ok_or("missing character")?;
            if chars.next().is_some() {
                return Err("expected a single character".into());
            }
            KeyInput::from_char(character)
        } else if let Some(value) = key.strip_prefix("v:") {
            KeyInput::from_vk(value.parse()?, ModifierState::default())
        } else if let Some(value) = key.strip_prefix("m:") {
            let (mask, rest) = value.split_once(':').ok_or("missing modifiers")?;
            let (code, text) = rest.split_once(':').ok_or("missing key code")?;
            let flags: u8 = mask.parse()?;
            if flags > 15 {
                return Err("invalid modifiers".into());
            }
            let mut chars = text.chars();
            let character = chars.next();
            if chars.next().is_some() {
                return Err("expected at most one character".into());
            }
            let modifiers = ModifierState::new(
                flags & 1 != 0,
                flags & 2 != 0,
                flags & 4 != 0,
                flags & 8 != 0,
            );
            KeyInput::new(code.parse()?, modifiers, character)
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
        let result = if preview {
            engine.process_key_test(input)?
        } else {
            engine.process_key(input)?
        };
        let (action, count, text) = match result.action {
            ActionType::None => ("none", 0, String::new()),
            ActionType::Insert(text) => ("insert", 0, text),
            ActionType::BackspaceDelete(count) => ("delete", count, String::new()),
            ActionType::BackspaceDeleteAndInsert(count, text) => ("replace", count, text),
        };
        println!(
            "CORPUS\t{action}\t{count}\t{}\t{}\t{}\t{}",
            hex(&text)?,
            hex(&result.composing_text)?,
            result.is_processed,
            hex(engine.composing_text())?
        );
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    run()
}
