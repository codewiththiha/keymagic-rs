//! Validate shared rules, metadata, and hotkey parsing.

use keymagic_types::{Hotkey, Keyboard, OutputElement, PatternElement, Rule, VirtualKey};
use std::error::Error;

fn keyboard(lhs: Vec<PatternElement>, rhs: Vec<OutputElement>) -> Keyboard {
    Keyboard {
        strings: vec!["abc".into()],
        rules: vec![Rule {
            lhs,
            rhs,
            source: None,
        }],
        ..Keyboard::default()
    }
}

#[test]
fn invalid_variables_and_capture_references_are_rejected() {
    let invalid = [
        keyboard(vec![PatternElement::Variable(1)], vec![]),
        keyboard(vec![PatternElement::AnyOf(1)], vec![]),
        keyboard(vec![PatternElement::NotAnyOf(1)], vec![]),
        keyboard(vec![], vec![OutputElement::Variable(1)]),
        keyboard(vec![PatternElement::Any], vec![OutputElement::Ref(1)]),
        keyboard(
            vec![PatternElement::Any],
            vec![OutputElement::VarIndexed { var: 0, by_ref: 1 }],
        ),
        keyboard(
            vec![PatternElement::Any],
            vec![OutputElement::VarIndexed { var: 1, by_ref: 0 }],
        ),
        keyboard(vec![PatternElement::VirtualKeys(vec![])], vec![]),
        keyboard(
            vec![PatternElement::VirtualKeys(vec![VirtualKey::Null])],
            vec![],
        ),
    ];
    for item in invalid {
        assert!(item.validate().is_err());
    }
    assert!(
        keyboard(
            vec![PatternElement::Any],
            vec![OutputElement::Ref(0), OutputElement::Null]
        )
        .validate()
        .is_ok()
    );
}

#[test]
fn hotkeys_follow_upstream_aliases() -> Result<(), Box<dyn Error>> {
    let parsed: Hotkey = "Ctrl+Shift+M".parse()?;
    assert_eq!(parsed.key, VirtualKey::KeyM);
    assert!(parsed.ctrl && parsed.shift && !parsed.alt && !parsed.meta);
    assert_eq!("cmd option space".parse::<Hotkey>()?.key, VirtualKey::Space);
    assert!("ctrl+shift".parse::<Hotkey>().is_err());
    assert!("a+b".parse::<Hotkey>().is_err());
    Ok(())
}

#[cfg(feature = "serde")]
#[test]
fn json_round_trip() -> Result<(), Box<dyn Error>> {
    let kb = keyboard(vec![PatternElement::Any], vec![OutputElement::Ref(0)]);
    let json = serde_json::to_string(&kb)?;
    assert_eq!(serde_json::from_str::<Keyboard>(&json)?, kb);
    Ok(())
}

#[test]
fn corpus_hotkeys_parse() -> Result<(), Box<dyn Error>> {
    let root = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../corpus/kms"));
    let mut pending = vec![root.to_path_buf()];
    let mut count = 0;
    while let Some(path) = pending.pop() {
        for entry in std::fs::read_dir(path)? {
            let path = entry?.path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|ext| ext == "kms") {
                let text = std::fs::read_to_string(&path)?;
                for line in text.lines().map(str::trim) {
                    if line.to_ascii_uppercase().starts_with("@HOTKEY") {
                        let (_, value) = line.split_once('=').ok_or("hotkey assignment")?;
                        let value = value.trim().trim_matches('"');
                        let _: Hotkey = value.parse()?;
                        count += 1;
                    }
                }
            }
        }
    }
    assert!(count > 0);
    Ok(())
}

#[test]
fn metadata_preserves_bytes_and_exposes_valid_text() {
    use keymagic_types::{InfoEntry, KeyboardInfo, Modifiers};
    let info = KeyboardInfo {
        entries: vec![
            InfoEntry {
                id: *b"eman",
                data: b"Example".to_vec(),
            },
            InfoEntry {
                id: *b"csed",
                data: vec![0xFF],
            },
            InfoEntry {
                id: *b"noci",
                data: vec![1, 2, 3],
            },
        ],
    };
    assert_eq!(info.name(), Some("Example"));
    assert_eq!(info.description(), None);
    assert_eq!(info.icon(), Some(&[1, 2, 3][..]));
    assert_eq!(Modifiers::from_bits(0x20), None);
    assert_eq!((Modifiers::CTRL | Modifiers::SHIFT).bits(), 3);
}

#[test]
fn opcode_values_match_the_wire_format() {
    use keymagic_types::opcodes;
    assert_eq!(opcodes::STRING, 0xF0);
    assert_eq!(opcodes::VARIABLE, 0xF1);
    assert_eq!(opcodes::REFERENCE, 0xF2);
    assert_eq!(opcodes::PREDEFINED, 0xF3);
    assert_eq!(opcodes::MODIFIER, 0xF4);
    assert_eq!(opcodes::ANY_OF, 0xF5);
    assert_eq!(opcodes::AND, 0xF6);
    assert_eq!(opcodes::NOT_ANY_OF, 0xF7);
    assert_eq!(opcodes::ANY, 0xF8);
    assert_eq!(opcodes::SWITCH, 0xF9);
}
