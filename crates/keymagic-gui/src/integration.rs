//! Input-method integration status for each operating system.

use std::env::consts::OS;

/// The operating systems with an integration panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Os {
    Windows,
    MacOs,
    Linux,
    Other,
}

/// The operating system this app runs on.
#[must_use]
pub fn current() -> Os {
    match OS {
        "windows" => Os::Windows,
        "macos" => Os::MacOs,
        "linux" => Os::Linux,
        _ => Os::Other,
    }
}

/// One panel of the Settings page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Panel {
    pub os: Os,
    pub title: &'static str,
    pub status: String,
}

/// The three panels. `connected` is `None` until the user checks.
#[must_use]
pub fn panels(current: Os, connected: Option<bool>) -> Vec<Panel> {
    [
        (Os::Windows, "Windows: Text Services Framework", "Windows"),
        (Os::MacOs, "macOS: Input Method Kit", "macOS"),
        (Os::Linux, "Linux: IBus", "Linux"),
    ]
    .into_iter()
    .map(|(os, title, system)| Panel {
        os,
        title,
        status: status(os == current, connected, system),
    })
    .collect()
}

fn status(is_current: bool, connected: Option<bool>, system: &str) -> String {
    match (is_current, connected) {
        (false, _) => "Not this system.".to_owned(),
        (true, None) => "Not checked yet. Press Check connection.".to_owned(),
        (true, Some(true)) => "Connected: an input method is listening.".to_owned(),
        (true, Some(false)) => {
            format!("Not connected. This build does not register KeyMagic with {system} yet.")
        }
    }
}
