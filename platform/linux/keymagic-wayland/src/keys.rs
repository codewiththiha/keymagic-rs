//! Keymap state for the keys that the input-method grab delivers.

use keymagic_engine::{KeyEvent, Modifiers};
use keymagic_types::evdev;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::fd::OwnedFd;
use xkbcommon::xkb;

/// Evdev codes sit 8 below X keycodes.
const X_OFFSET: u32 = 8;

/// A compiled xkb keymap with the modifier state the grab reports.
pub struct Keymap {
    state: xkb::State,
}

impl Keymap {
    /// Compiles keymap text; `None` when xkb rejects it.
    #[must_use]
    pub fn compile(text: String) -> Option<Self> {
        let context = xkb::Context::new(xkb::CONTEXT_NO_FLAGS);
        let keymap = xkb::Keymap::new_from_string(
            &context,
            text,
            xkb::KEYMAP_FORMAT_TEXT_V1,
            xkb::KEYMAP_COMPILE_NO_FLAGS,
        )?;
        Some(Self {
            state: xkb::State::new(&keymap),
        })
    }

    /// Sets the modifier masks and layout that the grab reported.
    pub fn set_modifiers(&mut self, depressed: u32, latched: u32, locked: u32, group: u32) {
        self.state
            .update_mask(depressed, latched, locked, 0, 0, group);
    }

    /// Builds the engine event for an evdev key press.
    #[must_use]
    pub fn key_event(&self, code: u32) -> KeyEvent {
        let keycode = xkb::Keycode::new(code.saturating_add(X_OFFSET));
        let sym = self.state.key_get_one_sym(keycode);
        KeyEvent {
            key: u16::try_from(code).ok().and_then(evdev::from_code),
            ch: char_for(sym),
            mods: self.modifiers(),
        }
    }

    fn modifiers(&self) -> Modifiers {
        Modifiers {
            shift: self.active(xkb::MOD_NAME_SHIFT),
            ctrl: self.active(xkb::MOD_NAME_CTRL),
            alt: self.active(xkb::MOD_NAME_ALT),
            caps_lock: self.active(xkb::MOD_NAME_CAPS),
        }
    }

    fn active(&self, name: &str) -> bool {
        self.state
            .mod_name_is_active(name, xkb::STATE_MODS_EFFECTIVE)
    }
}

fn char_for(sym: xkb::Keysym) -> Option<char> {
    char::from_u32(xkb::keysym_to_utf32(sym)).filter(|ch| !ch.is_control())
}

/// The default keymap text, from the xkb rules and layout defaults.
#[must_use]
pub fn default_text() -> Option<String> {
    let context = xkb::Context::new(xkb::CONTEXT_NO_FLAGS);
    let keymap =
        xkb::Keymap::new_from_names(&context, "", "", "", "", None, xkb::KEYMAP_COMPILE_NO_FLAGS)?;
    Some(keymap.get_as_string(xkb::KEYMAP_FORMAT_TEXT_V1))
}

/// Reads `size` bytes of keymap text from a keymap descriptor.
#[must_use]
pub fn read_text(fd: OwnedFd, size: u32) -> Option<String> {
    let mut file = File::from(fd);
    file.seek(SeekFrom::Start(0)).ok()?;
    let mut bytes = vec![0; usize::try_from(size).ok()?];
    file.read_exact(&mut bytes).ok()?;
    let end = bytes
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(bytes.len());
    bytes.truncate(end);
    String::from_utf8(bytes).ok()
}

/// Writes keymap text, with a NUL, to a file for a virtual keyboard.
#[must_use]
pub fn write_text(text: &str) -> Option<(File, u32)> {
    let mut bytes = text.as_bytes().to_vec();
    bytes.push(0);
    let path = std::env::temp_dir().join(format!("keymagic-keymap-{}.xkb", std::process::id()));
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .open(&path)
        .ok()?;
    std::fs::remove_file(&path).ok()?;
    file.write_all(&bytes).ok()?;
    file.seek(SeekFrom::Start(0)).ok()?;
    Some((file, u32::try_from(bytes.len()).ok()?))
}

#[cfg(test)]
mod tests {
    use super::{Keymap, default_text, read_text, write_text};
    use keymagic_types::VirtualKey;
    use std::os::fd::OwnedFd;

    #[test]
    fn letters_and_shift_give_chars_and_keys() {
        let Some(text) = default_text() else {
            panic!("no default keymap");
        };
        let Some(mut keymap) = Keymap::compile(text) else {
            panic!("the default keymap was rejected");
        };
        let event = keymap.key_event(30);
        assert_eq!(event.key, Some(VirtualKey::KeyA));
        assert_eq!(event.ch, Some('a'));
        keymap.set_modifiers(1, 0, 0, 0);
        let shifted = keymap.key_event(16);
        assert_eq!(shifted.ch, Some('Q'));
        assert!(shifted.mods.shift);
    }

    #[test]
    fn keymap_text_round_trips_through_a_file() {
        let Some((file, size)) = write_text("abc") else {
            panic!("cannot write the keymap");
        };
        assert_eq!(read_text(OwnedFd::from(file), size), Some("abc".to_owned()));
    }
}
