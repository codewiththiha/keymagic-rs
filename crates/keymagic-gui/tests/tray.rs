//! Tray menu policy, and a tray switch reaching a fake IME over IPC.

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
        km2: PathBuf::new(),
        icon: None,
    }
}

#[test]
fn menu_lists_enabled_keyboards_with_the_active_one_marked() -> TestResult {
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
            Item {
                label: "Zawgyi (active)".to_owned(),
                command: Command::Use("zawgyi".to_owned()),
            },
            Item {
                label: "Open KeyMagic".to_owned(),
                command: Command::Open,
            },
            Item {
                label: "Quit".to_owned(),
                command: Command::Quit,
            },
        ]
    );
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
fn tray_switch_is_saved_when_no_ime_is_running() -> TestResult {
    let dir = tempfile::tempdir()?;
    let paths = paths_in(&dir);
    let mut host = Host::open(&paths)?;
    let id = install_tiny(&mut host, &dir)?;

    assert!(host.switch_keyboard(&id)?);
    let reopened = Host::open(&paths)?;
    assert_eq!(
        reopened.config().keyboards.active.as_deref(),
        Some(id.as_str())
    );
    Ok(())
}
