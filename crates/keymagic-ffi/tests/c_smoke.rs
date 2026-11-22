//! Runs the C client in `ffi_smoke.c` against the exported ABI.

#![cfg(feature = "c-smoke")]

use std::ffi::c_int;

use keymagic_core as _;

unsafe extern "C" {
    fn keymagic_ffi_smoke() -> c_int;
}

#[test]
fn c_client_round_trips_through_the_engine() {
    // SAFETY: the C function takes no arguments and only calls the ABI.
    assert_eq!(unsafe { keymagic_ffi_smoke() }, 0);
}
