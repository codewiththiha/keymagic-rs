
```
use keymagic_types::{SourceSpan};
let span = SourceSpan { start: 1, end: 4 };
assert_eq!(span.end - span.start, 3);
```
