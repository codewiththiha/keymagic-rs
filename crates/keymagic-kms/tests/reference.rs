//! Byte identity with the KM2 files that the pinned upstream compiler writes.
//!
//! Reference KM2 files come from `tools/reference_driver.rs`.
//! reference-corpus.yml runs that driver out of process.

use keymagic_kms::{Resolver, compile};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

type TestResult = Result<(), Box<dyn Error>>;

const ALTERNATIVES: [&str; 3] = ["Eastern-Pwo_kjp", "Sgaw-Kawthoolei_ksw", "Western-Pwo_pwo"];

struct Dir(PathBuf);

impl Resolver for Dir {
    fn read(&self, path: &str) -> Option<Vec<u8>> {
        fs::read(self.0.join(path)).ok()
    }
}

fn corpus() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../corpus/kms")
}

fn sources() -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let mut found = Vec::new();
    for dir in ["cases", "upstream"] {
        for entry in fs::read_dir(corpus().join(dir))? {
            let path = entry?.path();
            if path.extension().is_some_and(|ext| ext == "kms") {
                found.push(path);
            }
        }
    }
    for name in ALTERNATIVES {
        found.push(corpus().join("alternatives").join(format!("{name}.kms")));
    }
    found.sort();
    Ok(found)
}

fn resolver_for(path: &Path) -> Dir {
    Dir(path.parent().map_or_else(PathBuf::new, Path::to_path_buf))
}

#[test]
#[ignore = "needs KEYMAGIC_REFERENCE_KM2_DIRECTORY from the pinned driver"]
fn every_corpus_script_matches_pinned_compiler() -> TestResult {
    let reference = PathBuf::from(std::env::var("KEYMAGIC_REFERENCE_KM2_DIRECTORY")?);
    let found = sources()?;
    assert_eq!(found.len(), 40);
    let mut failures = Vec::new();
    for source in found {
        let relative = source.strip_prefix(corpus())?.with_extension("km2");
        let src = fs::read_to_string(&source)?;
        let keyboard = match compile(&src, &resolver_for(&source)) {
            Ok(keyboard) => keyboard,
            Err(errors) => {
                failures.push(format!("{}: {errors:?}", relative.display()));
                continue;
            }
        };
        let bytes = keymagic_km2::write(&keyboard).map_err(|error| format!("{error:?}"))?;
        if bytes != fs::read(reference.join(&relative))? {
            failures.push(format!(
                "{} differs from the pinned KM2",
                relative.display()
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    Ok(())
}
