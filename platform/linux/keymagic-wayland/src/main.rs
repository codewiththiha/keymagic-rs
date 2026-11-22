//! Linux Wayland input-method adapter.

#![forbid(unsafe_code)]

#[cfg(target_os = "linux")]
fn main() {}

#[cfg(not(target_os = "linux"))]
fn main() {}
