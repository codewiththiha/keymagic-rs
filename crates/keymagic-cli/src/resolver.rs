//! Reads included files and icons relative to the root source's folder.

use keymagic_kms::Resolver;
use std::fs;
use std::path::{Path, PathBuf};

/// Resolves relative paths against one directory.
pub struct DirResolver(PathBuf);

impl DirResolver {
    /// Resolves against the folder that holds `file`.
    pub fn beside(file: &Path) -> Self {
        match file.parent() {
            Some(dir) if !dir.as_os_str().is_empty() => Self(dir.to_path_buf()),
            _ => Self(PathBuf::from(".")),
        }
    }
}

impl Resolver for DirResolver {
    fn read(&self, path: &str) -> Option<Vec<u8>> {
        fs::read(self.0.join(path)).ok()
    }
}
