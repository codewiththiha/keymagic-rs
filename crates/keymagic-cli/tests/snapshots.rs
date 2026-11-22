//! Snapshot cases for the `keymagic` binary, in the style of trycmd: each case
//! names its arguments, expected exit code and stdout snapshot. trycmd is not
//! used: it needs `humantime`, flagged by RUSTSEC-2026-0542, and `snapbox`
//! needs Apache-2.0-only crates. Set `KEYMAGIC_UPDATE_SNAPSHOTS=1` to update.

use std::env;
use std::fs;
use std::io;
use std::path::Path;
use std::process::Command;

struct Case {
    args: &'static [&'static str],
    code: i32,
    snapshot: &'static str,
}

const CASES: &[Case] = &[
    Case {
        args: &["check", "tests/fixtures/good.kms"],
        code: 0,
        snapshot: "check.stdout",
    },
    Case {
        args: &["fmt", "tests/fixtures/good.kms"],
        code: 0,
        snapshot: "fmt.stdout",
    },
    Case {
        args: &["test", "tests/fixtures/corpus/cases"],
        code: 1,
        snapshot: "test.stdout",
    },
    Case {
        args: &["--version"],
        code: 0,
        snapshot: "version.stdout",
    },
    Case {
        args: &["doctor"],
        code: 2,
        snapshot: "doctor.stdout",
    },
    Case {
        args: &["install"],
        code: 2,
        snapshot: "install.stdout",
    },
];

fn check_case(case: &Case, failures: &mut Vec<String>) -> io::Result<()> {
    let output = Command::new(env!("CARGO_BIN_EXE_keymagic"))
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .args(case.args)
        .output()?;
    let name = case.args.join(" ");
    if output.status.code() != Some(case.code) {
        failures.push(format!(
            "`keymagic {name}` exited {:?}, expected {}",
            output.status.code(),
            case.code
        ));
    }
    let actual = String::from_utf8_lossy(&output.stdout).into_owned();
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/snapshots")
        .join(case.snapshot);
    if env::var_os("KEYMAGIC_UPDATE_SNAPSHOTS").is_some() {
        fs::write(&path, &actual)?;
    }
    if fs::read_to_string(&path)? != actual {
        failures.push(format!("`keymagic {name}` differs from {}", case.snapshot));
    }
    Ok(())
}

#[test]
fn cases_match_snapshots_and_exit_codes() -> io::Result<()> {
    let mut failures = Vec::new();
    for case in CASES {
        check_case(case, &mut failures)?;
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    Ok(())
}
