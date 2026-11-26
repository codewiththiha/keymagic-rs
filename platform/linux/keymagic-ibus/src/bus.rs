//! Finds the private bus address of the running `IBus` daemon.

use std::fs;
use std::path::PathBuf;

/// Address from `IBUS_ADDRESS`, else from the first bus file that has one.
pub fn address() -> Result<String, String> {
    if let Some(value) = std::env::var("IBUS_ADDRESS")
        .ok()
        .filter(|value| !value.is_empty())
    {
        return Ok(value);
    }
    let dir = bus_dir().ok_or("no config folder for this account")?;
    let mut files = Vec::new();
    let entries =
        fs::read_dir(&dir).map_err(|error| format!("cannot read {}: {error}", dir.display()))?;
    for entry in entries.flatten() {
        files.push(entry.path());
    }
    files.sort();
    files
        .iter()
        .filter_map(|file| fs::read_to_string(file).ok())
        .find_map(|text| parse(&text))
        .ok_or_else(|| "IBus is not running; start ibus-daemon first".to_owned())
}

/// Reads the `IBUS_ADDRESS=` line from a bus file.
#[must_use]
pub fn parse(text: &str) -> Option<String> {
    text.lines()
        .find_map(|line| line.strip_prefix("IBUS_ADDRESS="))
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

fn bus_dir() -> Option<PathBuf> {
    let config = std::env::var_os("XDG_CONFIG_HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))?;
    Some(config.join("ibus/bus"))
}

#[cfg(test)]
mod tests {
    use super::parse;

    #[test]
    fn reads_the_address_line() {
        let text = "# comment\nIBUS_ADDRESS=unix:path=/run/ibus,guid=1\nIBUS_PID=7\n";
        assert_eq!(parse(text).as_deref(), Some("unix:path=/run/ibus,guid=1"));
    }

    #[test]
    fn empty_address_is_absent() {
        assert_eq!(parse("IBUS_ADDRESS=\n"), None);
    }
}
