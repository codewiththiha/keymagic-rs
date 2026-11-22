
```
use keymagic_types::{Modifiers};
let mods = Modifiers::CTRL | Modifiers::SHIFT;
assert!(mods.contains(Modifiers::CTRL));
```
