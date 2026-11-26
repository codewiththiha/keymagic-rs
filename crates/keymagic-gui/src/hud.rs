//! Keyboard-switch notices that IME components send over IPC.

use keymagic_host::ipc::{Message, Server};
use std::io;
use std::path::Path;
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

/// How long a notice stays on screen.
pub const SHOWN_FOR: Duration = Duration::from_millis(1500);

/// The latest notice, and the queue of notices from IME clients.
#[derive(Debug)]
pub struct Hud {
    incoming: Receiver<String>,
    notice: Option<(String, Instant)>,
}

impl Hud {
    /// Serves IME clients on `socket`, calling `wake` for each notice.
    #[doc = "# Errors\n\nReturns an error when the socket cannot be bound."]
    pub fn listen(socket: &Path, wake: impl Fn() + Send + 'static) -> io::Result<Self> {
        let server = Server::bind(socket)?;
        let (sender, incoming) = mpsc::channel();
        std::thread::spawn(move || {
            while let Ok(mut link) = server.accept() {
                while let Ok(message) = link.recv() {
                    let Message::Hud { text } = message else {
                        continue;
                    };
                    if sender.send(text).is_err() {
                        return;
                    }
                    wake();
                }
            }
        });
        Ok(Self {
            incoming,
            notice: None,
        })
    }

    /// Takes the notices that arrived since the last call, stamped `now`.
    pub fn poll(&mut self, now: Instant) {
        while let Ok(text) = self.incoming.try_recv() {
            self.show(text, now);
        }
    }

    /// The notice text still on screen at `now`, if any.
    #[must_use]
    pub fn current(&self, now: Instant) -> Option<&str> {
        self.notice
            .as_ref()
            .filter(|(_, shown)| now.saturating_duration_since(*shown) < SHOWN_FOR)
            .map(|(text, _)| text.as_str())
    }

    fn show(&mut self, text: String, now: Instant) {
        self.notice = Some((text, now));
    }
}

#[cfg(test)]
mod tests {
    use super::{Hud, SHOWN_FOR};
    use std::sync::mpsc;
    use std::time::Instant;

    #[test]
    fn notice_shows_for_a_moment_then_clears() {
        let (_, incoming) = mpsc::channel();
        let mut hud = Hud {
            incoming,
            notice: None,
        };
        let now = Instant::now();
        hud.show("Zawgyi".to_owned(), now);
        assert_eq!(hud.current(now), Some("Zawgyi"));
        assert_eq!(hud.current(now + SHOWN_FOR), None);
    }
}
