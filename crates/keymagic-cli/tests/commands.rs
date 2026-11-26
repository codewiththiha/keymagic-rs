//! End-to-end checks of the `keymagic` binary.

use std::error::Error;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

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
    let output = keymagic()
        .arg("--json")
        .arg("check")
        .arg(fixture("warn.kms"))
        .output()?;
    assert!(output.status.success(), "{}", text(&output.stderr));
    let shown = text(&output.stdout);
    assert!(shown.starts_with("[{\"severity\":\"warning\""), "{shown}");
    Ok(())
}

#[test]
fn check_json_reports_errors_and_fails() -> TestResult {
    let output = keymagic()
        .arg("--json")
        .arg("check")
        .arg(fixture("bad.kms"))
        .output()?;
    assert_eq!(output.status.code(), Some(1));
    let shown = text(&output.stdout);
    assert!(shown.contains("\"severity\":\"error\""), "{shown}");
    assert!(shown.contains("\"start\":5"), "{shown}");
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
fn color_flag_decides_escape_codes_without_a_terminal() -> TestResult {
    let cases = fixture("corpus/cases");
    for (mode, coloured) in [("always", true), ("never", false)] {
        let output = keymagic()
            .args(["--color", mode, "test"])
            .arg(&cases)
            .output()?;
        assert_eq!(
            text(&output.stdout).contains('\u{1b}'),
            coloured,
            "--color {mode}"
        );
    }
    let piped = keymagic().arg("test").arg(&cases).output()?;
    assert!(
        !text(&piped.stdout).contains('\u{1b}'),
        "auto is off when piped"
    );
    Ok(())
}

#[test]
fn test_command_counts_passes_and_fails() -> TestResult {
    let cases = fixture("corpus/cases");
    let output = keymagic().arg("--json").arg("test").arg(&cases).output()?;
    assert_eq!(output.status.code(), Some(1));
    let shown = text(&output.stdout);
    assert!(shown.starts_with("{\"passed\":1,\"failed\":1,"), "{shown}");
    assert!(
        shown.contains("\"name\":\"mismatch\",\"passed\":false"),
        "{shown}"
    );
    Ok(())
}

#[test]
fn doctor_is_still_a_stub() -> TestResult {
    let output = keymagic().arg("doctor").output()?;
    assert_eq!(output.status.code(), Some(2));
    assert!(text(&output.stderr).contains("not implemented yet"));
    Ok(())
}

#[test]
fn install_needs_the_user_flag() -> TestResult {
    let output = keymagic().arg("install").output()?;
    assert_eq!(output.status.code(), Some(2));
    assert!(text(&output.stderr).contains("--user"));
    Ok(())
}

#[test]
fn install_user_writes_the_ibus_component() -> TestResult {
    let data = std::env::temp_dir().join(format!("keymagic-install-{}", std::process::id()));
    let output = keymagic()
        .args(["install", "--user"])
        .env("XDG_DATA_HOME", &data)
        .output()?;
    if cfg!(target_os = "linux") {
        assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
        let xml = fs::read_to_string(data.join("ibus/component/keymagic.xml"))?;
        assert!(xml.contains("<name>org.freedesktop.IBus.KeyMagic</name>"));
    } else {
        assert_eq!(output.status.code(), Some(2));
        assert!(text(&output.stderr).contains("Linux only"));
    }
    let _ = fs::remove_dir_all(&data);
    Ok(())
}
