Errors carry the byte offset where parsing or serialization stopped.

```
use keymagic_km2::parse;

let error = parse(b"KMKL\x01\x05\x00").err();
assert_eq!(error.map(|e| e.offset), Some(6));
```
