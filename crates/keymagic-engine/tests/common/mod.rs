//! Corpus fixtures and helpers shared by the replay and differential tests.

use keymagic_engine::Action;
use std::env;
use std::error::Error;
use std::path::{Path, PathBuf};

/// Returns the repository corpus directory.
pub fn corpus_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("corpus")
}

/// Returns the KM2 path for a corpus keyboard.
pub fn keyboard_path(relative: &str) -> Result<PathBuf, Box<dyn Error>> {
    if Path::new(relative)
        .extension()
        .is_some_and(|ext| ext == "km2")
    {
        return Ok(corpus_dir().join(relative));
    }
    let reference = PathBuf::from(env::var("KEYMAGIC_REFERENCE_KM2_DIRECTORY")?);
    let stem = relative.strip_prefix("kms/").unwrap_or(relative);
    Ok(reference.join(stem).with_extension("km2"))
}

/// Returns the (action, delete count, inserted text) reported by the driver.
pub fn action_fields(action: &Action) -> (&'static str, usize, String) {
    match action {
        Action::None => ("none", 0, String::new()),
        Action::Insert(text) => ("insert", 0, text.clone()),
        Action::Delete(count) => ("delete", *count, String::new()),
        Action::Replace { delete, insert } => ("replace", *delete, insert.clone()),
    }
}
