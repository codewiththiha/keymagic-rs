//! Keyboard files on disk: scan, import, remove and the icon cache.

use crate::config::{Config, Installed};
use crate::invalid_data;
use crate::paths::Paths;
use keymagic_kms::{Diagnostic, Keyboard, Resolver, check, compile};
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};

const PNG_SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";

/// A `.km2` file found by [`KeyboardStore::scan`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Found {
    /// File stem, which is also the config id.
    pub id: String,
    /// Name from the KM2 info block, or the id when there is none.
    pub name: String,
    pub km2: PathBuf,
    /// Cached PNG icon, when the keyboard has a PNG or BMP icon.
    pub icon: Option<PathBuf>,
}

/// Keyboard folder and icon cache of one user.
#[derive(Debug, Clone)]
pub struct KeyboardStore {
    dir: PathBuf,
    icons: PathBuf,
}

impl KeyboardStore {
    /// Uses the keyboard and icon folders from `paths`.
    #[must_use]
    pub fn new(paths: &Paths) -> Self {
        Self {
            dir: paths.keyboards_dir(),
            icons: paths.icon_cache_dir(),
        }
    }

    /// Watches the keyboard folder. Keep the watcher alive to get events.
    #[doc = "# Errors\n\nReturns an error when the folder cannot be created or watched."]
    pub fn watch(&self) -> io::Result<(RecommendedWatcher, Receiver<notify::Result<Event>>)> {
        fs::create_dir_all(&self.dir)?;
        let (tx, rx) = mpsc::channel();
        let mut watcher = notify::recommended_watcher(tx).map_err(io::Error::other)?;
        watcher
            .watch(&self.dir, RecursiveMode::NonRecursive)
            .map_err(io::Error::other)?;
        Ok((watcher, rx))
    }

    /// Lists `.km2` files, caches icons and reconciles `config`.
    #[doc = "# Errors\n\nReturns an error for unreadable folders or bad KM2 files."]
    pub fn scan(&self, config: &mut Config) -> io::Result<Vec<Found>> {
        fs::create_dir_all(&self.dir)?;
        let mut found = Vec::new();
        for entry in fs::read_dir(&self.dir)? {
            let km2 = entry?.path();
            if km2.extension().and_then(OsStr::to_str) != Some("km2") {
                continue;
            }
            let Some(id) = km2.file_stem().and_then(OsStr::to_str).map(str::to_owned) else {
                continue;
            };
            let info = keymagic_km2::read_info_only(&fs::read(&km2)?).map_err(invalid_data)?;
            let name = info.name().unwrap_or(id.as_str()).to_owned();
            let icon = self.cache_icon(&id, info.icon())?;
            found.push(Found {
                id,
                name,
                km2,
                icon,
            });
        }
        found.sort_by(|a, b| a.id.cmp(&b.id));
        reconcile(config, &found);
        Ok(found)
    }

    /// Stores KM2 bytes as `<id>.km2` and installs them.
    #[doc = "# Errors\n\nReturns an error for invalid KM2 bytes or a failed write."]
    pub fn import_km2(&self, bytes: &[u8], config: &mut Config) -> io::Result<String> {
        let keyboard = keymagic_km2::parse(bytes).map_err(invalid_data)?;
        let name = keyboard.info.name().unwrap_or("keyboard").to_owned();
        let id = self.unique_id(&name, config);
        self.write_file(&format!("{id}.km2"), bytes)?;
        config.keyboards.installed.push(entry(&id, &name));
        Ok(id)
    }

    /// Compiles a KMS file and stores its source and `<id>.km2`.
    #[doc = "# Errors\n\nReturns the first diagnostic, or an I/O error."]
    pub fn import_kms(&self, source: &Path, config: &mut Config) -> io::Result<String> {
        let text = fs::read_to_string(source)?;
        let folder = source.parent().unwrap_or_else(|| Path::new("."));
        let keyboard = compile(&text, &FolderResolver(folder)).map_err(|diagnostics| {
            let first = diagnostics
                .first()
                .map_or("invalid KMS", |d| d.message.as_str());
            invalid_data(first)
        })?;
        let bytes = keymagic_km2::write(&keyboard).map_err(invalid_data)?;
        let name = keyboard.info.name().unwrap_or("keyboard").to_owned();
        let id = self.unique_id(&name, config);
        self.write_file(&format!("{id}.kms"), text.as_bytes())?;
        self.write_file(&format!("{id}.km2"), &bytes)?;
        config.keyboards.installed.push(entry(&id, &name));
        Ok(id)
    }

    /// Deletes both files of keyboard `id` and drops its entry.
    #[doc = "# Errors\n\nReturns an error when a file cannot be deleted."]
    pub fn remove(&self, id: &str, config: &mut Config) -> io::Result<bool> {
        for extension in ["km2", "kms"] {
            fs::remove_file(self.dir.join(format!("{id}.{extension}"))).or_else(ignore_missing)?;
        }
        let keyboards = &mut config.keyboards;
        let before = keyboards.installed.len();
        keyboards.installed.retain(|keyboard| keyboard.id != id);
        keyboards.last_used.retain(|used| used != id);
        if keyboards.active.as_deref() == Some(id) {
            keyboards.active = None;
        }
        Ok(keyboards.installed.len() != before)
    }

    /// Appends a tester case to `<id>.cases.toml` and returns the file path.
    #[doc = "# Errors\n\nReturns an error when the case file cannot be read, parsed or written."]
    pub fn record_case(&self, id: &str, input: &str, expected: &str) -> io::Result<PathBuf> {
        let name = format!("{id}.cases.toml");
        let mut file = match fs::read_to_string(self.dir.join(&name)) {
            Ok(text) => toml::from_str::<CaseFile>(&text).map_err(text_invalid)?,
            Err(error) if error.kind() == io::ErrorKind::NotFound => CaseFile::default(),
            Err(error) => return Err(error),
        };
        file.cases.push(Case {
            input: input.to_owned(),
            expected: expected.to_owned(),
        });
        let text = toml::to_string(&file).map_err(text_invalid)?;
        fs::create_dir_all(&self.dir)?;
        self.write_file(&name, text.as_bytes())?;
        Ok(self.dir.join(name))
    }

    /// Reads the compiled keyboard `<id>.km2`.
    #[doc = "# Errors\n\nReturns an error when the file is missing or is not KM2."]
    pub fn load(&self, id: &str) -> io::Result<Keyboard> {
        let bytes = fs::read(self.dir.join(format!("{id}.km2")))?;
        keymagic_km2::parse(&bytes).map_err(invalid_data)
    }

    /// Reads the KMS source of `id`, or `None` for KM2-only imports.
    #[doc = "# Errors\n\nReturns an error when the source file exists but cannot be read."]
    pub fn source(&self, id: &str) -> io::Result<Option<String>> {
        match fs::read_to_string(self.dir.join(format!("{id}.kms"))) {
            Ok(text) => Ok(Some(text)),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error),
        }
    }

    /// Reports every diagnostic for KMS `source`, resolving from the folder.
    #[must_use]
    pub fn check(&self, source: &str) -> Vec<Diagnostic> {
        check(source, &FolderResolver(&self.dir))
    }

    /// Compiles `source` in memory; nothing is written.
    #[doc = "# Errors\n\nReturns the error diagnostics when `source` does not compile."]
    pub fn compile(&self, source: &str) -> Result<Keyboard, Vec<Diagnostic>> {
        compile(source, &FolderResolver(&self.dir))
    }

    /// Formats `source` into canonical KMS text.
    #[doc = "# Errors\n\nReturns the error diagnostics when `source` does not parse."]
    pub fn format(&self, source: &str) -> Result<String, Vec<Diagnostic>> {
        keymagic_kms::format(source)
    }

    /// Writes only `<id>.kms`, leaving the compiled `<id>.km2` unchanged.
    #[doc = "# Errors\n\nReturns an error when the file cannot be written."]
    pub fn write_source(&self, id: &str, source: &str) -> io::Result<()> {
        self.write_file(&format!("{id}.kms"), source.as_bytes())
    }

    /// Stores `source` as `<id>.kms` and `<id>.km2` if it compiles.
    #[doc = "# Errors\n\nReturns an error when the compiled files cannot be written."]
    pub fn save_source(&self, id: &str, source: &str) -> io::Result<Vec<Diagnostic>> {
        let keyboard = match self.compile(source) {
            Ok(keyboard) => keyboard,
            Err(diagnostics) => return Ok(diagnostics),
        };
        let bytes = keymagic_km2::write(&keyboard).map_err(invalid_data)?;
        self.write_file(&format!("{id}.kms"), source.as_bytes())?;
        self.write_file(&format!("{id}.km2"), &bytes)?;
        Ok(Vec::new())
    }

    fn write_file(&self, name: &str, bytes: &[u8]) -> io::Result<()> {
        fs::create_dir_all(&self.dir)?;
        fs::write(self.dir.join(name), bytes)
    }

    fn unique_id(&self, name: &str, config: &Config) -> String {
        let base = slug(name);
        let mut id = base.clone();
        let mut suffix = 2;
        while self.is_taken(&id, config) {
            id = format!("{base}-{suffix}");
            suffix += 1;
        }
        id
    }

    fn is_taken(&self, id: &str, config: &Config) -> bool {
        config
            .keyboards
            .installed
            .iter()
            .any(|keyboard| keyboard.id == id)
            || self.dir.join(format!("{id}.km2")).exists()
            || self.dir.join(format!("{id}.kms")).exists()
    }

    fn cache_icon(&self, id: &str, icon: Option<&[u8]>) -> io::Result<Option<PathBuf>> {
        let Some(png) = icon.and_then(to_png) else {
            return Ok(None);
        };
        fs::create_dir_all(&self.icons)?;
        let path = self.icons.join(format!("{id}.png"));
        fs::write(&path, png)?;
        Ok(Some(path))
    }
}

/// PNG bytes as they are, BMP re-encoded as PNG, or `None`.
fn to_png(icon: &[u8]) -> Option<Vec<u8>> {
    if icon.starts_with(PNG_SIGNATURE) {
        return Some(icon.to_vec());
    }
    let image = image::load_from_memory_with_format(icon, image::ImageFormat::Bmp).ok()?;
    let mut png = io::Cursor::new(Vec::new());
    image.write_to(&mut png, image::ImageFormat::Png).ok()?;
    Some(png.into_inner())
}

/// Tester cases saved next to a keyboard, in `<id>.cases.toml`.
#[derive(Debug, Default, Serialize, Deserialize)]
struct CaseFile {
    #[serde(default, rename = "case")]
    cases: Vec<Case>,
}

/// One typed input and the text it must produce.
#[derive(Debug, Serialize, Deserialize)]
struct Case {
    input: String,
    expected: String,
}

/// Makes config entries match the files found on disk.
fn reconcile(config: &mut Config, found: &[Found]) {
    let installed = &mut config.keyboards.installed;
    installed.retain(|keyboard| found.iter().any(|item| item.id == keyboard.id));
    for item in found {
        if installed.iter().any(|keyboard| keyboard.id == item.id) {
            for keyboard in installed
                .iter_mut()
                .filter(|keyboard| keyboard.id == item.id)
            {
                keyboard.name.clone_from(&item.name);
            }
        } else {
            installed.push(entry(&item.id, &item.name));
        }
    }
}

fn text_invalid(error: impl std::fmt::Display) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, error.to_string())
}

fn ignore_missing(error: io::Error) -> io::Result<()> {
    if error.kind() == io::ErrorKind::NotFound {
        Ok(())
    } else {
        Err(error)
    }
}

fn entry(id: &str, name: &str) -> Installed {
    Installed {
        id: id.to_owned(),
        name: name.to_owned(),
        filename: format!("{id}.km2"),
        hotkey: None,
        hash: None,
        label: None,
        enabled: true,
    }
}

/// Lower-case ASCII id from a keyboard name, or `keyboard` when none remains.
fn slug(name: &str) -> String {
    let mapped: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    let trimmed = mapped.trim_matches('-');
    if trimmed.is_empty() {
        "keyboard".to_owned()
    } else {
        trimmed.to_owned()
    }
}

/// Reads KMS includes and icons from the source's folder.
struct FolderResolver<'a>(&'a Path);

impl Resolver for FolderResolver<'_> {
    fn read(&self, path: &str) -> Option<Vec<u8>> {
        fs::read(self.0.join(path)).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::{PNG_SIGNATURE, to_png};

    /// A 1x1 red 24-bit BMP.
    const BMP_1X1: &[u8] = &[
        b'B', b'M', 58, 0, 0, 0, 0, 0, 0, 0, 54, 0, 0, 0, 40, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 1,
        0, 24, 0, 0, 0, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        255, 0,
    ];

    #[test]
    fn bmp_icon_becomes_png() {
        assert!(to_png(BMP_1X1).is_some_and(|png| png.starts_with(PNG_SIGNATURE)));
    }

    #[test]
    fn unknown_icon_bytes_are_dropped() {
        assert_eq!(to_png(b"not an image"), None);
    }
}
