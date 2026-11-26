//! Builds the `IBus` property menu that lists the enabled keyboards.

use crate::text::{prop_list, property};
use keymagic_host::integration::ibus::Engine;
use zbus::Variant;

/// Key prefix of each keyboard option; the rest is the keyboard id.
pub const KEY_PREFIX: &str = "keymagic:";

/// `IBusPropState` for a checked option.
pub const CHECKED: u32 = 1;

const MENU_KEY: &str = "keymagic";
const MENU: u32 = 3;
const RADIO: u32 = 2;
const UNCHECKED: u32 = 0;
const DEFAULT_LABEL: &str = "KeyMagic";

/// Menu of `engines` with the keyboard `current` checked, as an `IBusPropList`.
pub fn menu(engines: &[Engine], current: &str) -> zbus::Result<Variant<'static>> {
    let mut options = Vec::with_capacity(engines.len());
    for engine in engines {
        let state = if engine.id == current {
            CHECKED
        } else {
            UNCHECKED
        };
        let key = format!("{KEY_PREFIX}{}", engine.id);
        let no_sub_props = prop_list(&[])?;
        options.push(property(&key, RADIO, &engine.label, state, no_sub_props));
    }
    let label = engines
        .iter()
        .find(|engine| engine.id == current)
        .map_or(DEFAULT_LABEL, |engine| engine.label.as_str());
    let sub_props = prop_list(&options)?;
    let top = property(MENU_KEY, MENU, label, UNCHECKED, sub_props);
    prop_list(&[top])
}

#[cfg(test)]
mod tests {
    use super::menu;
    use keymagic_host::integration::ibus::Engine;

    fn engine(id: &str, label: &str) -> Engine {
        Engine {
            id: id.to_owned(),
            label: label.to_owned(),
            icon: None,
        }
    }

    #[test]
    fn menu_lists_each_keyboard_by_its_engine_key() {
        let engines = [engine("unicode", "Unicode"), engine("zawgyi", "Zawgyi")];
        let Ok(list) = menu(&engines, "zawgyi") else {
            panic!("the menu must build");
        };
        assert_eq!(list.value_signature().to_string(), "(sa{sv}av)");
        let text = list.to_string();
        assert!(text.contains("keymagic:unicode"));
        assert!(text.contains("keymagic:zawgyi"));
    }

    #[test]
    fn menu_with_no_keyboards_still_builds() {
        let Ok(list) = menu(&[], "missing") else {
            panic!("an empty menu must build");
        };
        assert_eq!(list.value_signature().to_string(), "(sa{sv}av)");
    }
}
