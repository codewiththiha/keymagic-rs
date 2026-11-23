//! OS services: local sockets, the HTTP fetch and the file watcher.
//! These need the platform, so they stay out of the unit-test targets.

use keymagic_host::config::Config;
use keymagic_host::ipc::{self, Backoff, Message};
use keymagic_host::updater::fetch;
use keymagic_host::{KeyboardStore, paths::Paths};
use std::error::Error;
use std::io::{self, Read, Write};
use std::path::Path;
use std::sync::mpsc;
use std::time::Duration;

type TestResult = Result<(), Box<dyn Error>>;

const MYWIN: &[u8] = include_bytes!("../../../corpus/km2/alternatives/myWin.km2");
/// Id made from the KM2 name, "my-Win 2.3.0 Unicode 5.2".
const MYWIN_ID: &str = "my-win-2-3-0-unicode-5-2";
const WAIT: Duration = Duration::from_secs(10);

fn store_in(dir: &tempfile::TempDir) -> KeyboardStore {
    KeyboardStore::new(&Paths::under(
        &dir.path().join("config"),
        &dir.path().join("cache"),
    ))
}

const MOCK: &str = include_str!("fixtures/updates.json");

#[test]
fn local_socket_carries_a_message_both_ways() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("keymagic.sock");
    let server = ipc::Server::bind(&path)?;
    let client = std::thread::spawn(move || -> io::Result<Message> {
        let mut connection = ipc::connect(&path, [Duration::from_millis(10)])?;
        connection.send(&Message::SetActive {
            id: "myanmar3".to_owned(),
        })?;
        connection.recv()
    });
    let mut peer = server.accept()?;
    assert_eq!(
        peer.recv()?,
        Message::SetActive {
            id: "myanmar3".to_owned()
        }
    );
    peer.send(&Message::Hud {
        text: "Myanmar 3".to_owned(),
    })?;
    let reply = client
        .join()
        .map_err(|_| io::Error::other("client thread panicked"))??;
    assert_eq!(
        reply,
        Message::Hud {
            text: "Myanmar 3".to_owned()
        }
    );
    Ok(())
}


#[test]
fn connect_fails_without_a_server_once_retries_run_out() -> TestResult {
    let dir = tempfile::tempdir()?;
    let missing: &Path = &dir.path().join("absent.sock");
    assert!(ipc::connect(missing, [Duration::ZERO, Duration::ZERO]).is_err());
    Ok(())
}


#[test]
fn client_waits_for_a_server_that_starts_late() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("late.sock");
    let server_path = path.clone();
    let server = std::thread::spawn(move || -> io::Result<Message> {
        std::thread::sleep(Duration::from_millis(150));
        let server = ipc::Server::bind(&server_path)?;
        server.accept()?.recv()
    });
    let backoff = Backoff::new(Duration::from_millis(20), Duration::from_millis(200));
    let mut client = ipc::Client::new(&path, backoff);
    client.send(&Message::ReloadConfig)?;
    let received = server
        .join()
        .map_err(|_| io::Error::other("server thread panicked"))??;
    assert_eq!(received, Message::ReloadConfig);
    Ok(())
}

#[test]
fn fetch_downloads_and_parses_the_manifest_over_http() -> TestResult {
    let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
    let url = format!("http://{}/updates.json", listener.local_addr()?);
    let server = std::thread::spawn(move || -> io::Result<()> {
        let (mut stream, _) = listener.accept()?;
        let mut request = [0_u8; 1024];
        let _ = stream.read(&mut request)?;
        write!(
            stream,
            concat!(
                "HTTP/1.1 200 OK\r\n",
                "Content-Type: application/json\r\n",
                "Content-Length: {}\r\n",
                "Connection: close\r\n\r\n{}"
            ),
            MOCK.len(),
            MOCK
        )
    });
    let manifest = fetch(&url)?;
    server
        .join()
        .map_err(|_| io::Error::other("server thread panicked"))??;
    assert_eq!(manifest, Manifest::parse(MOCK.as_bytes())?);
    Ok(())
}

#[test]
fn watch_reports_a_new_keyboard_file() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = store_in(&dir);
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
fn client_reconnects_after_the_server_restarts() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("restart.sock");
    let first = ipc::Server::bind(&path)?;
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let received = first.accept().and_then(|mut link| link.recv());
        let _ = tx.send((received, first));
    });
    let backoff = Backoff::new(Duration::from_millis(20), Duration::from_millis(200));
    let mut client = ipc::Client::new(&path, backoff);
    client.send(&Message::ReloadConfig)?;
    let (received, first) = rx.recv_timeout(WAIT)?;
    assert_eq!(received?, Message::ReloadConfig);
    drop(first);

    let second = ipc::Server::bind(&path)?;
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(second.accept().and_then(|mut link| link.recv()));
    });
    client.send(&Message::SetActive {
        id: "myanmar3".to_owned(),
    })?;
    let received = rx.recv_timeout(WAIT)??;
    assert_eq!(
        received,
        Message::SetActive {
            id: "myanmar3".to_owned()
        }
    );
    Ok(())
}
