//! Our compiled output against the stored KM2 files for the shipped Karen
//! keyboards that match them byte for byte. The other stored files (`myWin` and
//! `comprehensive_test`) differ from the pinned compiler's output and are
//! recorded in `progress.md`; `every_corpus_script_matches_pinned_compiler`
//! remains the reference for those.

use keymagic_kms::{Resolver, compile};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

type TestResult = Result<(), Box<dyn Error>>;

const SHIPPED: [&str; 3] = ["Eastern-Pwo_kjp", "Sgaw-Kawthoolei_ksw", "Western-Pwo_pwo"];

struct Dir(PathBuf);

impl Resolver for Dir {
    fn read(&self, path: &str) -> Option<Vec<u8>> {
        fs::read(self.0.join(path)).ok()
    }
}

#[test]
fn shipped_karen_keyboards_match_stored_km2() -> TestResult {
    let corpus = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpus");
    let dir = corpus.join("kms/alternatives");
    let mut mismatches = Vec::new();
    for name in SHIPPED {
        let src = fs::read_to_string(dir.join(format!("{name}.kms")))?;
        let ours = compile(&src, &Dir(dir.clone())).map_err(|errs| format!("{name}: {errs:?}"))?;
        let bytes = keymagic_km2::write(&ours).map_err(|error| format!("{name}: {error}"))?;
        let stored = fs::read(corpus.join("km2/alternatives").join(format!("{name}.km2")))?;
        if bytes != stored {
            let first = bytes.iter().zip(&stored).position(|(a, b)| a != b);
            mismatches.push(format!(
                "{name}: {} bytes ours, {} stored, first difference at byte {first:?}",
                bytes.len(),
                stored.len()
            ));
        }
    }
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
    Ok(())
}
