//! Maps `IBus` key events onto `KeyMagic` key events.

use keymagic_engine::{KeyEvent, Modifiers};
use keymagic_types::VirtualKey;

/// Set on key release events.
const RELEASE: u32 = 1 << 30;
const SHIFT: u32 = 1 << 0;
const LOCK: u32 = 1 << 1;
const CONTROL: u32 = 1 << 2;
const ALT: u32 = 1 << 3;

/// Offset between X keycodes and evdev scancodes.
const X_OFFSET: u32 = 8;

/// Evdev scancode to `KeyMagic` key, for a US layout.
const SCANCODES: &[(u32, VirtualKey)] = &[
    (1, VirtualKey::Escape),
    (2, VirtualKey::Key1),
    (3, VirtualKey::Key2),
    (4, VirtualKey::Key3),
    (5, VirtualKey::Key4),
    (6, VirtualKey::Key5),
    (7, VirtualKey::Key6),
    (8, VirtualKey::Key7),
    (9, VirtualKey::Key8),
    (10, VirtualKey::Key9),
    (11, VirtualKey::Key0),
    (12, VirtualKey::OemMinus),
    (13, VirtualKey::OemPlus),
    (14, VirtualKey::Back),
    (15, VirtualKey::Tab),
    (16, VirtualKey::KeyQ),
    (17, VirtualKey::KeyW),
    (18, VirtualKey::KeyE),
    (19, VirtualKey::KeyR),
    (20, VirtualKey::KeyT),
    (21, VirtualKey::KeyY),
    (22, VirtualKey::KeyU),
    (23, VirtualKey::KeyI),
    (24, VirtualKey::KeyO),
    (25, VirtualKey::KeyP),
    (26, VirtualKey::Oem4),
    (27, VirtualKey::Oem6),
    (28, VirtualKey::Return),
    (29, VirtualKey::Control),
    (30, VirtualKey::KeyA),
    (31, VirtualKey::KeyS),
    (32, VirtualKey::KeyD),
    (33, VirtualKey::KeyF),
    (34, VirtualKey::KeyG),
    (35, VirtualKey::KeyH),
    (36, VirtualKey::KeyJ),
    (37, VirtualKey::KeyK),
    (38, VirtualKey::KeyL),
    (39, VirtualKey::Oem1),
    (40, VirtualKey::Oem7),
    (41, VirtualKey::Oem3),
    (42, VirtualKey::Shift),
    (43, VirtualKey::Oem5),
    (44, VirtualKey::KeyZ),
    (45, VirtualKey::KeyX),
    (46, VirtualKey::KeyC),
    (47, VirtualKey::KeyV),
    (48, VirtualKey::KeyB),
    (49, VirtualKey::KeyN),
    (50, VirtualKey::KeyM),
    (51, VirtualKey::OemComma),
    (52, VirtualKey::OemPeriod),
    (53, VirtualKey::Oem2),
    (54, VirtualKey::Shift),
    (56, VirtualKey::Menu),
    (57, VirtualKey::Space),
    (58, VirtualKey::Capital),
    (59, VirtualKey::F1),
    (60, VirtualKey::F2),
    (61, VirtualKey::F3),
    (62, VirtualKey::F4),
    (63, VirtualKey::F5),
    (64, VirtualKey::F6),
    (65, VirtualKey::F7),
    (66, VirtualKey::F8),
    (67, VirtualKey::F9),
    (68, VirtualKey::F10),
    (87, VirtualKey::F11),
    (88, VirtualKey::F12),
    (97, VirtualKey::Control),
    (100, VirtualKey::Menu),
    (104, VirtualKey::Prior),
    (109, VirtualKey::Next),
    (111, VirtualKey::Delete),
];

/// Builds the engine event for one `IBus` key press.
#[must_use]
pub fn key_event(keyval: u32, keycode: u32, state: u32) -> KeyEvent {
    KeyEvent {
        key: virtual_key(keycode),
        ch: char_for_keysym(keyval),
        mods: modifiers(state),
    }
}

/// True for key release events, which the engine ignores.
#[must_use]
pub const fn is_release(state: u32) -> bool {
    state & RELEASE != 0
}

const fn modifiers(state: u32) -> Modifiers {
    Modifiers {
        shift: state & SHIFT != 0,
        ctrl: state & CONTROL != 0,
        alt: state & ALT != 0,
        caps_lock: state & LOCK != 0,
    }
}

fn virtual_key(keycode: u32) -> Option<VirtualKey> {
    let scancode = keycode.checked_sub(X_OFFSET)?;
    SCANCODES
        .iter()
        .find(|(code, _)| *code == scancode)
        .map(|(_, key)| *key)
}

fn char_for_keysym(keyval: u32) -> Option<char> {
    let code = xkbcommon::xkb::keysym_to_utf32(xkbcommon::xkb::Keysym::new(keyval));
    char::from_u32(code).filter(|ch| !ch.is_control())
}

#[cfg(test)]
mod tests {
    use super::{char_for_keysym, is_release, key_event, virtual_key};
    use keymagic_types::VirtualKey;

    #[test]
    fn x_keycode_38_is_the_a_key() {
        assert_eq!(virtual_key(38), Some(VirtualKey::KeyA));
        assert_eq!(virtual_key(8), None);
        assert_eq!(virtual_key(3), None);
    }

    #[test]
    fn latin_and_unicode_keysyms_give_chars() {
        assert_eq!(char_for_keysym(0x61), Some('a'));
        assert_eq!(char_for_keysym(0xe9), Some('é'));
        assert_eq!(char_for_keysym(0x0100_1000), Some('\u{1000}'));
        assert_eq!(char_for_keysym(0x0100_d800), None);
        assert_eq!(char_for_keysym(0xff0d), None);
    }

    #[test]
    fn release_bit_and_modifiers_are_read() {
        assert!(is_release(1 << 30));
        let event = key_event(0x61, 38, (1 << 2) | (1 << 1));
        assert!(event.mods.ctrl);
        assert!(event.mods.caps_lock);
        assert!(!event.mods.shift);
        assert_eq!(event.key, Some(VirtualKey::KeyA));
        assert_eq!(event.ch, Some('a'));
    }
}
