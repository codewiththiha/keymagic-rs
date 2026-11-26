//! Per-user folders. Platforms differ only in the app folder name.

use directories::BaseDirs;
use std::path::{Path, PathBuf};

#[cfg(target_os = "linux")]
const APP_DIR: &str = "keymagic";
#[cfg(not(target_os = "linux"))]
const APP_DIR: &str = "KeyMagic";

/// Config, keyboard and cache folders for one user.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Paths {
    config: PathBuf,
    cache: PathBuf,
}

impl Paths {
    /// Places the app folder under the given roots without touching files.
    #[must_use]
    pub fn under(config_root: &Path, cache_root: &Path) -> Self {
        Self {
            config: config_root.join(APP_DIR),
            cache: cache_root.join(APP_DIR),
        }
    }

    /// Uses the current user's standard config and cache folders.
    #[must_use]
    pub fn for_user() -> Option<Self> {
        let dirs = BaseDirs::new()?;
        Some(Self::under(dirs.config_dir(), dirs.cache_dir()))
    }

    /// `config.toml` inside the app folder.
    #[must_use]
    pub fn config_file(&self) -> PathBuf {
        self.config.join("config.toml")
    }

    /// Folder holding the `.km2` and `.kms` files.
    #[must_use]
    pub fn keyboards_dir(&self) -> PathBuf {
        self.config.join("keyboards")
    }

    /// Folder holding PNG icons converted from keyboard files.
    #[must_use]
    pub fn icon_cache_dir(&self) -> PathBuf {
        self.cache.join("icons")
    }

    /// Unix socket path for IME messages.
    #[must_use]
    pub fn socket_path(&self) -> PathBuf {
        self.cache.join("keymagic.sock")
    }

    /// Unix socket path for keyboard-switch notices from IME components.
    #[must_use]
    pub fn hud_socket_path(&self) -> PathBuf {
        self.cache.join("keymagic-hud.sock")
    }
}
