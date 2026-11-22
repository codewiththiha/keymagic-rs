//! Frame format, in-memory loopback, and reconnect backoff.

use keymagic_host::ipc::{self, Backoff, Message};
use std::cell::RefCell;
use std::error::Error;
use std::io;
use std::path::Path;
use std::time::Duration;

type TestResult = Result<(), Box<dyn Error>>;

#[test]
fn frame_is_a_big_endian_length_then_json() -> TestResult {
    let mut bytes = Vec::new();
    ipc::write_message(&mut bytes, &Message::ReloadConfig)?;
    let body = br#"{"type":"reload_config"}"#;
    assert_eq!(&bytes[..4], &u32::try_from(body.len())?.to_be_bytes());
    assert_eq!(&bytes[4..], body);
    Ok(())
}

#[test]
fn loopback_round_trips_every_message_kind() -> TestResult {
    let sent = [
        Message::Hello {
            version: "0.1.0".to_owned(),
        },
        Message::ActiveKeyboardChanged {
            id: "myanmar3".to_owned(),
        },
        Message::SetActive {
            id: "zawgyi".to_owned(),
        },
        Message::ReloadConfig,
        Message::Hud {
            text: "Zawgyi".to_owned(),
        },
    ];
    let mut wire = Vec::new();
    for message in &sent {
        ipc::write_message(&mut wire, message)?;
    }
    let mut reader = wire.as_slice();
    for message in &sent {
        assert_eq!(&ipc::read_message(&mut reader)?, message);
    }
    assert!(reader.is_empty());
    Ok(())
}

#[test]
fn oversized_frame_is_rejected_before_allocating() {
    let header = u32::try_from(ipc::MAX_FRAME + 1)
        .unwrap_or(u32::MAX)
        .to_be_bytes();
    let error = ipc::read_message(&mut &header[..]);
    assert_eq!(
        error.err().map(|e| e.kind()),
        Some(io::ErrorKind::InvalidData)
    );
}

#[test]
fn backoff_doubles_and_caps() {
    let delays: Vec<_> = Backoff::new(Duration::from_secs(1), Duration::from_secs(3))
        .take(4)
        .collect();
    assert_eq!(
        delays,
        [
            Duration::from_secs(1),
            Duration::from_secs(2),
            Duration::from_secs(3),
            Duration::from_secs(3),
        ]
    );
}

#[test]
fn connect_retries_with_backoff_until_it_succeeds() -> TestResult {
    let attempts = RefCell::new(0);
    let slept = RefCell::new(Vec::new());
    let connected = ipc::connect_with_retry(
        || {
            *attempts.borrow_mut() += 1;
            if *attempts.borrow() < 3 {
                Err(io::Error::from(io::ErrorKind::ConnectionRefused))
            } else {
                Ok("stream")
            }
        },
        Backoff::new(Duration::from_millis(10), Duration::from_millis(40)),
        |delay| slept.borrow_mut().push(delay),
    )?;
    assert_eq!(connected, "stream");
    assert_eq!(
        *slept.borrow(),
        [Duration::from_millis(10), Duration::from_millis(20)]
    );
    Ok(())
}

#[test]
fn connect_gives_up_when_delays_run_out() {
    let result: io::Result<()> = ipc::connect_with_retry(
        || Err(io::Error::from(io::ErrorKind::NotFound)),
        Backoff::new(Duration::ZERO, Duration::ZERO).take(2),
        |_| {},
    );
    assert_eq!(
        result.err().map(|e| e.kind()),
        Some(io::ErrorKind::NotFound)
    );
}

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
