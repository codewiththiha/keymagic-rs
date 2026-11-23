//! Opens files and folders in the system file manager.

use std::io;
use std::path::Path;
use std::process::Command;

/// Opens `folder` in the file manager.
#[doc = "# Errors\n\nReturns an error when the file manager cannot be started."]
pub fn open_folder(folder: &Path) -> io::Result<()> {
    let mut command = if cfg!(target_os = "windows") {
        let mut command = Command::new("explorer");
        command.arg(folder);
        command
    } else if cfg!(target_os = "macos") {
        let mut command = Command::new("open");
        command.arg(folder);
        command
    } else {
        let mut command = Command::new("xdg-open");
        command.arg(folder);
        command
    };
    command.spawn().map(drop)
}

/// Opens the file manager with `file` selected.
#[doc = "# Errors\n\nReturns an error when the file manager cannot be started."]
pub fn show_file(file: &Path) -> io::Result<()> {
    let mut command = if cfg!(target_os = "windows") {
        let mut command = Command::new("explorer");
        command.arg(format!("/select,{}", file.display()));
        command
    } else if cfg!(target_os = "macos") {
        let mut command = Command::new("open");
        command.arg("-R").arg(file);
        command
    } else {
        // xdg-open cannot select a file, so open the folder that holds it.
        let mut command = Command::new("xdg-open");
        command.arg(file.parent().unwrap_or(file));
        command
    };
    command.spawn().map(drop)
}

/// Opens a URI such as a system settings page.
#[doc = "# Errors\n\nReturns an error when the handler cannot be started."]
pub fn open_uri(uri: &str) -> io::Result<()> {
    let mut command = if cfg!(target_os = "windows") {
        let mut command = Command::new("explorer");
        command.arg(uri);
        command
    } else if cfg!(target_os = "macos") {
        let mut command = Command::new("open");
        command.arg(uri);
        command
    } else {
        let mut command = Command::new("xdg-open");
        command.arg(uri);
        command
    };
    command.spawn().map(drop)
}
