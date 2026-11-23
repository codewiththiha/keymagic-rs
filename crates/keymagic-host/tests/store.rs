//! Keyboard store against real files in a temporary folder.

use keymagic_host::config::Config;
use keymagic_host::{KeyboardStore, paths::Paths};
use std::error::Error;
use std::time::Duration;

type TestResult = Result<(), Box<dyn Error>>;

const PNG_SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";
const MYWIN: &[u8] = include_bytes!("../../../corpus/km2/alternatives/myWin.km2");
/// Id made from the KM2 name, "my-Win 2.3.0 Unicode 5.2".
const MYWIN_ID: &str = "my-win-2-3-0-unicode-5-2";

struct Fixture {
    dir: tempfile::TempDir,
}

impl Fixture {
    fn new() -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            dir: tempfile::tempdir()?,
        })
    }

    fn paths(&self) -> Paths {
        Paths::under(
            &self.dir.path().join("config"),
            &self.dir.path().join("cache"),
        )
    }

    fn store(&self) -> KeyboardStore {
        KeyboardStore::new(&self.paths())
    }
}

#[test]
fn import_km2_installs_and_scan_finds_it_with_a_png_icon() -> TestResult {
    let fixture = Fixture::new()?;
    let store = fixture.store();
    let mut config = Config::default();
    let id = store.import_km2(MYWIN, &mut config)?;
    assert_eq!(id, MYWIN_ID);
    assert_eq!(config.keyboards.installed.len(), 1);

    let mut rescanned = Config::default();
    let found = store.scan(&mut rescanned)?;
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].id, MYWIN_ID);
    let icon = found[0].icon.as_ref().ok_or("icon was not cached")?;
    assert_eq!(&std::fs::read(icon)?[..8], PNG_SIGNATURE);
    assert_eq!(rescanned.keyboards.installed[0].id, MYWIN_ID);
    Ok(())
}

#[test]
fn second_import_gets_a_new_id_and_keeps_the_first_file() -> TestResult {
    let fixture = Fixture::new()?;
    let store = fixture.store();
    let mut config = Config::default();
    let first = store.import_km2(MYWIN, &mut config)?;
    let second = store.import_km2(MYWIN, &mut config)?;
    assert_eq!(first, MYWIN_ID);
    assert_eq!(second, format!("{MYWIN_ID}-2"));
    let keyboards = fixture.paths().keyboards_dir();
    assert!(keyboards.join(format!("{MYWIN_ID}.km2")).exists());
    assert!(keyboards.join(format!("{MYWIN_ID}-2.km2")).exists());
    Ok(())
}

#[test]
fn import_kms_stores_source_and_compiled_file() -> TestResult {
    let fixture = Fixture::new()?;
    let source = fixture.dir.path().join("tiny.kms");
    std::fs::write(&source, "\"a\" => \"x\"\n")?;
    let mut config = Config::default();
    let id = fixture.store().import_kms(&source, &mut config)?;
    let keyboards = fixture.paths().keyboards_dir();
    assert!(keyboards.join(format!("{id}.kms")).exists());
    assert!(keyboards.join(format!("{id}.km2")).exists());
    Ok(())
}

#[test]
fn import_kms_reports_the_first_compile_error() -> TestResult {
    let fixture = Fixture::new()?;
    let source = fixture.dir.path().join("bad.kms");
    std::fs::write(&source, "$x => \"a\"\n")?;
    let error = fixture.store().import_kms(&source, &mut Config::default());
    assert_eq!(
        error.err().map(|e| e.kind()),
        Some(std::io::ErrorKind::InvalidData)
    );
    Ok(())
}

#[test]
fn remove_deletes_both_files_and_forgets_the_entry() -> TestResult {
    let fixture = Fixture::new()?;
    let store = fixture.store();
    let mut config = Config::default();
    let id = store.import_km2(MYWIN, &mut config)?;
    config.keyboards.active = Some(id.clone());
    assert!(store.remove(&id, &mut config)?);
    assert!(config.keyboards.installed.is_empty());
    assert_eq!(config.keyboards.active, None);
    assert!(
        !fixture
            .paths()
            .keyboards_dir()
            .join(format!("{id}.km2"))
            .exists()
    );
    assert!(!store.remove(&id, &mut config)?);
    Ok(())
}

#[test]
fn scan_drops_entries_whose_file_is_gone() -> TestResult {
    let fixture = Fixture::new()?;
    let store = fixture.store();
    let mut config = Config::default();
    store.import_km2(MYWIN, &mut config)?;
    std::fs::remove_file(
        fixture
            .paths()
            .keyboards_dir()
            .join(format!("{MYWIN_ID}.km2")),
    )?;
    store.scan(&mut config)?;
    assert!(config.keyboards.installed.is_empty());
    Ok(())
}

#[test]
fn scan_ignores_other_files_in_the_folder() -> TestResult {
    let fixture = Fixture::new()?;
    let folder = fixture.paths().keyboards_dir();
    std::fs::create_dir_all(&folder)?;
    std::fs::write(folder.join("notes.txt"), "not a keyboard")?;
    assert!(fixture.store().scan(&mut Config::default())?.is_empty());
    Ok(())
}

#[test]
fn watch_reports_a_new_keyboard_file() -> TestResult {
    let fixture = Fixture::new()?;
    let store = fixture.store();
    let (_watcher, events) = store.watch()?;
    store.import_km2(MYWIN, &mut Config::default())?;
    let file_name = format!("{MYWIN_ID}.km2");
    let seen = std::iter::from_fn(|| events.recv_timeout(Duration::from_secs(10)).ok())
        .take(20)
        .filter_map(Result::ok)
        .any(|event| event.paths.iter().any(|path| path.ends_with(&file_name)));
    assert!(seen, "no event for {file_name}");
    Ok(())
}

#[test]
fn save_source_writes_only_when_the_source_compiles() -> TestResult {
    let fixture = Fixture::new()?;
    let store = fixture.store();
    let good = store.save_source("tiny", "\"a\" => \"x\"\n")?;
    assert!(good.is_empty());
    assert_eq!(store.source("tiny")?.as_deref(), Some("\"a\" => \"x\"\n"));
    assert!(store.load("tiny").is_ok());
    let bad = store.save_source("tiny", "$x => \"a\"\n")?;
    assert!(!bad.is_empty());
    assert_eq!(store.source("tiny")?.as_deref(), Some("\"a\" => \"x\"\n"));
    Ok(())
}

#[test]
fn source_is_none_for_km2_only_keyboards() -> TestResult {
    let fixture = Fixture::new()?;
    let store = fixture.store();
    let id = store.import_km2(MYWIN, &mut Config::default())?;
    assert_eq!(store.source(&id)?, None);
    assert!(store.load(&id).is_ok());
    Ok(())
}
