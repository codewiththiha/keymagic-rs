//! macOS `InputMethodKit` adapter.

#![forbid(unsafe_code)]

#[cfg(target_os = "macos")]
fn main() -> std::process::ExitCode {
    eprintln!("keymagic-imk: not implemented yet");
    std::process::ExitCode::FAILURE
}

#[cfg(not(target_os = "macos"))]
fn main() -> std::process::ExitCode {
    eprintln!("keymagic-imk: not implemented yet");
    std::process::ExitCode::FAILURE
}
