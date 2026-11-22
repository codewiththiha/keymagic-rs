//! End-to-end checks of the `keymagic` binary.

use std::error::Error;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

type TestResult = Result<(), Box<dyn Error>>;

fn keymagic() -> Command {
    Command::new(env!("CARGO_BIN_EXE_keymagic"))
}

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn scratch(name: &str) -> Result<PathBuf, Box<dyn Error>> {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("keymagic-cli");
    fs::create_dir_all(&dir)?;
    Ok(dir.join(name))
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn run(args: &[&str]) -> std::io::Result<Output> {
    keymagic().args(args).output()
}

#[test]
fn compile_writes_km2_that_dump_reads() -> TestResult {
    let out = scratch("good.km2")?;
    let output = keymagic()
        .arg("compile")
        .arg(fixture("good.kms"))
        .arg("-o")
        .arg(&out)
        .output()?;
    assert!(output.status.success(), "{}", text(&output.stderr));
    assert!(text(&output.stdout).starts_with("wrote "));
    let dump = keymagic().arg("dump").arg(&out).output()?;
    assert!(dump.status.success(), "{}", text(&dump.stderr));
    let shown = text(&dump.stdout);
    assert!(shown.contains("name: Good"), "{shown}");
    assert!(shown.contains("rules: 1"), "{shown}");
    Ok(())
}

#[test]
fn compile_reports_errors_and_fails() -> TestResult {
    let output = keymagic().arg("compile").arg(fixture("bad.kms")).output()?;
    assert_eq!(output.status.code(), Some(1));
    assert!(text(&output.stderr).contains("undefined variable '$b'"));
    Ok(())
}

#[test]
fn check_json_reports_warnings_and_succeeds() -> TestResult {
    let output = run(&["--json", "check", &fixture("warn.kms").to_string_lossy()])?;
    assert!(output.status.success(), "{}", text(&output.stderr));
    let parsed: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    let items = parsed.as_array().ok_or("expected a JSON array")?;
    assert!(!items.is_empty());
    assert_eq!(items[0]["severity"], "warning");
    Ok(())
}

#[test]
fn check_json_reports_errors_and_fails() -> TestResult {
    let output = run(&["--json", "check", &fixture("bad.kms").to_string_lossy()])?;
    assert_eq!(output.status.code(), Some(1));
    let parsed: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(parsed[0]["severity"], "error");
    assert_eq!(parsed[0]["start"], 5);
    Ok(())
}

#[test]
fn fmt_check_fails_on_loose_source_and_write_fixes_it() -> TestResult {
    let path = scratch("loose.kms")?;
    fs::copy(fixture("loose.kms"), &path)?;
    let before = keymagic().args(["fmt", "--check"]).arg(&path).output()?;
    assert_eq!(before.status.code(), Some(1));
    let write = keymagic().args(["fmt", "--write"]).arg(&path).output()?;
    assert!(write.status.success(), "{}", text(&write.stderr));
    assert_eq!(fs::read_to_string(&path)?, "$v = \"a\"\n\"a\" => \"b\"\n");
    let after = keymagic().args(["fmt", "--check"]).arg(&path).output()?;
    assert!(after.status.success());
    Ok(())
}

#[test]
fn repl_types_characters_through_the_engine() -> TestResult {
    let mut child = keymagic()
        .arg("repl")
        .arg(fixture("good.kms"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(b"a\n")?;
    }
    let output = child.wait_with_output()?;
    assert!(output.status.success(), "{}", text(&output.stderr));
    assert!(text(&output.stdout).contains("composing: b"));
    Ok(())
}

#[test]
fn test_command_counts_passes_and_fails() -> TestResult {
    let cases = fixture("corpus/cases");
    let output = run(&["--json", "test", &cases.to_string_lossy()])?;
    assert_eq!(output.status.code(), Some(1));
    let parsed: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(parsed["passed"], 1);
    assert_eq!(parsed["failed"], 1);
    assert_eq!(parsed["cases"][1]["name"], "mismatch");
    Ok(())
}

#[test]
fn doctor_and_install_are_stubs() -> TestResult {
    for command in ["doctor", "install"] {
        let output = keymagic().arg(command).output()?;
        assert_eq!(output.status.code(), Some(2));
        assert!(text(&output.stderr).contains("not implemented yet"));
    }
    Ok(())
}
