//! Corpus checks: lexing, token snapshots and formatter round trips.

mod common;

use keymagic_kms::{Diagnostic, Resolver, compile, format, lexer};
use std::error::Error;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

type TestResult = Result<(), Box<dyn Error>>;

struct Dir(PathBuf);

impl Resolver for Dir {
    fn read(&self, path: &str) -> Option<Vec<u8>> {
        fs::read(self.0.join(path)).ok()
    }
}

fn corpus() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../corpus/kms")
}

fn scripts() -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let mut found = Vec::new();
    for dir in ["alternatives", "cases", "upstream"] {
        for entry in fs::read_dir(corpus().join(dir))? {
            let path = entry?.path();
            if path.extension().is_some_and(|ext| ext == "kms") {
                found.push(path);
            }
        }
    }
    found.sort();
    Ok(found)
}

fn resolver_for(path: &Path) -> Dir {
    Dir(path.parent().map_or_else(PathBuf::new, Path::to_path_buf))
}

#[test]
fn every_corpus_script_lexes_without_error() -> TestResult {
    let found = scripts()?;
    assert_eq!(found.len(), 42);
    for path in found {
        let src = fs::read_to_string(&path)?;
        if let Err(error) = lexer::lex(&src) {
            panic!("{}: {}", path.display(), error.render(&src, "<input>"));
        }
    }
    Ok(())
}

#[test]
fn token_snapshots_match() -> TestResult {
    let snapshots = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/snapshots");
    let names = [
        "upstream/simple_test",
        "upstream/comprehensive_test",
        "alternatives/Eastern-Pwo_kjp",
        "cases/vk_sequence",
        "cases/positional_backrefs",
    ];
    let mut failures = Vec::new();
    for name in names {
        let src = fs::read_to_string(corpus().join(format!("{name}.kms")))?;
        let lexed = lexer::lex(&src).map_err(|error| error.render(&src, "<input>"))?;
        let mut dump = String::new();
        for token in &lexed.tokens {
            let _ = writeln!(
                dump,
                "{}..{} {:?}",
                token.span.start, token.span.end, token.kind
            );
        }
        let file = format!("{}.tokens", name.replace('/', "_"));
        common::compare(&snapshots, &file, &dump, &mut failures);
    }
    common::finish(&failures);
    Ok(())
}

#[test]
fn formatter_is_idempotent_and_keeps_the_keyboard() -> TestResult {
    for path in scripts()? {
        let src = fs::read_to_string(&path)?;
        let name = path.display();
        let once = format(&src).map_err(|errors| format!("{name}: {errors:?}"))?;
        let twice = format(&once).map_err(|errors| format!("{name}: {errors:?}"))?;
        assert_eq!(once, twice, "{name}");
        match (
            compile(&src, &resolver_for(&path)),
            compile(&once, &resolver_for(&path)),
        ) {
            (Ok(before), Ok(after)) => assert_eq!(before, after, "{name}"),
            (Err(before), Err(after)) => {
                let messages = |errors: &[Diagnostic]| {
                    errors.iter().map(|d| d.message.clone()).collect::<Vec<_>>()
                };
                assert_eq!(messages(&before), messages(&after), "{name}");
            }
            _ => panic!("{name}: formatting changed whether the script compiles"),
        }
    }
    Ok(())
}

#[test]
fn formatter_keeps_every_block_comment() -> TestResult {
    for path in scripts()? {
        let src = fs::read_to_string(&path)?;
        let name = path.display();
        let once = format(&src).map_err(|errors| format!("{name}: {errors:?}"))?;
        assert_eq!(
            src.matches("/*").count(),
            once.matches("/*").count(),
            "{name}"
        );
    }
    Ok(())
}
