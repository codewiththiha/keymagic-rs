//! macOS InputMethodKit adapter.

#![forbid(unsafe_code)]

#[cfg(target_os = "macos")]
fn main() {}

#[cfg(not(target_os = "macos"))]
fn main() {}
