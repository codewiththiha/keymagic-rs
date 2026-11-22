
```
use keymagic_types::{ValidationError};
let err = ValidationError { rule: 0, element: 1, message: "bad index" };
assert_eq!(err.element, 1);
```
