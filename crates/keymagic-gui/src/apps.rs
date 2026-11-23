//! Running applications, offered by the per-app mode pickers.

use sysinfo::{ProcessesToUpdate, System};

/// The names of the processes running now, sorted and without repeats.
#[must_use]
pub fn running() -> Vec<String> {
    // An unreadable process table leaves the picker empty, not the app broken.
    let Ok(mut system) = System::new() else {
        return Vec::new();
    };
    system.refresh_processes(ProcessesToUpdate::All, true);
    normalise(
        system
            .processes()
            .values()
            .map(|process| process.name().to_string_lossy().into_owned()),
    )
}

/// Sorts `names` case-insensitively, drops empty and repeated names.
#[must_use]
pub fn normalise(names: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut names: Vec<String> = names.into_iter().filter(|name| !name.is_empty()).collect();
    names.sort_by_key(|name| name.to_lowercase());
    names.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
    names
}

/// The running names not already in `listed`, compared without case.
#[must_use]
pub fn choices(running: &[String], listed: &[String]) -> Vec<String> {
    running
        .iter()
        .filter(|name| !listed.iter().any(|app| app.eq_ignore_ascii_case(name)))
        .cloned()
        .collect()
}
