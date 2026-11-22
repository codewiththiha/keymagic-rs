Keep the original minor version and alignment byte for exact round trips.

```
use keymagic_km2::{Document, parse_document, write_document};
use keymagic_types::Keyboard;

let document = Document { keyboard: Keyboard::default(), minor_version: 5, padding: 0 };
let bytes = write_document(&document).ok().unwrap_or_default();
assert_eq!(parse_document(&bytes).ok(), Some(document));
```
