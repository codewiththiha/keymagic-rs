//! Linux Wayland input-method adapter.

#![forbid(unsafe_code)]

#[cfg(target_os = "linux")]
fn main() -> std::process::ExitCode {
    eprintln!("keymagic-wayland: not implemented yet");
    std::process::ExitCode::FAILURE
}

#[cfg(not(target_os = "linux"))]
fn main() -> std::process::ExitCode {
    eprintln!("keymagic-wayland: not implemented yet");
    std::process::ExitCode::FAILURE
}
