//! Read bounded little-endian sections and decode contextual rule operands.

use crate::{Document, Km2Error, Km2ErrorKind};
use keymagic_types::opcodes;
use keymagic_types::{
    InfoEntry, Keyboard, KeyboardInfo, LayoutOptions, OutputElement, PatternElement, Rule,
    VirtualKey,
};

struct Cursor<'a> {
    bytes: &'a [u8],
    pos: usize,
    end: usize,
}
impl<'a> Cursor<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            pos: 0,
            end: bytes.len(),
        }
    }
    const fn error(&self, kind: Km2ErrorKind) -> Km2Error {
        Km2Error {
            kind,
            offset: self.pos,
        }
    }
    fn take(&mut self, count: usize) -> Result<&'a [u8], Km2Error> {
        let end = self
            .pos
            .checked_add(count)
            .ok_or_else(|| self.error(Km2ErrorKind::Truncated))?;
        if end > self.end {
            return Err(self.error(Km2ErrorKind::Truncated));
        }
        let result = &self.bytes[self.pos..end];
        self.pos = end;
        Ok(result)
    }
    fn byte(&mut self) -> Result<u8, Km2Error> {
        Ok(self.take(1)?[0])
    }
    fn word(&mut self) -> Result<u16, Km2Error> {
        let b = self.take(2)?;
        Ok(u16::from_le_bytes([b[0], b[1]]))
    }
    fn boolean(&mut self) -> Result<bool, Km2Error> {
        match self.byte()? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(Km2Error {
                kind: Km2ErrorKind::InvalidBoolean,
                offset: self.pos - 1,
            }),
        }
    }
    fn text(&mut self) -> Result<String, Km2Error> {
        let len = usize::from(self.word()?);
        let start = self.pos;
        let data = self.take(len * 2)?;
        let words: Vec<u16> = data
            .chunks_exact(2)
            .map(|v| u16::from_le_bytes([v[0], v[1]]))
            .collect();
        String::from_utf16(&words).map_err(|_| Km2Error {
            kind: Km2ErrorKind::InvalidUtf16,
            offset: start,
        })
    }
    fn index(&mut self) -> Result<usize, Km2Error> {
        let at = self.pos;
        self.word()?
            .checked_sub(1)
            .map(usize::from)
            .ok_or(Km2Error {
                kind: Km2ErrorKind::InvalidOperand,
                offset: at,
            })
    }
    fn section(&mut self) -> Result<Self, Km2Error> {
        let words = usize::from(self.word()?);
        let start = self.pos;
        self.take(words * 2)?;
        Ok(Self {
            bytes: self.bytes,
            pos: start,
            end: self.pos,
        })
    }
}

struct Header {
    minor: u8,
    strings: u16,
    info: u16,
    rules: u16,
    options: LayoutOptions,
    padding: u8,
}
fn header(cursor: &mut Cursor<'_>) -> Result<Header, Km2Error> {
    if cursor.take(4)? != b"KMKL" {
        return Err(Km2Error {
            kind: Km2ErrorKind::InvalidMagic,
            offset: 0,
        });
    }
    let major = cursor.byte()?;
    let minor = cursor.byte()?;
    if major != 1 || !(3..=5).contains(&minor) {
        return Err(Km2Error {
            kind: Km2ErrorKind::UnsupportedVersion,
            offset: 4,
        });
    }
    let strings = cursor.word()?;
    let info = if minor >= 4 { cursor.word()? } else { 0 };
    let rules = cursor.word()?;
    let track_caps = cursor.boolean()?;
    let auto_bksp = cursor.boolean()?;
    let eat = cursor.boolean()?;
    let pos_based = cursor.boolean()?;
    let right_alt = if minor >= 5 { cursor.boolean()? } else { true };
    let padding = if minor >= 5 { cursor.byte()? } else { 0 };
    Ok(Header {
        minor,
        strings,
        info,
        rules,
        options: LayoutOptions {
            track_caps,
            auto_bksp,
            eat,
            pos_based,
            right_alt,
        },
        padding,
    })
}
fn strings(cursor: &mut Cursor<'_>, count: u16) -> Result<Vec<String>, Km2Error> {
    let mut result = Vec::new();
    for _ in 0..count {
        result.push(cursor.text()?);
    }
    Ok(result)
}
fn info(cursor: &mut Cursor<'_>, count: u16) -> Result<KeyboardInfo, Km2Error> {
    let mut entries = Vec::new();
    for _ in 0..count {
        let id: [u8; 4] = cursor
            .take(4)?
            .try_into()
            .map_err(|_| cursor.error(Km2ErrorKind::Truncated))?;
        let len = usize::from(cursor.word()?);
        entries.push(InfoEntry {
            id,
            data: cursor.take(len)?.to_vec(),
        });
    }
    Ok(KeyboardInfo { entries })
}
fn operand_error(cursor: &Cursor<'_>, offset: usize) -> Km2Error {
    Km2Error {
        kind: Km2ErrorKind::InvalidOperand,
        offset: offset.min(cursor.end),
    }
}
fn pattern(cursor: &mut Cursor<'_>) -> Result<Vec<PatternElement>, Km2Error> {
    let mut result = Vec::new();
    while cursor.pos < cursor.end {
        let at = cursor.pos;
        let op = cursor.word()?;
        let value = match op {
            opcodes::STRING => PatternElement::Text(cursor.text()?),
            opcodes::VARIABLE => {
                let index = cursor.index()?;
                if cursor.end - cursor.pos >= 2
                    && u16::from_le_bytes([cursor.bytes[cursor.pos], cursor.bytes[cursor.pos + 1]])
                        == opcodes::MODIFIER
                {
                    cursor.word()?;
                    match cursor.word()? {
                        opcodes::ANY_OF => PatternElement::AnyOf(index),
                        opcodes::NOT_ANY_OF => PatternElement::NotAnyOf(index),
                        _ => return Err(operand_error(cursor, cursor.pos - 2)),
                    }
                } else {
                    PatternElement::Variable(index)
                }
            }
            opcodes::AND => {
                let mut keys = Vec::new();
                while cursor.end - cursor.pos >= 2
                    && u16::from_le_bytes([cursor.bytes[cursor.pos], cursor.bytes[cursor.pos + 1]])
                        == opcodes::PREDEFINED
                {
                    cursor.word()?;
                    let key_at = cursor.pos;
                    let key = VirtualKey::from_u16(cursor.word()?)
                        .filter(|key| *key != VirtualKey::Null)
                        .ok_or_else(|| operand_error(cursor, key_at))?;
                    keys.push(key);
                }
                if keys.is_empty() {
                    return Err(operand_error(cursor, at));
                }
                PatternElement::VirtualKeys(keys)
            }
            opcodes::ANY => PatternElement::Any,
            opcodes::SWITCH => PatternElement::State(cursor.word()?),
            _ => {
                return Err(Km2Error {
                    kind: Km2ErrorKind::InvalidOpcode,
                    offset: at,
                });
            }
        };
        result.push(value);
    }
    Ok(result)
}
fn output(cursor: &mut Cursor<'_>) -> Result<Vec<OutputElement>, Km2Error> {
    let mut result = Vec::new();
    while cursor.pos < cursor.end {
        let at = cursor.pos;
        let op = cursor.word()?;
        let value = match op {
            opcodes::STRING => OutputElement::Text(cursor.text()?),
            opcodes::VARIABLE => {
                let var = cursor.index()?;
                if cursor.end - cursor.pos >= 2
                    && u16::from_le_bytes([cursor.bytes[cursor.pos], cursor.bytes[cursor.pos + 1]])
                        == opcodes::MODIFIER
                {
                    cursor.word()?;
                    OutputElement::VarIndexed {
                        var,
                        by_ref: cursor.index()?,
                    }
                } else {
                    OutputElement::Variable(var)
                }
            }
            opcodes::REFERENCE => OutputElement::Ref(cursor.index()?),
            opcodes::PREDEFINED => {
                if cursor.word()? != 1 {
                    return Err(operand_error(cursor, cursor.pos - 2));
                }
                OutputElement::Null
            }
            opcodes::SWITCH => OutputElement::State(cursor.word()?),
            _ => {
                return Err(Km2Error {
                    kind: Km2ErrorKind::InvalidOpcode,
                    offset: at,
                });
            }
        };
        result.push(value);
    }
    Ok(result)
}

/// Read a keyboard; malformed input returns its byte offset.
#[doc = "# Errors\n\nReturns the kind and byte offset of a failure."]
pub fn parse(bytes: &[u8]) -> Result<Keyboard, Km2Error> {
    Ok(parse_document(bytes)?.keyboard)
}

/// Read a keyboard and its original version and alignment byte.
#[doc = "# Errors\n\nReturns the kind and byte offset of a failure."]
pub fn parse_document(bytes: &[u8]) -> Result<Document, Km2Error> {
    let mut cursor = Cursor::new(bytes);
    let header = header(&mut cursor)?;
    let strings = strings(&mut cursor, header.strings)?;
    let info = info(&mut cursor, header.info)?;
    let mut rules = Vec::new();
    for _ in 0..header.rules {
        let lhs = pattern(&mut cursor.section()?)?;
        let rhs = output(&mut cursor.section()?)?;
        rules.push(Rule {
            lhs,
            rhs,
            source: None,
        });
    }
    if cursor.pos != cursor.end {
        return Err(cursor.error(Km2ErrorKind::TrailingBytes));
    }
    Ok(Document {
        keyboard: Keyboard {
            options: header.options,
            strings,
            info,
            rules,
        },
        minor_version: header.minor,
        padding: header.padding,
    })
}

/// Read only through metadata, without parsing rules or requiring their bytes.
#[doc = "# Errors\n\nReturns the kind and byte offset of a failure."]
pub fn read_info_only(bytes: &[u8]) -> Result<KeyboardInfo, Km2Error> {
    let mut cursor = Cursor::new(bytes);
    let header = header(&mut cursor)?;
    for _ in 0..header.strings {
        let length = usize::from(cursor.word()?);
        cursor.take(length * 2)?;
    }
    info(&mut cursor, header.info)
}
