//! Thirty broken scripts, each compared with its `.diag` snapshot.

mod common;

use keymagic_kms::{Resolver, check};
use std::error::Error;
use std::fs;
use std::path::PathBuf;

type TestResult = Result<(), Box<dyn Error>>;

struct NoFiles;

impl Resolver for NoFiles {
    fn read(&self, _path: &str) -> Option<Vec<u8>> {
        None
    }
}

#[test]
fn thirty_broken_scripts_have_snapshot_diagnostics() -> TestResult {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/broken");
    let mut scripts = Vec::new();
    for entry in fs::read_dir(&dir)? {
        let path = entry?.path();
        if path.extension().is_some_and(|ext| ext == "kms") {
            scripts.push(path);
        }
    }
    scripts.sort();
    assert_eq!(scripts.len(), 30);
    let mut failures = Vec::new();
    for path in &scripts {
        let src = fs::read_to_string(path)?;
        let rendered: String = check(&src, &NoFiles)
            .iter()
            .map(|diagnostic| diagnostic.render(&src))
            .collect();
        let stem = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or_default();
        common::compare(&dir, &format!("{stem}.diag"), &rendered, &mut failures);
    }
    common::finish(&failures);
    Ok(())
}
