//! Snapshot cases for the `keymagic` binary, in the style of trycmd. Each case
//! names its arguments, optional stdin, expected exit code and a transcript
//! snapshot holding the exit code, stdout and stderr. trycmd is not used: it
//! needs `humantime`, flagged by RUSTSEC-2026-0542, and `snapbox` needs
//! Apache-2.0-only crates. Set `KEYMAGIC_UPDATE_SNAPSHOTS=1` to update.

use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::process::{Command, Stdio};

struct Case {
    args: Vec<String>,
    stdin: &'static str,
    code: i32,
    snapshot: &'static str,
}

struct Outcome {
    code: Option<i32>,
    transcript: String,
}

fn case(args: &[&str], stdin: &'static str, code: i32, snapshot: &'static str) -> Case {
    Case {
        args: args.iter().map(|arg| (*arg).to_owned()).collect(),
        stdin,
        code,
        snapshot,
    }
}

fn cases() -> Vec<Case> {
    let out = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("good.km2")
        .to_string_lossy()
        .into_owned();
    vec![
        case(&["check", "tests/fixtures/good.kms"], "", 0, "check.txt"),
        case(&["fmt", "tests/fixtures/good.kms"], "", 0, "fmt.txt"),
        case(&["test", "tests/fixtures/corpus/cases"], "", 1, "test.txt"),
        case(&["--version"], "", 0, "version.txt"),
        case(
            &["compile", "tests/fixtures/good.kms", "-o", out.as_str()],
            "",
            0,
            "compile.txt",
        ),
        case(
            &["dump", "../../corpus/km2/alternatives/Eastern-Pwo_kjp.km2"],
            "",
            0,
            "dump.txt",
        ),
        case(
            &["repl", "tests/fixtures/good.kms"],
            "a\n:reset\n",
            0,
            "repl.txt",
        ),
        case(&["doctor"], "", 2, "doctor.txt"),
        case(&["install"], "", 2, "install.txt"),
    ]
}

fn run(case: &Case) -> io::Result<Outcome> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_keymagic"))
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .args(&case.args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(case.stdin.as_bytes())?;
    }
    let output = child.wait_with_output()?;
    let code = output.status.code();
    let transcript = format!(
        "exit: {}\n--- stdout ---\n{}--- stderr ---\n{}",
        code.map_or_else(|| "signal".to_owned(), |code| code.to_string()),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    )
    .replace(env!("CARGO_TARGET_TMPDIR"), "<tmp>")
    .replace("<tmp>\\", "<tmp>/")
    .replace("keymagic.exe", "keymagic");
    Ok(Outcome { code, transcript })
}

fn check_case(case: &Case, failures: &mut Vec<String>) -> io::Result<()> {
    let outcome = run(case)?;
    let name = case.args.join(" ");
    if outcome.code != Some(case.code) {
        failures.push(format!(
            "`keymagic {name}` exited {:?}, expected {}",
            outcome.code, case.code
        ));
    }
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/snapshots")
        .join(case.snapshot);
    if env::var_os("KEYMAGIC_UPDATE_SNAPSHOTS").is_some() {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, &outcome.transcript)?;
    }
    let expected = fs::read_to_string(&path).unwrap_or_default();
    if expected != outcome.transcript {
        failures.push(format!(
            "`keymagic {name}` differs from {}; actual: {:?}",
            case.snapshot, outcome.transcript
        ));
    }
    Ok(())
}

#[test]
fn cases_match_snapshots_and_exit_codes() -> io::Result<()> {
    let mut failures = Vec::new();
    for case in cases() {
        check_case(&case, &mut failures)?;
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    Ok(())
}
