# Virtual keys — upstream reference

This table transcribes the KeyMagic 3 internal predefined-key IDs at revision
[`2db30dc8e8a5f091d1434a2bc1702b29cf113804`](https://github.com/thantthet/keymagic-3/blob/2db30dc8e8a5f091d1434a2bc1702b29cf113804/keymagic-core/src/types/virtual_keys.rs).
The order and names were cross-checked against the legacy
[KeyCodes.h](https://github.com/thantthet/keymagic/blob/6a662fb071e1f0e0c9271f84e28712b9e78962ab/cross-platform/libkeymagic/KeyCodes.h).
These are **KeyMagic internal IDs**, not Windows virtual-key numbers. For
example `VK_BACK` is 2 here, not the Windows `VK_BACK` value. The Rust enum also includes `Null = 1`, the RHS deletion marker.
Snapshot tests regenerate both tables from the implementation.

Optional `win_vk`, `mac_keycode`, `evdev`, and `xkb_keysym` features provide
pure conversion tables. Windows and XKB mappings follow the pinned KM3 core
and IBus sources; macOS follows its `KeycodeMapping.swift` and Carbon virtual
key codes, while evdev uses Linux input-event codes. XKB keysyms are logical
symbols and do not identify physical keys; macOS and evdev codes are physical.
Mappings return no value for unsupported codes and are not platform adapters.

## Internal IDs

| ID | KM3 Rust variant | Legacy ID name |
| ---: | --- | --- |
| 2 | `Back` | `VK_BACK` |
| 3 | `Tab` | `VK_TAB` |
| 4 | `Return` | `VK_RETURN` |
| 5 | `Shift` | `VK_SHIFT` |
| 6 | `Control` | `VK_CONTROL` |
| 7 | `Menu` | `VK_MENU` |
| 8 | `Pause` | `VK_PAUSE` |
| 9 | `Capital` | `VK_CAPITAL` |
| 10 | `Kanji` | `VK_KANJI` |
| 11 | `Escape` | `VK_ESCAPE` |
| 12 | `Space` | `VK_SPACE` |
| 13 | `Prior` | `VK_PRIOR` |
| 14 | `Next` | `VK_NEXT` |
| 15 | `Delete` | `VK_DELETE` |
| 16 | `Key0` | `VK_KEY_0` |
| 17 | `Key1` | `VK_KEY_1` |
| 18 | `Key2` | `VK_KEY_2` |
| 19 | `Key3` | `VK_KEY_3` |
| 20 | `Key4` | `VK_KEY_4` |
| 21 | `Key5` | `VK_KEY_5` |
| 22 | `Key6` | `VK_KEY_6` |
| 23 | `Key7` | `VK_KEY_7` |
| 24 | `Key8` | `VK_KEY_8` |
| 25 | `Key9` | `VK_KEY_9` |
| 26 | `KeyA` | `VK_KEY_A` |
| 27 | `KeyB` | `VK_KEY_B` |
| 28 | `KeyC` | `VK_KEY_C` |
| 29 | `KeyD` | `VK_KEY_D` |
| 30 | `KeyE` | `VK_KEY_E` |
| 31 | `KeyF` | `VK_KEY_F` |
| 32 | `KeyG` | `VK_KEY_G` |
| 33 | `KeyH` | `VK_KEY_H` |
| 34 | `KeyI` | `VK_KEY_I` |
| 35 | `KeyJ` | `VK_KEY_J` |
| 36 | `KeyK` | `VK_KEY_K` |
| 37 | `KeyL` | `VK_KEY_L` |
| 38 | `KeyM` | `VK_KEY_M` |
| 39 | `KeyN` | `VK_KEY_N` |
| 40 | `KeyO` | `VK_KEY_O` |
| 41 | `KeyP` | `VK_KEY_P` |
| 42 | `KeyQ` | `VK_KEY_Q` |
| 43 | `KeyR` | `VK_KEY_R` |
| 44 | `KeyS` | `VK_KEY_S` |
| 45 | `KeyT` | `VK_KEY_T` |
| 46 | `KeyU` | `VK_KEY_U` |
| 47 | `KeyV` | `VK_KEY_V` |
| 48 | `KeyW` | `VK_KEY_W` |
| 49 | `KeyX` | `VK_KEY_X` |
| 50 | `KeyY` | `VK_KEY_Y` |
| 51 | `KeyZ` | `VK_KEY_Z` |
| 52 | `Numpad0` | `VK_NUMPAD0` |
| 53 | `Numpad1` | `VK_NUMPAD1` |
| 54 | `Numpad2` | `VK_NUMPAD2` |
| 55 | `Numpad3` | `VK_NUMPAD3` |
| 56 | `Numpad4` | `VK_NUMPAD4` |
| 57 | `Numpad5` | `VK_NUMPAD5` |
| 58 | `Numpad6` | `VK_NUMPAD6` |
| 59 | `Numpad7` | `VK_NUMPAD7` |
| 60 | `Numpad8` | `VK_NUMPAD8` |
| 61 | `Numpad9` | `VK_NUMPAD9` |
| 62 | `Multiply` | `VK_MULTIPLY` |
| 63 | `Add` | `VK_ADD` |
| 64 | `Separator` | `VK_SEPARATOR` |
| 65 | `Subtract` | `VK_SUBTRACT` |
| 66 | `Decimal` | `VK_DECIMAL` |
| 67 | `Divide` | `VK_DIVIDE` |
| 68 | `F1` | `VK_F1` |
| 69 | `F2` | `VK_F2` |
| 70 | `F3` | `VK_F3` |
| 71 | `F4` | `VK_F4` |
| 72 | `F5` | `VK_F5` |
| 73 | `F6` | `VK_F6` |
| 74 | `F7` | `VK_F7` |
| 75 | `F8` | `VK_F8` |
| 76 | `F9` | `VK_F9` |
| 77 | `F10` | `VK_F10` |
| 78 | `F11` | `VK_F11` |
| 79 | `F12` | `VK_F12` |
| 80 | `LShift` | `VK_LSHIFT` |
| 81 | `RShift` | `VK_RSHIFT` |
| 82 | `LControl` | `VK_LCONTROL` |
| 83 | `RControl` | `VK_RCONTROL` |
| 84 | `LMenu` | `VK_LMENU` |
| 85 | `RMenu` | `VK_RMENU` |
| 86 | `Oem1` | `VK_OEM_1` |
| 87 | `OemPlus` | `VK_OEM_PLUS` |
| 88 | `OemComma` | `VK_OEM_COMMA` |
| 89 | `OemMinus` | `VK_OEM_MINUS` |
| 90 | `OemPeriod` | `VK_OEM_PERIOD` |
| 91 | `Oem2` | `VK_OEM_2` |
| 92 | `Oem3` | `VK_OEM_3` |
| 93 | `Oem4` | `VK_OEM_4` |
| 94 | `Oem5` | `VK_OEM_5` |
| 95 | `Oem6` | `VK_OEM_6` |
| 96 | `Oem7` | `VK_OEM_7` |
| 97 | `Oem8` | `VK_OEM_8` |
| 98 | `OemAx` | `VK_OEM_AX` |
| 99 | `Oem102` | `VK_OEM_102` |
| 100 | `IcoHelp` | `VK_ICO_HELP` |
| 101 | `Ico00` | `VK_ICO_00` |

## Name aliases

These additional `create_vk_map` names resolve to the corresponding variant.

| Alias | Variant | ID |
| --- | --- | ---: |
| `VK_ENTER` | `Return` | 4 |
| `VK_CTRL` | `Control` | 6 |
| `VK_ALT` | `Menu` | 7 |
| `VK_CAPSLOCK` | `Capital` | 9 |
| `VK_ESC` | `Escape` | 11 |
| `VK_LCTRL` | `LControl` | 82 |
| `VK_RCTRL` | `RControl` | 83 |
| `VK_LALT` | `LMenu` | 84 |
| `VK_RALT` | `RMenu` | 85 |
| `VK_ALT_GR` | `RMenu` | 85 |
| `VK_COLON` | `Oem1` | 86 |
| `VK_QUESTION` | `Oem2` | 91 |
| `VK_CFLEX` | `Oem3` | 92 |
| `VK_LBRACKET` | `Oem4` | 93 |
| `VK_BACKSLASH` | `Oem5` | 94 |
| `VK_RBRACKET` | `Oem6` | 95 |
| `VK_QUOTE` | `Oem7` | 96 |
| `VK_EXCM` | `Oem8` | 97 |
| `VK_LESSTHEN` | `Oem102` | 99 |
