//! Messages between IME components and the GUI or tray. Each frame is a
//! 4-byte big-endian body length followed by the JSON body.

use crate::invalid_data;
use serde::{Deserialize, Serialize};
use std::io::{self, Read, Write};
use std::time::Duration;

/// Largest body accepted, so a bad peer cannot force a huge allocation.
pub const MAX_FRAME: usize = 64 * 1024;

/// Messages exchanged between IME components and the GUI or tray.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Message {
    /// First message on a connection, with the sender's version.
    Hello { version: String },
    /// The active keyboard changed to `id`.
    ActiveKeyboardChanged { id: String },
    /// Asks the receiver to make `id` the active keyboard.
    SetActive { id: String },
    /// The receiver should reread `config.toml`.
    ReloadConfig,
    /// Text for the on-screen indicator.
    Hud { text: String },
}

/// Writes one frame and flushes the writer.
#[doc = "# Errors\n\nReturns an error when the body exceeds `MAX_FRAME` or the write fails."]
pub fn write_message<W: Write>(writer: &mut W, message: &Message) -> io::Result<()> {
    let body = serde_json::to_vec(message).map_err(invalid_data)?;
    if body.len() > MAX_FRAME {
        return Err(invalid_data("message too large"));
    }
    let length = u32::try_from(body.len()).map_err(invalid_data)?;
    writer.write_all(&length.to_be_bytes())?;
    writer.write_all(&body)?;
    writer.flush()
}

/// Reads one frame.
#[doc = "# Errors\n\nReturns an error for read failures, oversized or invalid frames."]
pub fn read_message<R: Read>(reader: &mut R) -> io::Result<Message> {
    let mut header = [0; 4];
    reader.read_exact(&mut header)?;
    let length = usize::try_from(u32::from_be_bytes(header)).map_err(invalid_data)?;
    if length > MAX_FRAME {
        return Err(invalid_data("frame too large"));
    }
    let mut body = vec![0; length];
    reader.read_exact(&mut body)?;
    serde_json::from_slice(&body).map_err(invalid_data)
}

/// Delays that double from `first` up to `max`, forever. Use with `take`.
#[derive(Debug, Clone)]
pub struct Backoff {
    next: Duration,
    max: Duration,
}

impl Backoff {
    /// Starts at `first` and never waits longer than `max`.
    #[must_use]
    pub const fn new(first: Duration, max: Duration) -> Self {
        Self { next: first, max }
    }
}

impl Iterator for Backoff {
    type Item = Duration;

    fn next(&mut self) -> Option<Duration> {
        let delay = self.next;
        self.next = (self.next * 2).min(self.max);
        Some(delay)
    }
}

/// Calls `connect` until it succeeds. Each failure waits for the next delay.
#[doc = "# Errors\n\nReturns the last error once `delays` runs out."]
pub fn connect_with_retry<S>(
    mut connect: impl FnMut() -> io::Result<S>,
    delays: impl IntoIterator<Item = Duration>,
    mut sleep: impl FnMut(Duration),
) -> io::Result<S> {
    let mut delays = delays.into_iter();
    loop {
        let error = match connect() {
            Ok(stream) => return Ok(stream),
            Err(error) => error,
        };
        let Some(delay) = delays.next() else {
            return Err(error);
        };
        sleep(delay);
    }
}
