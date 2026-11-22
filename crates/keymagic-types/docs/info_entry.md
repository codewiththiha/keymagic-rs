
```
use keymagic_types::{InfoEntry};
let entry = InfoEntry { id: *b"eman", data: b"Example".to_vec() };
assert_eq!(entry.data, b"Example");
```
