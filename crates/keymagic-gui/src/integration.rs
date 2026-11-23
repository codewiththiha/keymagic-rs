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
    pub health: Health,
}

/// How an integration looks right now; drives the status dot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Health {
    Unchecked,
    Ready,
    Missing,
    Elsewhere,
}

/// The OS page where the user finishes the integration, when there is one.
#[must_use]
pub const fn settings_uri(os: Os) -> Option<&'static str> {
    match os {
        Os::Windows => Some("ms-settings:regionlanguage"),
        Os::MacOs => Some("x-apple.systempreferences:com.apple.Keyboard-Settings.extension"),
        Os::Linux | Os::Other => None,
    }
}

const fn health(is_current: bool, connected: Option<bool>) -> Health {
    match (is_current, connected) {
        (false, _) => Health::Elsewhere,
        (true, None) => Health::Unchecked,
        (true, Some(true)) => Health::Ready,
        (true, Some(false)) => Health::Missing,
    }
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
        health: health(os == current, connected),
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
