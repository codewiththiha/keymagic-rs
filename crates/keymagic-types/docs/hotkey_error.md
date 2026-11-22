
```
use keymagic_types::{Hotkey, HotkeyError};
assert_eq!("".parse::<Hotkey>(), Err(HotkeyError));
```
