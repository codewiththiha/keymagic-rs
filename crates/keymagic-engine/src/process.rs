//! Public engine: composing session, key processing, peek and external control.

use crate::apply::{render, stops_recursion};
use crate::input::KeyEvent;
use crate::matcher::find;
use crate::record::{Recorder, TraceStep};
use crate::result::{Action, Output};
use crate::rules::{CompiledRule, compile};
use crate::states::States;
use crate::text::ComposingBuffer;
use keymagic_types::{Keyboard, ValidationError, VirtualKey};
use std::collections::VecDeque;
use std::sync::Arc;

/// Maximum recursive rule applications per key event, as in upstream KM3.
pub const MAX_RECURSION_DEPTH: usize = 10;

/// Snapshots kept for smart backspace.
const HISTORY_LIMIT: usize = 20;

/// Composing text, states and undo history; everything a key event may change.
#[derive(Debug, Clone, Default)]
struct Session {
    buffer: ComposingBuffer,
    states: States,
    history: VecDeque<ComposingBuffer>,
}

/// Deterministic key-to-text engine for one keyboard layout.
#[derive(Debug)]
pub struct Engine {
    keyboard: Arc<Keyboard>,
    rules: Vec<CompiledRule>,
    session: Session,
}

impl Engine {
    /// Builds an engine; fails on the first invalid rule element.
    #[doc = "# Errors\n\nReturns the first invalid rule element."]
    pub fn new(keyboard: Arc<Keyboard>) -> Result<Self, ValidationError> {
        keyboard.validate()?;
        let rules = compile(&keyboard);
        Ok(Self {
            keyboard,
            rules,
            session: Session::default(),
        })
    }

    /// Processes one key press and updates the composing text.
    pub fn process(&mut self, event: KeyEvent) -> Output {
        let core = Core {
            keyboard: &self.keyboard,
            rules: &self.rules,
        };
        core.run(&mut self.session, event)
    }

    /// Returns the output `event` would produce, leaving the engine unchanged.
    #[must_use]
    pub fn peek(&self, event: KeyEvent) -> Output {
        let core = Core {
            keyboard: &self.keyboard,
            rules: &self.rules,
        };
        core.run(&mut self.session.clone(), event)
    }

    /// Clears composing text, states and undo history.
    pub fn reset(&mut self) {
        self.session = Session::default();
    }

    /// Replaces composing text and clears states and undo history.
    pub fn set_composing(&mut self, text: &str) {
        self.session.buffer = ComposingBuffer::from_text(text);
        self.session.states.clear();
        self.session.history.clear();
    }

    /// Returns the composing text.
    #[must_use]
    pub fn composing(&self) -> String {
        self.session.buffer.to_text()
    }
}

/// Borrowed keyboard data shared by processing and peek.
struct Core<'a> {
    keyboard: &'a Keyboard,
    rules: &'a [CompiledRule],
}

impl Core<'_> {
    fn run(&self, session: &mut Session, event: KeyEvent) -> Output {
        let mut recorder = Recorder::default();
        let before = session.buffer.clone();
        let consumed = self.step(session, event, &mut recorder);
        session.states.finish_event();
        let action = Action::from_edit(&session.buffer.diff(&before));
        if event.key != Some(VirtualKey::Back) && consumed {
            session.history.push_back(before);
            if session.history.len() > HISTORY_LIMIT {
                session.history.pop_front();
            }
        }
        Output {
            action,
            consumed,
            composing: session.buffer.to_text(),
            trace: recorder.finish(),
        }
    }

    fn step(&self, session: &mut Session, event: KeyEvent, recorder: &mut Recorder) -> bool {
        let Some((match_len, has_vk, output)) = self.key_match(session, event, recorder) else {
            return self.unmatched(session, event);
        };
        let appended = if has_vk { None } else { event.ch };
        if let Some(ch) = appended {
            session.buffer.push(ch);
        }
        session.buffer.replace_tail(match_len, &output);
        if !stops_recursion(&output) {
            self.recurse(session, recorder);
        }
        true
    }

    fn key_match(
        &self,
        session: &mut Session,
        event: KeyEvent,
        recorder: &mut Recorder,
    ) -> Option<(usize, bool, String)> {
        let found = find(
            self.rules,
            session.buffer.chars(),
            Some(&event),
            &session.states.active,
        )?;
        let output = render(
            self.keyboard,
            found.rule.index,
            found.text,
            &found.captures,
            &mut session.states.next,
        );
        recorder.record(|| TraceStep::Applied {
            rule: found.rule.index,
            rank: found.rank,
            recursive: false,
            output: output.clone(),
        });
        Some((found.rule.match_len, found.rule.has_vk(), output))
    }

    fn recurse(&self, session: &mut Session, recorder: &mut Recorder) {
        for _ in 0..MAX_RECURSION_DEPTH {
            let Some((match_len, output)) = self.text_match(session, recorder) else {
                return;
            };
            session.buffer.replace_tail(match_len, &output);
            if stops_recursion(&output) {
                return;
            }
        }
        recorder.record(|| TraceStep::Capped);
    }

    fn text_match(
        &self,
        session: &mut Session,
        recorder: &mut Recorder,
    ) -> Option<(usize, String)> {
        let found = find(
            self.rules,
            session.buffer.chars(),
            None,
            &session.states.next,
        )?;
        let output = render(
            self.keyboard,
            found.rule.index,
            found.text,
            &found.captures,
            &mut session.states.next,
        );
        recorder.record(|| TraceStep::Applied {
            rule: found.rule.index,
            rank: found.rank,
            recursive: true,
            output: output.clone(),
        });
        Some((found.rule.match_len, output))
    }

    fn unmatched(&self, session: &mut Session, event: KeyEvent) -> bool {
        if event.key == Some(VirtualKey::Back) && !session.buffer.is_empty() {
            let restored = if self.keyboard.options.auto_bksp {
                session.history.pop_back()
            } else {
                None
            };
            match restored {
                Some(previous) => session.buffer = previous,
                None => session.buffer.pop(),
            }
            return true;
        }
        if let Some(ch) = event.ch {
            if !self.keyboard.options.eat {
                session.buffer.push(ch);
            }
            return true;
        }
        false
    }
}
