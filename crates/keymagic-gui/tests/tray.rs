//! Tray menu policy, and tray commands reaching a fake IME over IPC.

use keymagic_gui::state::Host;
use keymagic_gui::tray::{Command, Item, items};
use keymagic_host::ipc::{Message, Server};
use keymagic_host::store::Found;
use keymagic_host::{Config, Paths};
use std::error::Error;
use std::fs;
use std::io;
use std::path::PathBuf;

type TestResult = Result<(), Box<dyn Error>>;

const TINY: &str = "\"a\" => \"x\"\n";

fn paths_in(dir: &tempfile::TempDir) -> Paths {
    Paths::under(&dir.path().join("config"), &dir.path().join("cache"))
}

/// Installs and returns the id of a one-rule keyboard.
fn install_tiny(host: &mut Host, dir: &tempfile::TempDir) -> Result<String, Box<dyn Error>> {
    let source = dir.path().join("tiny.kms");
    fs::write(&source, TINY)?;
    Ok(host.import(&source)?)
}

fn found(id: &str, name: &str) -> Found {
    Found {
        id: id.to_owned(),
        name: name.to_owned(),
        description: None,
        default_hotkey: None,
        km2: PathBuf::new(),
        icon: None,
    }
}

fn entry(label: &str, command: Command, checked: Option<bool>) -> Item {
    Item {
        label: label.to_owned(),
        command,
        icon: None,
        checked,
    }
}

#[test]
fn menu_starts_with_the_on_off_entry_then_enabled_keyboards() -> TestResult {
    let config = Config::from_toml(
        r#"
        [keyboards]
        active = "zawgyi"

        [[keyboards.installed]]
        id = "zawgyi"
        enabled = true

        [[keyboards.installed]]
        id = "unicode"
        enabled = false
        "#,
    )?;
    let found = [found("zawgyi", "Zawgyi"), found("unicode", "Unicode")];
    assert_eq!(
        items(&config, &found),
        [
            entry("KeyMagic on", Command::Toggle, Some(true)),
            entry("Zawgyi (active)", Command::Use("zawgyi".to_owned()), None),
            entry("Open KeyMagic", Command::Open, None),
            entry("Quit", Command::Quit, None),
        ]
    );
    Ok(())
}

#[test]
fn keyboard_items_carry_their_icons() -> TestResult {
    let config = Config::from_toml(
        r#"
        [[keyboards.installed]]
        id = "zawgyi"
        "#,
    )?;
    let mut keyboard = found("zawgyi", "Zawgyi");
    keyboard.icon = Some(PathBuf::from("cache/icons/zawgyi.png"));
    let menu = items(&config, &[keyboard]);
    assert_eq!(
        menu[1].icon.as_deref(),
        Some(PathBuf::from("cache/icons/zawgyi.png").as_path())
    );
    assert_eq!(menu[0].icon, None, "the on/off entry has no icon");
    Ok(())
}

#[test]
fn tray_switch_reaches_a_fake_ime_over_ipc() -> TestResult {
    let dir = tempfile::tempdir()?;
    let paths = paths_in(&dir);
    let mut host = Host::open(&paths)?;
    let id = install_tiny(&mut host, &dir)?;

    let socket = paths.socket_path();
    let server = Server::bind(&socket).map_err(|error| format!("bind {socket:?}: {error}"))?;
    let fake_ime = std::thread::spawn(move || -> io::Result<Message> {
        let mut link = server.accept()?;
        link.recv()
    });
    let switched = host
        .switch_keyboard(&id)
        .map_err(|error| format!("switch: {error}"))?;
    assert!(switched);

    let received = fake_ime.join().map_err(|_| "the fake IME panicked")?;
    let received = received.map_err(|error| format!("fake IME receive: {error}"))?;
    assert_eq!(received, Message::SetActive { id: id.clone() });
    assert_eq!(host.config().keyboards.active.as_deref(), Some(id.as_str()));
    Ok(())
}

#[test]
fn tray_off_reaches_a_fake_ime_and_is_saved() -> TestResult {
    let dir = tempfile::tempdir()?;
    let paths = paths_in(&dir);
    let mut host = Host::open(&paths)?;
    let id = install_tiny(&mut host, &dir)?;
    assert!(host.switch_keyboard(&id)?);

    let socket = paths.socket_path();
    let server = Server::bind(&socket).map_err(|error| format!("bind {socket:?}: {error}"))?;
    let fake_ime = std::thread::spawn(move || -> io::Result<Message> {
        let mut link = server.accept()?;
        link.recv()
    });
    assert!(host.turn_off()?);

    let received = fake_ime.join().map_err(|_| "the fake IME panicked")?;
    assert_eq!(received?, Message::ReloadConfig);
    assert_eq!(host.config().keyboards.active, None);
    assert_eq!(Host::open(&paths)?.config().keyboards.active, None);
    Ok(())
}

#[test]
fn tray_on_turns_on_the_keyboard_used_last() -> TestResult {
    let dir = tempfile::tempdir()?;
    let paths = paths_in(&dir);
    let mut host = Host::open(&paths)?;
    let id = install_tiny(&mut host, &dir)?;
    assert!(host.switch_keyboard(&id)?);
    assert!(host.turn_off()?);
    assert!(host.turn_on()?);
    assert_eq!(host.config().keyboards.active.as_deref(), Some(id.as_str()));
    Ok(())
}
