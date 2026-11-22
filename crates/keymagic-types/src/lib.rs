//! `KeyMagic` virtual-key IDs and canonical script names.

#![forbid(unsafe_code)]

mod ir;
pub mod opcodes;
pub use ir::*;

#[cfg(feature = "evdev")]
pub mod evdev;
#[cfg(feature = "mac_keycode")]
pub mod mac_keycode;
#[cfg(feature = "win_vk")]
pub mod win_vk;
#[cfg(feature = "xkb_keysym")]
pub mod xkb_keysym;

macro_rules! virtual_keys {
    ($( $variant:ident = $id:literal => $name:literal $( | $alias:literal )*; )+) => {
        /// Internal `KeyMagic` key identifiers, not platform key codes.
        #[repr(u16)]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
        #[doc = include_str!("../docs/virtual_key.md")]
        pub enum VirtualKey {
            $( $variant = $id, )+
        }

        impl VirtualKey {
            /// Every key, in declaration order.
            pub const ALL: &[Self] = &[$( Self::$variant, )+];

            /// Resolve an internal 16-bit ID without unsafe conversion.
            #[must_use]
            pub const fn from_u16(value: u16) -> Option<Self> {
                match value {
                    $( $id => Some(Self::$variant), )+
                    _ => None,
                }
            }

            /// Return the canonical `KeyMagic` script name.
            #[must_use]
            pub const fn kms_name(self) -> &'static str {
                match self {
                    $( Self::$variant => $name, )+
                }
            }

            /// Return the alternate script names for this key.
            #[must_use]
            pub const fn aliases(self) -> &'static [&'static str] {
                match self {
                    $( Self::$variant => &[$( $alias ),*], )+
                }
            }

            /// Resolve a canonical name or a documented alias.
            #[must_use]
            pub fn from_kms_name(name: &str) -> Option<Self> {
                match name {
                    $( $name $( | $alias )* => Some(Self::$variant), )+
                    _ => None,
                }
            }
        }
    };
}

virtual_keys! {
    Null = 1 => "NULL";
    Back = 2 => "VK_BACK";
    Tab = 3 => "VK_TAB";
    Return = 4 => "VK_RETURN" | "VK_ENTER";
    Shift = 5 => "VK_SHIFT";
    Control = 6 => "VK_CONTROL" | "VK_CTRL";
    Menu = 7 => "VK_MENU" | "VK_ALT";
    Pause = 8 => "VK_PAUSE";
    Capital = 9 => "VK_CAPITAL" | "VK_CAPSLOCK";
    Kanji = 10 => "VK_KANJI";
    Escape = 11 => "VK_ESCAPE" | "VK_ESC";
    Space = 12 => "VK_SPACE";
    Prior = 13 => "VK_PRIOR";
    Next = 14 => "VK_NEXT";
    Delete = 15 => "VK_DELETE";
    Key0 = 16 => "VK_KEY_0";
    Key1 = 17 => "VK_KEY_1";
    Key2 = 18 => "VK_KEY_2";
    Key3 = 19 => "VK_KEY_3";
    Key4 = 20 => "VK_KEY_4";
    Key5 = 21 => "VK_KEY_5";
    Key6 = 22 => "VK_KEY_6";
    Key7 = 23 => "VK_KEY_7";
    Key8 = 24 => "VK_KEY_8";
    Key9 = 25 => "VK_KEY_9";
    KeyA = 26 => "VK_KEY_A";
    KeyB = 27 => "VK_KEY_B";
    KeyC = 28 => "VK_KEY_C";
    KeyD = 29 => "VK_KEY_D";
    KeyE = 30 => "VK_KEY_E";
    KeyF = 31 => "VK_KEY_F";
    KeyG = 32 => "VK_KEY_G";
    KeyH = 33 => "VK_KEY_H";
    KeyI = 34 => "VK_KEY_I";
    KeyJ = 35 => "VK_KEY_J";
    KeyK = 36 => "VK_KEY_K";
    KeyL = 37 => "VK_KEY_L";
    KeyM = 38 => "VK_KEY_M";
    KeyN = 39 => "VK_KEY_N";
    KeyO = 40 => "VK_KEY_O";
    KeyP = 41 => "VK_KEY_P";
    KeyQ = 42 => "VK_KEY_Q";
    KeyR = 43 => "VK_KEY_R";
    KeyS = 44 => "VK_KEY_S";
    KeyT = 45 => "VK_KEY_T";
    KeyU = 46 => "VK_KEY_U";
    KeyV = 47 => "VK_KEY_V";
    KeyW = 48 => "VK_KEY_W";
    KeyX = 49 => "VK_KEY_X";
    KeyY = 50 => "VK_KEY_Y";
    KeyZ = 51 => "VK_KEY_Z";
    Numpad0 = 52 => "VK_NUMPAD0";
    Numpad1 = 53 => "VK_NUMPAD1";
    Numpad2 = 54 => "VK_NUMPAD2";
    Numpad3 = 55 => "VK_NUMPAD3";
    Numpad4 = 56 => "VK_NUMPAD4";
    Numpad5 = 57 => "VK_NUMPAD5";
    Numpad6 = 58 => "VK_NUMPAD6";
    Numpad7 = 59 => "VK_NUMPAD7";
    Numpad8 = 60 => "VK_NUMPAD8";
    Numpad9 = 61 => "VK_NUMPAD9";
    Multiply = 62 => "VK_MULTIPLY";
    Add = 63 => "VK_ADD";
    Separator = 64 => "VK_SEPARATOR";
    Subtract = 65 => "VK_SUBTRACT";
    Decimal = 66 => "VK_DECIMAL";
    Divide = 67 => "VK_DIVIDE";
    F1 = 68 => "VK_F1";
    F2 = 69 => "VK_F2";
    F3 = 70 => "VK_F3";
    F4 = 71 => "VK_F4";
    F5 = 72 => "VK_F5";
    F6 = 73 => "VK_F6";
    F7 = 74 => "VK_F7";
    F8 = 75 => "VK_F8";
    F9 = 76 => "VK_F9";
    F10 = 77 => "VK_F10";
    F11 = 78 => "VK_F11";
    F12 = 79 => "VK_F12";
    LShift = 80 => "VK_LSHIFT";
    RShift = 81 => "VK_RSHIFT";
    LControl = 82 => "VK_LCONTROL" | "VK_LCTRL";
    RControl = 83 => "VK_RCONTROL" | "VK_RCTRL";
    LMenu = 84 => "VK_LMENU" | "VK_LALT";
    RMenu = 85 => "VK_RMENU" | "VK_RALT" | "VK_ALT_GR";
    Oem1 = 86 => "VK_OEM_1" | "VK_COLON";
    OemPlus = 87 => "VK_OEM_PLUS";
    OemComma = 88 => "VK_OEM_COMMA";
    OemMinus = 89 => "VK_OEM_MINUS";
    OemPeriod = 90 => "VK_OEM_PERIOD";
    Oem2 = 91 => "VK_OEM_2" | "VK_QUESTION";
    Oem3 = 92 => "VK_OEM_3" | "VK_CFLEX";
    Oem4 = 93 => "VK_OEM_4" | "VK_LBRACKET";
    Oem5 = 94 => "VK_OEM_5" | "VK_BACKSLASH";
    Oem6 = 95 => "VK_OEM_6" | "VK_RBRACKET";
    Oem7 = 96 => "VK_OEM_7" | "VK_QUOTE";
    Oem8 = 97 => "VK_OEM_8" | "VK_EXCM";
    OemAx = 98 => "VK_OEM_AX";
    Oem102 = 99 => "VK_OEM_102" | "VK_LESSTHEN";
    IcoHelp = 100 => "VK_ICO_HELP";
    Ico00 = 101 => "VK_ICO_00";
}

impl VirtualKey {
    /// Display name for the FFI; `NULL` has none.
    #[must_use]
    #[expect(
        clippy::too_many_lines,
        reason = "one arm per key keeps the table exhaustive"
    )]
    pub const fn display_name(self) -> Option<&'static str> {
        match self {
            Self::Null => None,
            Self::Back => Some("Backspace"),
            Self::Tab => Some("Tab"),
            Self::Return => Some("Enter"),
            Self::Shift => Some("Shift"),
            Self::Control => Some("Ctrl"),
            Self::Menu => Some("Alt"),
            Self::Pause => Some("Pause"),
            Self::Capital => Some("CapsLock"),
            Self::Kanji => Some("Kanji"),
            Self::Escape => Some("Esc"),
            Self::Space => Some("Space"),
            Self::Prior => Some("PageUp"),
            Self::Next => Some("PageDown"),
            Self::Delete => Some("Delete"),
            Self::Key0 => Some("0"),
            Self::Key1 => Some("1"),
            Self::Key2 => Some("2"),
            Self::Key3 => Some("3"),
            Self::Key4 => Some("4"),
            Self::Key5 => Some("5"),
            Self::Key6 => Some("6"),
            Self::Key7 => Some("7"),
            Self::Key8 => Some("8"),
            Self::Key9 => Some("9"),
            Self::KeyA => Some("A"),
            Self::KeyB => Some("B"),
            Self::KeyC => Some("C"),
            Self::KeyD => Some("D"),
            Self::KeyE => Some("E"),
            Self::KeyF => Some("F"),
            Self::KeyG => Some("G"),
            Self::KeyH => Some("H"),
            Self::KeyI => Some("I"),
            Self::KeyJ => Some("J"),
            Self::KeyK => Some("K"),
            Self::KeyL => Some("L"),
            Self::KeyM => Some("M"),
            Self::KeyN => Some("N"),
            Self::KeyO => Some("O"),
            Self::KeyP => Some("P"),
            Self::KeyQ => Some("Q"),
            Self::KeyR => Some("R"),
            Self::KeyS => Some("S"),
            Self::KeyT => Some("T"),
            Self::KeyU => Some("U"),
            Self::KeyV => Some("V"),
            Self::KeyW => Some("W"),
            Self::KeyX => Some("X"),
            Self::KeyY => Some("Y"),
            Self::KeyZ => Some("Z"),
            Self::Numpad0 => Some("Num0"),
            Self::Numpad1 => Some("Num1"),
            Self::Numpad2 => Some("Num2"),
            Self::Numpad3 => Some("Num3"),
            Self::Numpad4 => Some("Num4"),
            Self::Numpad5 => Some("Num5"),
            Self::Numpad6 => Some("Num6"),
            Self::Numpad7 => Some("Num7"),
            Self::Numpad8 => Some("Num8"),
            Self::Numpad9 => Some("Num9"),
            Self::Multiply => Some("Multiply"),
            Self::Add => Some("Add"),
            Self::Separator => Some("Separator"),
            Self::Subtract => Some("Subtract"),
            Self::Decimal => Some("Decimal"),
            Self::Divide => Some("Divide"),
            Self::F1 => Some("F1"),
            Self::F2 => Some("F2"),
            Self::F3 => Some("F3"),
            Self::F4 => Some("F4"),
            Self::F5 => Some("F5"),
            Self::F6 => Some("F6"),
            Self::F7 => Some("F7"),
            Self::F8 => Some("F8"),
            Self::F9 => Some("F9"),
            Self::F10 => Some("F10"),
            Self::F11 => Some("F11"),
            Self::F12 => Some("F12"),
            Self::LShift => Some("LShift"),
            Self::RShift => Some("RShift"),
            Self::LControl => Some("LCtrl"),
            Self::RControl => Some("RCtrl"),
            Self::LMenu => Some("LAlt"),
            Self::RMenu => Some("RAlt"),
            Self::Oem1 => Some(";"),
            Self::OemPlus => Some("+"),
            Self::OemComma => Some(","),
            Self::OemMinus => Some("-"),
            Self::OemPeriod => Some("."),
            Self::Oem2 => Some("/"),
            Self::Oem3 => Some("`"),
            Self::Oem4 => Some("["),
            Self::Oem5 => Some("\\"),
            Self::Oem6 => Some("]"),
            Self::Oem7 => Some("'"),
            Self::Oem8 => Some("OEM8"),
            Self::OemAx => Some("AX"),
            Self::Oem102 => Some("OEM102"),
            Self::IcoHelp => Some("Help"),
            Self::Ico00 => Some("00"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::VirtualKey;

    #[test]
    fn every_key_but_null_has_a_display_name() {
        for key in VirtualKey::ALL {
            assert_eq!(key.display_name().is_none(), *key == VirtualKey::Null);
        }
    }
}
