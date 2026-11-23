//! Settings in the TOML layout `KeyMagic` 3 writes, so its file loads as-is.

use crate::invalid_data;
use keymagic_types::Hotkey;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, Write};
use std::path::Path;

/// Settings read from and written to `config.toml`. Unknown keys are ignored.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub general: General,
    pub keyboards: Keyboards,
    pub composition_mode: HostList,
    pub direct_mode: HostList,
    pub hotkeys: Hotkeys,
}

/// The `[hotkeys]` table: the global switch and the next-keyboard combination.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Hotkeys {
    pub enabled: bool,
    pub next_keyboard: Option<String>,
}

impl Default for Hotkeys {
    fn default() -> Self {
        Self {
            enabled: true,
            next_keyboard: None,
        }
    }
}

/// The `[general]` table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct General {
    pub start_with_system: bool,
    pub check_for_updates: bool,
    pub last_update_check: Option<String>,
    pub last_scanned_version: Option<String>,
    pub update_remind_after: Option<String>,
    pub enabled_languages: Vec<String>,
    pub update_channel: String,
    /// Turns off interface animation.
    pub reduce_motion: bool,
    /// Shows the keyboard name when the active keyboard changes.
    pub show_hud: bool,
}

impl Default for General {
    fn default() -> Self {
        Self {
            start_with_system: false,
            check_for_updates: true,
            last_update_check: None,
            last_scanned_version: None,
            update_remind_after: None,
            enabled_languages: Vec::new(),
            update_channel: "stable".to_owned(),
            reduce_motion: false,
            show_hud: true,
        }
    }
}

/// The `[keyboards]` table: active id, recent ids, installed list.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Keyboards {
    pub active: Option<String>,
    pub last_used: Vec<String>,
    pub installed: Vec<Installed>,
}

/// One `[[keyboards.installed]]` entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Installed {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub filename: String,
    /// Name shown in place of the keyboard's own name, when set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// `None` and `Some("")` both mean no switch hotkey.
    pub hotkey: Option<String>,
    pub hash: Option<String>,
    #[serde(default = "enabled_by_default")]
    pub enabled: bool,
}

/// A list of application names, as used by the per-app mode tables.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct HostList {
    pub enabled_hosts: Vec<String>,
}

const fn enabled_by_default() -> bool {
    true
}

impl Config {
    /// Enabled keyboard id pairs whose switch hotkeys are equal.
    #[must_use]
    pub fn hotkey_conflicts(&self) -> Vec<(&str, &str)> {
        let hotkeys: Vec<(&str, Hotkey)> = self
            .keyboards
            .installed
            .iter()
            .filter(|keyboard| keyboard.enabled)
            .filter_map(|keyboard| {
                let hotkey = keyboard.hotkey.as_deref()?.parse().ok()?;
                Some((keyboard.id.as_str(), hotkey))
            })
            .collect();
        let mut conflicts = Vec::new();
        for (index, (first, hotkey)) in hotkeys.iter().enumerate() {
            for (second, other) in &hotkeys[index + 1..] {
                if hotkey == other {
                    conflicts.push((*first, *second));
                }
            }
        }
        conflicts
    }

    /// Parses TOML text. Missing sections take their defaults.
    #[doc = "# Errors\n\nReturns the TOML error for malformed text or wrong value types."]
    pub fn from_toml(text: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(text)
    }

    /// Serialises the config as TOML.
    #[doc = "# Errors\n\nReturns the TOML error when a value cannot be represented."]
    pub fn to_toml(&self) -> Result<String, toml::ser::Error> {
        toml::to_string_pretty(self)
    }

    /// Reads `path`, or returns the defaults when the file does not exist.
    #[doc = "# Errors\n\nReturns an error for read failures or malformed TOML."]
    pub fn load_or_default(path: &Path) -> io::Result<Self> {
        match fs::read_to_string(path) {
            Ok(text) => Self::from_toml(&text).map_err(invalid_data),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Self::default()),
            Err(error) => Err(error),
        }
    }

    /// Writes through a temporary sibling file, creating parent folders.
    #[doc = "# Errors\n\nReturns an error when the folder cannot be created or the write fails."]
    pub fn save(&self, path: &Path) -> io::Result<()> {
        let text = self.to_toml().map_err(invalid_data)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let temporary = path.with_extension("toml.tmp");
        let mut file = fs::File::create(&temporary)?;
        file.write_all(text.as_bytes())?;
        file.sync_all()?;
        fs::rename(&temporary, path)
    }
}

impl Keyboards {
    /// Makes `id` active when installed and enabled.
    pub fn set_active(&mut self, id: &str) -> bool {
        let usable = self
            .installed
            .iter()
            .any(|keyboard| keyboard.id == id && keyboard.enabled);
        if usable {
            self.active = Some(id.to_owned());
        }
        usable
    }

    /// Enables or disables `id`. Disabling the active one moves to the next.
    pub fn set_enabled(&mut self, id: &str, enabled: bool) -> bool {
        let Some(keyboard) = self.installed.iter_mut().find(|keyboard| keyboard.id == id) else {
            return false;
        };
        keyboard.enabled = enabled;
        if !enabled && self.active.as_deref() == Some(id) {
            self.active = self
                .installed
                .iter()
                .find(|keyboard| keyboard.enabled)
                .map(|keyboard| keyboard.id.clone());
        }
        true
    }

    /// Sets the switch hotkey of `id`. Blank or `None` removes it.
    pub fn set_hotkey(&mut self, id: &str, hotkey: Option<String>) -> bool {
        let Some(keyboard) = self.installed.iter_mut().find(|keyboard| keyboard.id == id) else {
            return false;
        };
        keyboard.hotkey = hotkey.filter(|text| !text.trim().is_empty());
        true
    }

    /// Sets the display name of `id`. Blank or `None` resets it.
    pub fn rename(&mut self, id: &str, label: Option<String>) -> bool {
        let Some(keyboard) = self.installed.iter_mut().find(|keyboard| keyboard.id == id) else {
            return false;
        };
        keyboard.label = label
            .map(|text| text.trim().to_owned())
            .filter(|text| !text.is_empty());
        true
    }

    /// Moves `id` to `index`, clamped to the end.
    pub fn move_to(&mut self, id: &str, index: usize) -> bool {
        let Some(from) = self.installed.iter().position(|keyboard| keyboard.id == id) else {
            return false;
        };
        let keyboard = self.installed.remove(from);
        self.installed
            .insert(index.min(self.installed.len()), keyboard);
        true
    }
}
