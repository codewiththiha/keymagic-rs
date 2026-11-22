//! Check that arbitrary inputs cannot crash the parser.

#![no_main]
#![forbid(unsafe_code)]

use keymagic_km2::{parse_document, read_info_only, write_document};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = read_info_only(data);
    if let Ok(document) = parse_document(data) {
        if let Ok(encoded) = write_document(&document) {
            assert_eq!(encoded, data);
        }
    }
});
