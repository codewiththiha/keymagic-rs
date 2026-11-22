//! Shared keyboard data with zero-based variable and capture indices.

use crate::VirtualKey;
use std::{error::Error, fmt, str::FromStr};

/// Zero-based index into the keyboard string table.
#[doc = include_str!("../docs/var_id.md")]
pub type VarId = usize;
/// State identifiers are opaque, retaining the on-disk value.
#[doc = include_str!("../docs/state_id.md")]
pub type StateId = u16;

/// Keyboard behavior flags with upstream compiler defaults.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[expect(clippy::struct_excessive_bools, reason = "Independent format flags")]
#[doc = include_str!("../docs/options.md")]
pub struct LayoutOptions {
    pub track_caps: bool,
    pub auto_bksp: bool,
    pub eat: bool,
    pub pos_based: bool,
    pub right_alt: bool,
}

impl Default for LayoutOptions {
    fn default() -> Self {
        Self {
            track_caps: true,
            auto_bksp: false,
            eat: false,
            pos_based: false,
            right_alt: true,
        }
    }
}

/// A record with its on-disk ID and arbitrary bytes.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[doc = include_str!("../docs/info_entry.md")]
pub struct InfoEntry {
    pub id: [u8; 4],
    pub data: Vec<u8>,
}

/// Ordered metadata, including duplicates and unknown entries.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[doc = include_str!("../docs/keyboard_info.md")]
pub struct KeyboardInfo {
    pub entries: Vec<InfoEntry>,
}

impl KeyboardInfo {
    /// Return the first matching record, without interpreting its bytes.
    #[must_use]
    pub fn get(&self, id: &[u8; 4]) -> Option<&[u8]> {
        self.entries
            .iter()
            .find(|entry| &entry.id == id)
            .map(|entry| entry.data.as_slice())
    }
    /// Return the first text record only if its contents are valid UTF-8.
    #[must_use]
    pub fn text(&self, id: &[u8; 4]) -> Option<&str> {
        std::str::from_utf8(self.get(id)?).ok()
    }
    /// Return the keyboard name, if present and valid UTF-8.
    #[must_use]
    pub fn name(&self) -> Option<&str> {
        self.text(b"eman")
    }
    /// Return the description, if present and valid UTF-8.
    #[must_use]
    pub fn description(&self) -> Option<&str> {
        self.text(b"csed")
    }
    /// Return the font family, if present and valid UTF-8.
    #[must_use]
    pub fn font_family(&self) -> Option<&str> {
        self.text(b"tnof")
    }
    /// Return the hotkey text, if present and valid UTF-8.
    #[must_use]
    pub fn hotkey(&self) -> Option<&str> {
        self.text(b"ykth")
    }
    /// Return the raw icon record if present.
    #[must_use]
    pub fn icon(&self) -> Option<&[u8]> {
        self.get(b"noci")
    }
}

/// Origin location in a source file, measured in bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[doc = include_str!("../docs/span.md")]
pub struct SourceSpan {
    pub start: usize,
    pub end: usize,
}

/// Input pattern elements; a virtual-key group retains its original order.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[doc = include_str!("../docs/pattern.md")]
pub enum PatternElement {
    Text(String),
    Variable(VarId),
    AnyOf(VarId),
    NotAnyOf(VarId),
    Any,
    VirtualKeys(Vec<VirtualKey>),
    State(StateId),
}

/// Output elements; reference indices are zero-based capture positions.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[doc = include_str!("../docs/output.md")]
pub enum OutputElement {
    Text(String),
    Variable(VarId),
    Ref(usize),
    VarIndexed { var: VarId, by_ref: usize },
    Null,
    State(StateId),
}

/// A mapping rule with an optional source location.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[doc = include_str!("../docs/rule.md")]
pub struct Rule {
    pub lhs: Vec<PatternElement>,
    pub rhs: Vec<OutputElement>,
    pub source: Option<SourceSpan>,
}

/// Shared keyboard data independent of the binary encoding.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[doc = include_str!("../docs/keyboard.md")]
pub struct Keyboard {
    pub options: LayoutOptions,
    pub strings: Vec<String>,
    pub info: KeyboardInfo,
    pub rules: Vec<Rule>,
}

/// Validation failure with the rule and element positions.
#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = include_str!("../docs/validation_error.md")]
pub struct ValidationError {
    pub rule: usize,
    pub element: usize,
    pub message: &'static str,
}
impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "rule {}, element {}: {}",
            self.rule, self.element, self.message
        )
    }
}
impl Error for ValidationError {}

impl Keyboard {
    /// Check bounds and key groups; errors identify the first bad element.
    #[doc = "# Errors\n\nReturns the first invalid rule element."]
    pub fn validate(&self) -> Result<(), ValidationError> {
        for (rule_index, rule) in self.rules.iter().enumerate() {
            let mut captures = 0;
            for (element, item) in rule.lhs.iter().enumerate() {
                let message = match item {
                    PatternElement::Variable(v)
                    | PatternElement::AnyOf(v)
                    | PatternElement::NotAnyOf(v)
                        if *v >= self.strings.len() =>
                    {
                        Some("variable index out of bounds")
                    }
                    PatternElement::VirtualKeys(keys)
                        if keys.is_empty() || keys.contains(&VirtualKey::Null) =>
                    {
                        Some("invalid virtual-key group")
                    }
                    _ => None,
                };
                if let Some(message) = message {
                    return Err(ValidationError {
                        rule: rule_index,
                        element,
                        message,
                    });
                }
                if matches!(
                    item,
                    PatternElement::Text(_)
                        | PatternElement::Variable(_)
                        | PatternElement::AnyOf(_)
                        | PatternElement::NotAnyOf(_)
                        | PatternElement::Any
                ) {
                    captures += 1;
                }
            }
            for (element, item) in rule.rhs.iter().enumerate() {
                let message = match item {
                    OutputElement::Variable(v) | OutputElement::VarIndexed { var: v, .. }
                        if *v >= self.strings.len() =>
                    {
                        Some("variable index out of bounds")
                    }
                    OutputElement::Ref(v) | OutputElement::VarIndexed { by_ref: v, .. }
                        if *v >= captures =>
                    {
                        Some("capture index out of bounds")
                    }
                    _ => None,
                };
                if let Some(message) = message {
                    return Err(ValidationError {
                        rule: rule_index,
                        element,
                        message,
                    });
                }
            }
        }
        Ok(())
    }
}

/// Modifier bits independent of platform APIs.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[doc = include_str!("../docs/modifiers.md")]
pub struct Modifiers(u8);
impl Modifiers {
    pub const SHIFT: Self = Self(1);
    pub const CTRL: Self = Self(2);
    pub const ALT: Self = Self(4);
    pub const CAPS: Self = Self(8);
    pub const META: Self = Self(16);
    /// Matcher order is shift, control, alt, caps, then meta.
    pub const ORDER: [Self; 5] = [Self::SHIFT, Self::CTRL, Self::ALT, Self::CAPS, Self::META];
    /// Construct flags only when no undefined bits are present.
    #[must_use]
    pub const fn from_bits(bits: u8) -> Option<Self> {
        if bits & !31 == 0 {
            Some(Self(bits))
        } else {
            None
        }
    }
    /// Return the underlying flag bits.
    #[must_use]
    pub const fn bits(self) -> u8 {
        self.0
    }
    /// Test whether all requested modifier bits are present.
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }
}
impl std::ops::BitOr for Modifiers {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl std::ops::BitAnd for Modifiers {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self {
        Self(self.0 & rhs.0)
    }
}
impl std::ops::BitOrAssign for Modifiers {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

/// A parsed global hotkey and its modifier requirements.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[expect(clippy::struct_excessive_bools, reason = "Independent flags")]
#[doc = include_str!("../docs/hotkey.md")]
pub struct Hotkey {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub meta: bool,
    pub key: VirtualKey,
}

/// A hotkey has no key, more than one key, or an unknown key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc = include_str!("../docs/hotkey_error.md")]
pub struct HotkeyError;
impl fmt::Display for HotkeyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("invalid hotkey")
    }
}
impl Error for HotkeyError {}
impl FromStr for Hotkey {
    type Err = HotkeyError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let (mut ctrl, mut alt, mut shift, mut meta, mut key) = (false, false, false, false, None);
        for part in value
            .split(['+', ' '])
            .filter(|part| !part.is_empty())
            .map(str::to_ascii_uppercase)
        {
            match part.as_str() {
                "CTRL" | "CONTROL" => ctrl = true,
                "ALT" | "OPTION" => alt = true,
                "SHIFT" => shift = true,
                "META" | "CMD" | "COMMAND" | "WIN" | "SUPER" => meta = true,
                part => {
                    if key.is_some() {
                        return Err(HotkeyError);
                    }
                    let name = match part {
                        "ENTER" | "RETURN" => "VK_RETURN".to_owned(),
                        "BACKSPACE" | "BACK" | "DELETE" => "VK_BACK".to_owned(),
                        "ESC" | "ESCAPE" => "VK_ESCAPE".to_owned(),
                        "CAPSLOCK" | "CAPS" => "VK_CAPITAL".to_owned(),
                        "PLUS" | "=" => "VK_OEM_PLUS".to_owned(),
                        "MINUS" | "-" => "VK_OEM_MINUS".to_owned(),
                        "COMMA" | "," => "VK_OEM_COMMA".to_owned(),
                        "PERIOD" | "." => "VK_OEM_PERIOD".to_owned(),
                        "SEMICOLON" | ";" => "VK_OEM_1".to_owned(),
                        "SLASH" | "/" => "VK_OEM_2".to_owned(),
                        "GRAVE" | "`" => "VK_OEM_3".to_owned(),
                        "LEFTBRACKET" | "[" => "VK_OEM_4".to_owned(),
                        "BACKSLASH" | "\\" => "VK_OEM_5".to_owned(),
                        "RIGHTBRACKET" | "]" => "VK_OEM_6".to_owned(),
                        "QUOTE" | "'" => "VK_OEM_7".to_owned(),
                        _ if part.len() == 1 => format!("VK_KEY_{part}"),
                        _ if part.starts_with("VK_") => part.to_owned(),
                        _ => format!("VK_{part}"),
                    };
                    key = VirtualKey::from_kms_name(&name);
                    if key.is_none() {
                        return Err(HotkeyError);
                    }
                }
            }
        }
        Ok(Self {
            ctrl,
            alt,
            shift,
            meta,
            key: key.ok_or(HotkeyError)?,
        })
    }
}
