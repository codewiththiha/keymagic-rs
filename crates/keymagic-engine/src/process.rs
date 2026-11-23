//! Public engine: composing session, key processing, peek and external control.

use crate::apply::{captured, render, stops_recursion};
use crate::input::KeyEvent;
use crate::matcher::find;
use crate::record::{Recorder, TraceStep};
use crate::result::{Action, Output};
use crate::rules::{CompiledRule, RuleIndex, compile};
use crate::states::States;
use crate::text::{ComposingBuffer, Journal};
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
    /// Text changes of recorded events, oldest first; `log[i]` is `base + i`.
    log: VecDeque<Journal>,
    /// Absolute position of `log[0]`.
    base: usize,
    /// Log position before each event kept for smart backspace, oldest first.
    history: VecDeque<usize>,
}

impl Session {
    /// Absolute position just after the last recorded change.
    fn end(&self) -> usize {
        self.base + self.log.len()
    }

    /// Keeps `position` as an undo point and drops history beyond the limit.
    fn record_history(&mut self, position: usize) {
        self.history.push_back(position);
        if self.history.len() > HISTORY_LIMIT {
            self.history.pop_front();
        }
        self.prune();
    }

    /// Drops log entries no remaining undo point can reach.
    fn prune(&mut self) {
        let Some(&oldest) = self.history.front() else {
            self.log.clear();
            self.base = 0;
            return;
        };
        while self.base < oldest {
            self.log.pop_front();
            self.base += 1;
        }
    }

    /// Restores the text as it was at undo point `position`.
    fn rewind(&mut self, position: usize) {
        while self.end() > position {
            let Some(journal) = self.log.pop_back() else {
                break;
            };
            self.buffer.restore(&journal);
        }
        self.prune();
    }

    /// Forgets all undo points and changes.
    fn clear_history(&mut self) {
        self.history.clear();
        self.log.clear();
        self.base = 0;
    }
}

/// Deterministic key-to-text engine for one keyboard layout.
#[derive(Debug)]
pub struct Engine {
    keyboard: Arc<Keyboard>,
    rules: Vec<CompiledRule>,
    index: RuleIndex,
    session: Session,
}

impl Engine {
    /// Builds an engine; fails on the first invalid rule element.
    #[doc = "# Errors\n\nReturns the first invalid rule element."]
    pub fn new(keyboard: Arc<Keyboard>) -> Result<Self, ValidationError> {
        keyboard.validate()?;
        let rules = compile(&keyboard);
        let index = RuleIndex::new(&rules);
        Ok(Self {
            keyboard,
            rules,
            index,
            session: Session::default(),
        })
    }

    /// Processes one key press and updates the composing text.
    pub fn process(&mut self, event: KeyEvent) -> Output {
        let core = Core {
            keyboard: &self.keyboard,
            rules: &self.rules,
            index: &self.index,
        };
        core.run(&mut self.session, event)
    }

    /// Returns the output `event` would produce, leaving the engine unchanged.
    #[must_use]
    pub fn peek(&self, event: KeyEvent) -> Output {
        let core = Core {
            keyboard: &self.keyboard,
            rules: &self.rules,
            index: &self.index,
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
        self.session.clear_history();
    }

    /// Returns the composing text.
    #[must_use]
    pub fn composing(&self) -> &str {
        self.session.buffer.as_str()
    }
}

/// How a key event was handled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Outcome {
    /// The engine ignored the key and left the text alone.
    Ignored,
    /// The engine handled the key and changed the text.
    Consumed,
    /// Smart backspace rewound the text to an earlier undo point.
    Rewound,
}

/// Borrowed keyboard data shared by processing and peek.
struct Core<'a> {
    keyboard: &'a Keyboard,
    rules: &'a [CompiledRule],
    index: &'a RuleIndex,
}

impl Core<'_> {
    fn run(&self, session: &mut Session, event: KeyEvent) -> Output {
        let mut recorder = Recorder::default();
        let start = session.end();
        session.buffer.begin_event();
        let outcome = self.step(session, event, &mut recorder);
        session.states.finish_event();
        let action = Action::from_edit(&session.buffer.edit());
        if outcome != Outcome::Rewound && action != Action::None {
            session.log.push_back(session.buffer.journal());
        }
        if outcome != Outcome::Ignored && event.key != Some(VirtualKey::Back) {
            session.record_history(start);
        }
        Output {
            action,
            consumed: outcome != Outcome::Ignored,
            composing: session.buffer.as_str().to_owned(),
            trace: recorder.finish(),
        }
    }

    fn step(&self, session: &mut Session, event: KeyEvent, recorder: &mut Recorder) -> Outcome {
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
        Outcome::Consumed
    }

    fn key_match(
        &self,
        session: &mut Session,
        event: KeyEvent,
        recorder: &mut Recorder,
    ) -> Option<(usize, bool, String)> {
        let found = find(
            self.rules,
            self.index,
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
            captures: captured(found.text, &found.captures),
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
            self.index,
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
            captures: captured(found.text, &found.captures),
        });
        Some((found.rule.match_len, output))
    }

    fn unmatched(&self, session: &mut Session, event: KeyEvent) -> Outcome {
        if event.key == Some(VirtualKey::Back) && !session.buffer.is_empty() {
            let restored = if self.keyboard.options.auto_bksp {
                session.history.pop_back()
            } else {
                None
            };
            if let Some(position) = restored {
                session.rewind(position);
                return Outcome::Rewound;
            }
            session.buffer.pop();
            return Outcome::Consumed;
        }
        if let Some(ch) = event.ch {
            if !self.keyboard.options.eat {
                session.buffer.push(ch);
            }
            return Outcome::Consumed;
        }
        Outcome::Ignored
    }
}
