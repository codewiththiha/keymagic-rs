//! Wayland input-method client for keymagic, using input-method-v2.

#[cfg(target_os = "linux")]
mod engine;
#[cfg(target_os = "linux")]
mod keys;
#[cfg(target_os = "linux")]
mod probe;
#[cfg(target_os = "linux")]
mod tracker;

#[cfg(target_os = "linux")]
fn main() -> std::process::ExitCode {
    probe::run()
}

#[cfg(not(target_os = "linux"))]
fn main() -> std::process::ExitCode {
    eprintln!("keymagic-wayland runs on Linux only");
    std::process::ExitCode::FAILURE
}
