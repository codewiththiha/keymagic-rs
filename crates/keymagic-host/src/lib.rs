//! Host policy shared by every platform adapter: paths, config, keyboard
//! files, commit rules, IPC framing and update checks. Adapters translate
//! OS events and apply the actions they receive.

pub mod bundled;
pub mod config;
pub mod integration;
pub mod ipc;
pub mod paths;
pub mod session;
pub mod store;
pub mod updater;

pub use config::Config;
pub use paths::Paths;
pub use session::{HostAction, Mode, Session};
pub use store::KeyboardStore;

pub(crate) fn invalid_data(error: impl std::fmt::Display) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidData, error.to_string())
}
