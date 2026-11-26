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
    pub fix: Option<Fix>,
    pub deferred: Option<Deferred>,
}

/// A control that works only once the platform's input method is installed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Deferred {
    pub note: &'static str,
    pub action: &'static str,
}

/// A repair run as the current user: never elevated, never via a shell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fix {
    pub label: &'static str,
    pub program: &'static str,
    pub args: &'static [&'static str],
}

impl Fix {
    /// The command as the user reads it in the confirmation.
    #[must_use]
    pub fn command_line(&self) -> String {
        std::iter::once(self.program)
            .chain(self.args.iter().copied())
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// Linux cards: title, process name (at most 15 characters), and repair.
const LINUX_CARDS: [(&str, &str, Fix); 4] = [
    (
        "Linux: IBus",
        "ibus-daemon",
        Fix {
            label: "Restart IBus",
            program: "ibus",
            args: &["restart"],
        },
    ),
    (
        "Linux: Wayland",
        "xdg-desktop-por",
        Fix {
            label: "Restart the portal",
            program: "systemctl",
            args: &["--user", "restart", "xdg-desktop-portal"],
        },
    ),
    (
        "Linux: KDE Plasma",
        "kwin_wayland",
        Fix {
            label: "Rebuild KDE cache",
            program: "kbuildsycoca6",
            args: &["--noincremental"],
        },
    ),
    (
        "Linux: Fcitx5",
        "fcitx5",
        Fix {
            label: "Restart Fcitx5",
            program: "fcitx5",
            args: &["-r"],
        },
    ),
];

/// Whether each Linux card's process is running now.
#[must_use]
pub fn probe_linux() -> Vec<bool> {
    let running = crate::apps::running();
    LINUX_CARDS
        .iter()
        .map(|(_, process, _)| running.iter().any(|name| name == *process))
        .collect()
}

/// Runs a repair as the current user and returns the message to show.
#[must_use]
pub fn run_fix(fix: &Fix) -> String {
    let command = fix.command_line();
    match std::process::Command::new(fix.program)
        .args(fix.args)
        .status()
    {
        Ok(status) if status.success() => {
            format!("Ran `{command}`. Check connection again.")
        }
        Ok(status) => format!("`{command}` exited with {status}."),
        Err(error) => format!("Could not run `{command}`: {error}."),
    }
}

/// Runs a repair off the UI thread and returns a channel with its message.
#[must_use]
pub fn start_fix(fix: Fix) -> std::sync::mpsc::Receiver<String> {
    let (sender, receiver) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = sender.send(run_fix(&fix));
    });
    receiver
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

/// Panels for this system; Linux cards take the per-card probe results.
#[must_use]
pub fn panels(current: Os, connected: Option<bool>, linux_running: Option<&[bool]>) -> Vec<Panel> {
    if current == Os::Linux {
        return LINUX_CARDS
            .iter()
            .enumerate()
            .map(|(index, (title, _, fix))| {
                let running = linux_running.and_then(|flags| flags.get(index).copied());
                Panel {
                    os: Os::Linux,
                    title,
                    status: linux_status(running),
                    health: linux_health(running),
                    fix: Some(*fix),
                    deferred: None,
                }
            })
            .collect();
    }
    [
        (Os::Windows, "Windows: Text Services Framework", "Windows"),
        (Os::MacOs, "macOS: Input Method Kit", "macOS"),
    ]
    .into_iter()
    .map(|(os, title, system)| Panel {
        os,
        title,
        status: status(os == current, connected, system),
        health: health(os == current, connected),
        fix: None,
        deferred: deferred(os),
    })
    .collect()
}

/// The language-profile or input-mode control for a system, if it has one.
const fn deferred(os: Os) -> Option<Deferred> {
    match os {
        Os::Windows => Some(Deferred {
            note: "Available once the Windows input method is installed.",
            action: "Apply language profiles",
        }),
        Os::MacOs => Some(Deferred {
            note: "Available once the macOS input method is installed.",
            action: "Regenerate input modes",
        }),
        Os::Linux | Os::Other => None,
    }
}

const fn linux_health(running: Option<bool>) -> Health {
    match running {
        None => Health::Unchecked,
        Some(true) => Health::Ready,
        Some(false) => Health::Missing,
    }
}

fn linux_status(running: Option<bool>) -> String {
    match running {
        None => "Not checked yet. Press Check connection.".to_owned(),
        Some(true) => "Running.".to_owned(),
        Some(false) => "Not running. Restart it with the button below.".to_owned(),
    }
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

#[cfg(test)]
mod tests {
    use super::LINUX_CARDS;

    #[test]
    fn linux_process_names_fit_the_kernel_name_length() {
        assert!(
            LINUX_CARDS
                .iter()
                .all(|(_, process, _)| process.len() <= 15)
        );
    }
}
