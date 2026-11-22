//! Check platform key codes remain distinct from internal identifiers.

#[cfg(any(
    feature = "win_vk",
    feature = "mac_keycode",
    feature = "evdev",
    feature = "xkb_keysym"
))]
use keymagic_types::VirtualKey;

#[cfg(feature = "win_vk")]
#[test]
fn windows_codes() {
    assert_eq!(
        keymagic_types::win_vk::from_code(0x08),
        Some(VirtualKey::Back)
    );
    assert_eq!(keymagic_types::win_vk::from_code(2), None);
    assert_eq!(
        keymagic_types::win_vk::from_code(0x41),
        Some(VirtualKey::KeyA)
    );
}

#[cfg(feature = "mac_keycode")]
#[test]
fn mac_codes() {
    assert_eq!(
        keymagic_types::mac_keycode::from_code(0),
        Some(VirtualKey::KeyA)
    );
    assert_eq!(
        keymagic_types::mac_keycode::from_code(51),
        Some(VirtualKey::Back)
    );
}

#[cfg(feature = "evdev")]
#[test]
fn linux_codes() {
    assert_eq!(keymagic_types::evdev::from_code(30), Some(VirtualKey::KeyA));
    assert_eq!(keymagic_types::evdev::from_code(14), Some(VirtualKey::Back));
}

#[cfg(feature = "xkb_keysym")]
#[test]
fn x11_symbols() {
    assert_eq!(
        keymagic_types::xkb_keysym::from_code(0x61),
        Some(VirtualKey::KeyA)
    );
    assert_eq!(
        keymagic_types::xkb_keysym::from_code(0x41),
        Some(VirtualKey::KeyA)
    );
    assert_eq!(
        keymagic_types::xkb_keysym::from_code(0xFF08),
        Some(VirtualKey::Back)
    );
}
