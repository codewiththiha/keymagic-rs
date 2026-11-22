Match on the kind to distinguish truncation from invalid content.

```
use keymagic_km2::{Km2ErrorKind, parse};

let error = parse(b"KM").err();
assert_eq!(error.map(|e| e.kind), Some(Km2ErrorKind::Truncated));
```
