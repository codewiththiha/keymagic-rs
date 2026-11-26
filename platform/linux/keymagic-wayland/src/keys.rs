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
