
```
use keymagic_types::{Hotkey, VirtualKey};
let parsed = "CTRL+M".parse::<Hotkey>();
assert_eq!(parsed.map(|key| key.key), Ok(VirtualKey::KeyM));
```
