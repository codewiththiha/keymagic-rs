//! Serialize bounded sections with checked on-disk length conversions.

use crate::{Document, Km2Error, Km2ErrorKind};
use keymagic_types::opcodes;
use keymagic_types::{Keyboard, OutputElement, PatternElement};

const fn error(kind: Km2ErrorKind, offset: usize) -> Km2Error {
    Km2Error { kind, offset }
}
fn word(output: &mut Vec<u8>, value: u16) {
    output.extend_from_slice(&value.to_le_bytes());
}
fn count(output: &mut Vec<u8>, value: usize) -> Result<(), Km2Error> {
    word(
        output,
        u16::try_from(value).map_err(|_| error(Km2ErrorKind::TooLarge, output.len()))?,
    );
    Ok(())
}
fn index(output: &mut Vec<u8>, value: usize) -> Result<(), Km2Error> {
    count(
        output,
        value
            .checked_add(1)
            .ok_or_else(|| error(Km2ErrorKind::TooLarge, output.len()))?,
    )
}
fn text(output: &mut Vec<u8>, value: &str) -> Result<(), Km2Error> {
    count(output, value.encode_utf16().count())?;
    for unit in value.encode_utf16() {
        word(output, unit);
    }
    Ok(())
}
fn pattern(output: &mut Vec<u8>, elements: &[PatternElement]) -> Result<(), Km2Error> {
    for element in elements {
        match element {
            PatternElement::Text(value) => {
                word(output, opcodes::STRING);
                text(output, value)?;
            }
            PatternElement::Variable(var)
            | PatternElement::AnyOf(var)
            | PatternElement::NotAnyOf(var) => {
                word(output, opcodes::VARIABLE);
                index(output, *var)?;
                match element {
                    PatternElement::AnyOf(_) => {
                        word(output, opcodes::MODIFIER);
                        word(output, opcodes::ANY_OF);
                    }
                    PatternElement::NotAnyOf(_) => {
                        word(output, opcodes::MODIFIER);
                        word(output, opcodes::NOT_ANY_OF);
                    }
                    _ => {}
                }
            }
            PatternElement::Any => word(output, opcodes::ANY),
            PatternElement::VirtualKeys(keys) => {
                word(output, opcodes::AND);
                for key in keys {
                    word(output, opcodes::PREDEFINED);
                    word(output, *key as u16);
                }
            }
            PatternElement::State(state) => {
                word(output, opcodes::SWITCH);
                word(output, *state);
            }
        }
    }
    Ok(())
}
fn output_elements(output: &mut Vec<u8>, elements: &[OutputElement]) -> Result<(), Km2Error> {
    for element in elements {
        match element {
            OutputElement::Text(value) => {
                word(output, opcodes::STRING);
                text(output, value)?;
            }
            OutputElement::Variable(var) => {
                word(output, opcodes::VARIABLE);
                index(output, *var)?;
            }
            OutputElement::Ref(reference) => {
                word(output, opcodes::REFERENCE);
                index(output, *reference)?;
            }
            OutputElement::VarIndexed { var, by_ref } => {
                word(output, opcodes::VARIABLE);
                index(output, *var)?;
                word(output, opcodes::MODIFIER);
                index(output, *by_ref)?;
            }
            OutputElement::Null => {
                word(output, opcodes::PREDEFINED);
                word(output, 1);
            }
            OutputElement::State(state) => {
                word(output, opcodes::SWITCH);
                word(output, *state);
            }
        }
    }
    Ok(())
}
fn section(output: &mut Vec<u8>, data: &[u8]) -> Result<(), Km2Error> {
    count(output, data.len() / 2)?;
    output.extend_from_slice(data);
    Ok(())
}

/// Write canonical version 1.5; lengths and invalid keyboards return errors.
#[doc = "# Errors\n\nReturns the kind and byte offset of a failure."]
pub fn write(keyboard: &Keyboard) -> Result<Vec<u8>, Km2Error> {
    write_document(&Document {
        keyboard: keyboard.clone(),
        minor_version: 5,
        padding: 0,
    })
}

/// Preserve the document version and alignment byte when representable.
#[doc = "# Errors\n\nReturns the kind and byte offset of a failure."]
pub fn write_document(document: &Document) -> Result<Vec<u8>, Km2Error> {
    let keyboard = &document.keyboard;
    let minor = document.minor_version;
    if !(3..=5).contains(&minor) {
        return Err(error(Km2ErrorKind::UnsupportedVersion, 4));
    }
    if minor == 3 && !keyboard.info.entries.is_empty() || minor < 5 && !keyboard.options.right_alt {
        return Err(error(Km2ErrorKind::InvalidKeyboard, 5));
    }
    keyboard
        .validate()
        .map_err(|_| error(Km2ErrorKind::InvalidKeyboard, 0))?;
    let mut output = b"KMKL".to_vec();
    output.extend_from_slice(&[1, minor]);
    count(&mut output, keyboard.strings.len())?;
    if minor >= 4 {
        count(&mut output, keyboard.info.entries.len())?;
    }
    count(&mut output, keyboard.rules.len())?;
    let options = keyboard.options;
    output.extend_from_slice(&[
        u8::from(options.track_caps),
        u8::from(options.auto_bksp),
        u8::from(options.eat),
        u8::from(options.pos_based),
    ]);
    if minor >= 5 {
        output.extend_from_slice(&[u8::from(options.right_alt), document.padding]);
    }
    for value in &keyboard.strings {
        text(&mut output, value)?;
    }
    for info in &keyboard.info.entries {
        output.extend_from_slice(&info.id);
        count(&mut output, info.data.len())?;
        output.extend_from_slice(&info.data);
    }
    for rule in &keyboard.rules {
        let mut data = Vec::new();
        pattern(&mut data, &rule.lhs)?;
        section(&mut output, &data)?;
        data.clear();
        output_elements(&mut data, &rule.rhs)?;
        section(&mut output, &data)?;
    }
    Ok(output)
}
