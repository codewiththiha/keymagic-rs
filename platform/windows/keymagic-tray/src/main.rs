//! Windows tray process.

#![forbid(unsafe_code)]

#[cfg(target_os = "windows")]
fn main() -> std::process::ExitCode {
    eprintln!("keymagic-tray: not implemented yet");
    std::process::ExitCode::FAILURE
}

#[cfg(not(target_os = "windows"))]
fn main() -> std::process::ExitCode {
    eprintln!("keymagic-tray: not implemented yet");
    std::process::ExitCode::FAILURE
}
