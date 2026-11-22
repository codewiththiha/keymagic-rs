//! Safe KM2 version 1.3, 1.4 and 1.5 decoding and encoding.

#![forbid(unsafe_code)]

mod reader;
mod writer;

pub use reader::{parse, parse_document, read_info_only};
pub use writer::{write, write_document};

use keymagic_types::Keyboard;
use std::{error::Error, fmt};

#[doc = include_str!("../docs/document.md")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document {
    pub keyboard: Keyboard,
    pub minor_version: u8,
    pub padding: u8,
}

#[doc = include_str!("../docs/km2_error_kind.md")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Km2ErrorKind {
    Truncated,
    InvalidMagic,
    UnsupportedVersion,
    InvalidUtf16,
    InvalidOpcode,
    InvalidOperand,
    InvalidBoolean,
    TrailingBytes,
    TooLarge,
    InvalidKeyboard,
}

#[doc = include_str!("../docs/km2_error.md")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Km2Error {
    pub kind: Km2ErrorKind,
    pub offset: usize,
}
impl fmt::Display for Km2Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "KM2 {:?} at byte {}", self.kind, self.offset)
    }
}
impl Error for Km2Error {}
