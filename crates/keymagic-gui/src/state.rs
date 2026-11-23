//! Shared state: settings, the keyboard list, file actions and the watcher.

use keymagic_host::store::Found;
use keymagic_host::{Config, KeyboardStore, Paths};
use keymagic_kms::{Diagnostic, Keyboard};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

/// Settings and installed keyboards, read from `keymagic-host`.
pub struct Host {
    store: KeyboardStore,
    config: Config,
    config_path: PathBuf,
    keyboards: Vec<Found>,
}

impl Host {
    /// Loads `config.toml` (or defaults) and lists the keyboards folder.
    #[doc = "# Errors\n\nReturns an error when the config or keyboard folder cannot be read."]
    pub fn open(paths: &Paths) -> io::Result<Self> {
        let config_path = paths.config_file();
        let mut host = Self {
            store: KeyboardStore::new(paths),
            config: Config::load_or_default(&config_path)?,
            config_path,
            keyboards: Vec::new(),
        };
        host.rescan()?;
        Ok(host)
    }

    /// Lists keyboards again after the folder changed.
    #[doc = "# Errors\n\nReturns an error when the keyboard folder cannot be read."]
    pub fn rescan(&mut self) -> io::Result<()> {
        self.keyboards = self.store.scan(&mut self.config)?;
        Ok(())
    }

    /// The keyboards found by the last scan.
    #[must_use]
    pub fn keyboards(&self) -> &[Found] {
        &self.keyboards
    }

    /// The settings as last loaded.
    #[must_use]
    pub const fn config(&self) -> &Config {
        &self.config
    }

    /// The settings, for editing before [`Host::save`].
    #[must_use]
    pub const fn config_mut(&mut self) -> &mut Config {
        &mut self.config
    }

    /// Writes `config.toml`, creating its folder when needed.
    #[doc = "# Errors\n\nReturns an error when the file cannot be written."]
    pub fn save(&self) -> io::Result<()> {
        if let Some(folder) = self.config_path.parent() {
            fs::create_dir_all(folder)?;
        }
        self.config.save(&self.config_path)
    }

    /// Installs a `.km2` or `.kms` file and rescans the folder.
    #[doc = "# Errors\n\nReturns an error for other extensions, bad files or failed writes."]
    pub fn import(&mut self, path: &Path) -> io::Result<String> {
        let id = match path.extension().and_then(|ext| ext.to_str()) {
            Some("km2") => self.store.import_km2(&fs::read(path)?, &mut self.config)?,
            Some("kms") => self.store.import_kms(path, &mut self.config)?,
            _ => {
                return Err(io::Error::other(
                    "only .km2 and .kms files can be installed",
                ));
            }
        };
        self.rescan()?;
        Ok(id)
    }

    /// Deletes keyboard `id` and rescans the folder.
    #[doc = "# Errors\n\nReturns an error when its files cannot be deleted."]
    pub fn remove(&mut self, id: &str) -> io::Result<bool> {
        let removed = self.store.remove(id, &mut self.config)?;
        self.rescan()?;
        Ok(removed)
    }

    /// Makes `id` the active keyboard. Returns `false` when not installed.
    #[must_use]
    pub fn use_keyboard(&mut self, id: &str) -> bool {
        self.config.keyboards.set_active(id)
    }

    /// Turns keyboard `id` on or off. Returns `false` when it is not installed.
    #[must_use]
    pub fn set_enabled(&mut self, id: &str, enabled: bool) -> bool {
        self.config.keyboards.set_enabled(id, enabled)
    }

    /// Reads the compiled keyboard `id`.
    #[doc = "# Errors\n\nReturns an error when the keyboard file is missing or invalid."]
    pub fn load_keyboard(&self, id: &str) -> io::Result<Keyboard> {
        self.store.load(id)
    }

    /// The KMS source of `id`, or `None` for keyboards imported as KM2 only.
    #[doc = "# Errors\n\nReturns an error when the source file cannot be read."]
    pub fn source(&self, id: &str) -> io::Result<Option<String>> {
        self.store.source(id)
    }

    /// Every diagnostic for KMS `source`.
    #[must_use]
    pub fn check(&self, source: &str) -> Vec<Diagnostic> {
        self.store.check(source)
    }

    /// Compiles `source` without writing anything.
    #[doc = "# Errors\n\nReturns the error diagnostics when `source` does not compile."]
    pub fn compile(&self, source: &str) -> Result<Keyboard, Vec<Diagnostic>> {
        self.store.compile(source)
    }

    /// Formats `source` into canonical KMS text.
    #[doc = "# Errors\n\nReturns the error diagnostics when `source` does not parse."]
    pub fn format(&self, source: &str) -> Result<String, Vec<Diagnostic>> {
        self.store.format(source)
    }

    /// Writes the KMS source of keyboard `id` without compiling it.
    #[doc = "# Errors\n\nReturns an error when the file cannot be written."]
    pub fn write_source(&self, id: &str, source: &str) -> io::Result<()> {
        self.store.write_source(id, source)
    }

    /// Stores `source` as keyboard `id` if it compiles; else returns errors.
    #[doc = "# Errors\n\nReturns an error when the compiled files cannot be written."]
    pub fn save_source(&self, id: &str, source: &str) -> io::Result<Vec<Diagnostic>> {
        self.store.save_source(id, source)
    }
}

/// Locks the host, recovering from a poisoned lock so the UI keeps running.
pub fn lock(host: &Mutex<Host>) -> MutexGuard<'_, Host> {
    host.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Rescans the keyboard folder on every change and then calls `on_change`.
#[doc = "# Errors\n\nReturns an error when the keyboard folder cannot be watched."]
pub fn watch_keyboards(
    host: &Arc<Mutex<Host>>,
    on_change: impl Fn() + Send + 'static,
) -> io::Result<()> {
    let (watcher, events) = {
        let guard = lock(host);
        guard.store.watch()?
    };
    let host = Arc::clone(host);
    std::thread::spawn(move || {
        let _watcher = watcher;
        for event in events {
            if event.is_ok() && lock(&host).rescan().is_ok() {
                on_change();
            }
        }
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{Host, lock};
    use keymagic_host::Paths;
    use std::error::Error;
    use std::fs;
    use std::sync::{Arc, Mutex};

    const MYWIN: &[u8] = include_bytes!("../../../corpus/km2/alternatives/myWin.km2");

    fn paths_in(dir: &tempfile::TempDir) -> Paths {
        Paths::under(&dir.path().join("config"), &dir.path().join("cache"))
    }

    #[test]
    fn empty_folder_lists_no_keyboards() -> Result<(), Box<dyn Error>> {
        let dir = tempfile::tempdir()?;
        let host = Host::open(&paths_in(&dir))?;
        assert!(host.keyboards().is_empty());
        Ok(())
    }

    #[test]
    fn rescan_finds_a_keyboard_copied_into_the_folder() -> Result<(), Box<dyn Error>> {
        let dir = tempfile::tempdir()?;
        let paths = paths_in(&dir);
        let mut host = Host::open(&paths)?;
        let folder = paths.keyboards_dir();
        fs::create_dir_all(&folder)?;
        fs::write(folder.join("myWin.km2"), MYWIN)?;
        host.rescan()?;
        let found: Vec<_> = host
            .keyboards()
            .iter()
            .map(|found| (found.id.as_str(), found.name.as_str()))
            .collect();
        assert_eq!(found, [("myWin", "my-Win 2.3.0 Unicode 5.2")]);
        Ok(())
    }

    #[test]
    fn import_then_use_then_save_round_trips_settings() -> Result<(), Box<dyn Error>> {
        let dir = tempfile::tempdir()?;
        let paths = paths_in(&dir);
        let mut host = Host::open(&paths)?;
        let source = dir.path().join("myWin.km2");
        fs::write(&source, MYWIN)?;
        let id = host.import(&source)?;
        assert!(host.use_keyboard(&id));
        host.config_mut().general.check_for_updates = false;
        host.save()?;
        let reopened = Host::open(&paths)?;
        assert_eq!(
            reopened.config().keyboards.active.as_deref(),
            Some(id.as_str())
        );
        assert!(!reopened.config().general.check_for_updates);
        Ok(())
    }

    #[test]
    fn import_rejects_other_extensions() -> Result<(), Box<dyn Error>> {
        let dir = tempfile::tempdir()?;
        let mut host = Host::open(&paths_in(&dir))?;
        let source = dir.path().join("notes.txt");
        fs::write(&source, "hello")?;
        assert!(host.import(&source).is_err());
        Ok(())
    }

    #[test]
    fn lock_recovers_after_a_panic_in_another_thread() -> Result<(), Box<dyn Error>> {
        let dir = tempfile::tempdir()?;
        let host = Arc::new(Mutex::new(Host::open(&paths_in(&dir))?));
        let shared = Arc::clone(&host);
        let outcome = std::thread::spawn(move || {
            let _guard = shared.lock();
            panic!("poison the lock");
        })
        .join();
        assert!(outcome.is_err());
        assert!(lock(&host).keyboards().is_empty());
        Ok(())
    }
}
