//! Key events as a platform adapter reports them.

use keymagic_types::VirtualKey;

/// Modifier flags attached to a key event.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[expect(clippy::struct_excessive_bools, reason = "independent keyboard modifier flags")]
pub struct Modifiers {
    /// Shift is held.
    pub shift: bool,
    /// Control is held.
    pub ctrl: bool,
    /// Alt is held.
    pub alt: bool,
    /// Caps Lock is on; it never changes matching.
    pub caps_lock: bool,
}

/// One key press: its virtual key, delivered character and modifiers.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct KeyEvent {
    /// Physical key; `None` for unknown codes and character-only input.
    pub key: Option<VirtualKey>,
    /// Character the key delivered, if any.
    pub ch: Option<char>,
    /// Modifier state when the key was pressed.
    pub mods: Modifiers,
}
