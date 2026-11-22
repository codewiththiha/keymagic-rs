//! Windows tray process.

#![forbid(unsafe_code)]

#[cfg(target_os = "windows")]
fn main() {}

#[cfg(not(target_os = "windows"))]
fn main() {}
