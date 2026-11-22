//! Corpus synchronization tests using a local pinned Git repository.

use super::{Manifest, Mode, Scratch, sync};
use std::{error::Error, fs, path::Path, process::Command};

fn git(repo: &Path, args: &[&str]) -> Result<String, Box<dyn Error>> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).into_owned().into());
    }
    Ok(String::from_utf8(output.stdout)?.trim().to_owned())
}

#[test]
fn manifest_rejects_unsafe_paths_and_bad_hashes() -> Result<(), Box<dyn Error>> {
    let revision = "a".repeat(40);
    let blob = "b".repeat(40);
    let digest = "c".repeat(64);
    let source = format!("source\thttps://example.test/repo\t{revision}\n");
    let valid = format!("{source}file\tinput.kms\tkms/input.kms\t{blob}\t{digest}\n");
    assert_eq!(Manifest::parse(&valid)?.files.len(), 1);
    assert!(Manifest::parse(&valid.replace("kms/input.kms", "../escape.kms")).is_err());
    assert!(Manifest::parse(&valid.replace("input.kms", "../input.kms")).is_err());
    assert!(Manifest::parse(&valid.replace(&blob, "bad")).is_err());
    assert!(Manifest::parse(&valid.replace(&digest, "bad")).is_err());
    assert!(Manifest::parse(&format!("{valid}{valid}")).is_err());
    Ok(())
}

#[test]
fn sync_dry_run_check_and_restore_pinned_bytes() -> Result<(), Box<dyn Error>> {
    let scratch = Scratch::new()?;
    let source = scratch.path().join("source");
    let destination = scratch.path().join("corpus");
    fs::create_dir(&source)?;
    fs::create_dir(&destination)?;
    git(&source, &["init", "-q"])?;
    git(&source, &["config", "user.name", "Test"])?;
    git(&source, &["config", "user.email", "test@example.invalid"])?;
    fs::write(source.join("fixture.kms"), b"original\r\n")?;
    git(&source, &["add", "fixture.kms"])?;
    git(&source, &["commit", "-qm", "fixture"])?;
    let revision = git(&source, &["rev-parse", "HEAD"])?;
    let blob = git(&source, &["rev-parse", "HEAD:fixture.kms"])?;
    let digest = "a".repeat(64);
    let text = format!(
        "source\thttps://example.test/repo\t{revision}\nfile\tfixture.kms\tkms/fixture.kms\t{blob}\t{digest}\n"
    );
    let mut manifest = Manifest::parse(&text)?;
    manifest.repository = source.to_string_lossy().into_owned();
    assert_eq!(sync(&manifest, &destination, Mode::DryRun)?, 1);
    assert!(!destination.join("kms/fixture.kms").exists());
    assert!(sync(&manifest, &destination, Mode::Check).is_err());
    assert_eq!(sync(&manifest, &destination, Mode::Write)?, 1);
    assert_eq!(
        fs::read(destination.join("kms/fixture.kms"))?,
        b"original\r\n"
    );
    assert_eq!(sync(&manifest, &destination, Mode::Check)?, 0);
    fs::write(destination.join("kms/fixture.kms"), b"modified")?;
    assert!(sync(&manifest, &destination, Mode::Check).is_err());
    assert_eq!(sync(&manifest, &destination, Mode::Write)?, 1);
    assert_eq!(
        fs::read(destination.join("kms/fixture.kms"))?,
        b"original\r\n"
    );
    manifest.files[0].blob = "0".repeat(40);
    assert!(sync(&manifest, &destination, Mode::Write).is_err());
    assert_eq!(
        fs::read(destination.join("kms/fixture.kms"))?,
        b"original\r\n"
    );
    Ok(())
}
