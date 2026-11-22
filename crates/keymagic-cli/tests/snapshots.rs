//! Compares command output with the files in `tests/snapshots`.
//!
//! Run with `KEYMAGIC_UPDATE_SNAPSHOTS=1` to rewrite a snapshot after review.

use std::env;
use std::error::Error;
use std::fs;
use std::path::Path;
use std::process::Command;

type TestResult = Result<(), Box<dyn Error>>;

fn run(args: &[&str]) -> Result<String, Box<dyn Error>> {
    let output = Command::new(env!("CARGO_BIN_EXE_keymagic"))
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .args(args)
        .output()?;
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn assert_snapshot(name: &str, actual: &str) -> TestResult {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/snapshots")
        .join(name);
    if env::var_os("KEYMAGIC_UPDATE_SNAPSHOTS").is_some() {
        fs::write(&path, actual)?;
    }
    let expected = fs::read_to_string(&path)?;
    assert_eq!(actual, expected, "snapshot {name} differs");
    Ok(())
}

#[test]
fn check_output_matches_snapshot() -> TestResult {
    let actual = run(&["check", "tests/fixtures/good.kms"])?;
    assert_snapshot("check.stdout", &actual)
}

#[test]
fn fmt_output_matches_snapshot() -> TestResult {
    let actual = run(&["fmt", "tests/fixtures/good.kms"])?;
    assert_snapshot("fmt.stdout", &actual)
}

#[test]
fn test_table_matches_snapshot() -> TestResult {
    let actual = run(&["test", "tests/fixtures/corpus/cases"])?;
    assert_snapshot("test.stdout", &actual)
}

#[test]
fn version_matches_snapshot() -> TestResult {
    let actual = run(&["--version"])?;
    assert_snapshot("version.stdout", &actual)
}
