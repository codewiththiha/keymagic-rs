//! KeyMagic engine session for the grabbed keyboard, as protocol steps.

use keymagic_engine::{Engine, KeyEvent};
use keymagic_host::session::HostAction;
use keymagic_host::{Config, KeyboardStore, Paths, Session};
use std::path::PathBuf;
use std::sync::Arc;

/// One protocol request for the input method.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Step {
    /// Replace the preedit with this text.
    Preedit(String),
    /// Insert this text at the caret.
    Commit(String),
    /// Delete this many bytes before the caret.
    DeleteBefore(u32),
}

/// The engine session that turns key events into steps.
pub struct Composer {
    store: KeyboardStore,
    config_path: PathBuf,
    session: Session,
    tail: String,
}

impl Composer {
    /// Opens the active keyboard; prints the error and gives `None`.
    #[must_use]
    pub fn open() -> Option<Self> {
        Self::load().inspect_err(|error| eprintln!("error: {error}")).ok()
    }

    fn load() -> Result<Self, String> {
        let paths = Paths::for_user().ok_or("no user folders for this account")?;
        let store = KeyboardStore::new(&paths);
        let config_path = paths.config_file();
        let config = Config::load_or_default(&config_path).map_err(|error| error.to_string())?;
        let id = config
            .keyboards
            .active
            .clone()
            .ok_or("no keyboard is active")?;
        let session = session_for(&store, &config, &id)?;
        Ok(Self {
            store,
            config_path,
            session,
            tail: String::new(),
        })
    }

    /// Starts composing in a newly focused field.
    pub fn focus_in(&mut self) {
        self.session.focus_in("");
        self.tail.clear();
    }

    /// Flushes pending text when the field loses focus.
    #[must_use]
    pub fn focus_out(&mut self) -> Vec<Step> {
        plan(self.session.focus_out().as_slice(), &mut self.tail)
    }

    /// Feeds one key press; returns whether the engine consumed it.
    #[must_use]
    pub fn key(&mut self, event: KeyEvent) -> (bool, Vec<Step>) {
        let actions = self.session.key(event);
        let consumed = consumes(&actions);
        let switches: Vec<&str> = actions
            .iter()
            .filter_map(|action| match action {
                HostAction::SwitchKeyboard(id) => Some(id.as_str()),
                _ => None,
            })
            .collect();
        for id in switches {
            if let Err(error) = self.switch(id) {
                eprintln!("error: {error}");
            }
        }
        (consumed, plan(&actions, &mut self.tail))
    }

    fn switch(&mut self, id: &str) -> Result<(), String> {
        let config = Config::load_or_default(&self.config_path).map_err(|e| e.to_string())?;
        self.session = session_for(&self.store, &config, id)?;
        self.session.focus_in("");
        Ok(())
    }
}

fn session_for(store: &KeyboardStore, config: &Config, id: &str) -> Result<Session, String> {
    let keyboard = store
        .load(id)
        .map_err(|error| format!("cannot load {id}: {error}"))?;
    let engine = Engine::new(Arc::new(keyboard)).map_err(|error| error.to_string())?;
    Ok(Session::new(engine, config))
}

/// True when any action means the engine handled the key.
#[must_use]
pub fn consumes(actions: &[HostAction]) -> bool {
    actions
        .iter()
        .any(|action| !matches!(action, HostAction::Pass | HostAction::Hud(_)))
}

/// Maps host actions to steps, and keeps the text this client committed.
#[must_use]
pub fn plan(actions: &[HostAction], tail: &mut String) -> Vec<Step> {
    let mut steps = Vec::new();
    for action in actions {
        match action {
            HostAction::Commit(text) => {
                tail.push_str(text);
                steps.push(Step::Commit(text.clone()));
            }
            HostAction::UpdatePreedit(text) => steps.push(Step::Preedit(text.clone())),
            HostAction::ReplaceCommitted { delete, insert } => {
                let bytes = delete_bytes(tail, *delete);
                let keep = tail.len() - usize::try_from(bytes).unwrap_or(0);
                tail.truncate(keep);
                tail.push_str(insert);
                steps.push(Step::DeleteBefore(bytes));
                steps.push(Step::Commit(insert.clone()));
            }
            HostAction::Pass
            | HostAction::Eat
            | HostAction::Hud(_)
            | HostAction::SwitchKeyboard(_) => {}
        }
    }
    steps
}

/// Bytes taken by the last `chars` characters of `text`.
#[must_use]
pub fn delete_bytes(text: &str, chars: usize) -> u32 {
    let bytes: usize = text
        .chars()
        .rev()
        .take(chars)
        .map(char::len_utf8)
        .sum();
    u32::try_from(bytes).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use super::{Step, consumes, delete_bytes, plan};
    use keymagic_host::session::HostAction;

    #[test]
    fn pass_and_hud_do_not_consume_the_key() {
        assert!(!consumes(&[HostAction::Pass]));
        assert!(!consumes(&[HostAction::Hud("Unicode".to_owned())]));
        assert!(consumes(&[HostAction::Eat]));
        assert!(consumes(&[HostAction::Commit("ခ".to_owned())]));
    }

    #[test]
    fn commit_and_preedit_steps_track_the_tail() {
        let mut tail = String::new();
        let steps = plan(&[HostAction::Commit("ခ".to_owned())], &mut tail);
        assert_eq!(steps, vec![Step::Commit("ခ".to_owned())]);
        assert_eq!(tail, "ခ");
        let steps = plan(&[HostAction::UpdatePreedit("က".to_owned())], &mut tail);
        assert_eq!(steps, vec![Step::Preedit("က".to_owned())]);
        assert_eq!(tail, "ခ");
    }

    #[test]
    fn replace_deletes_the_bytes_of_the_last_chars() {
        let mut tail = "ခa".to_owned();
        let action = HostAction::ReplaceCommitted {
            delete: 1,
            insert: "က".to_owned(),
        };
        let steps = plan(&[action], &mut tail);
        assert_eq!(
            steps,
            vec![Step::DeleteBefore(1), Step::Commit("က".to_owned())]
        );
        assert_eq!(tail, "ခက");
    }

    #[test]
    fn delete_counts_bytes_not_chars() {
        assert_eq!(delete_bytes("aခ", 1), 3);
        assert_eq!(delete_bytes("ab", 5), 2);
        assert_eq!(delete_bytes("", 1), 0);
    }
}
