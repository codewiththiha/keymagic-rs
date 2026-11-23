//! View models: plain data built from host values, with no egui types.

use keymagic_engine::{KeyEvent, VirtualKey};
use keymagic_host::store::Found;
use keymagic_host::{Config, HostAction};
use keymagic_kms::{Diagnostic, Severity};
use keymagic_types::Hotkey;
use std::path::PathBuf;
use std::str::FromStr;

/// One row of the Keyboards page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyboardRow {
    pub id: String,
    pub name: String,
    pub icon: Option<PathBuf>,
    pub hotkey: Option<String>,
    pub enabled: bool,
    pub active: bool,
}

/// Rows for every keyboard on disk, in saved order, then unsaved ones.
#[must_use]
pub fn keyboard_rows(config: &Config, found: &[Found]) -> Vec<KeyboardRow> {
    let installed = &config.keyboards.installed;
    let saved = installed
        .iter()
        .filter_map(|entry| found.iter().find(|keyboard| keyboard.id == entry.id));
    let unsaved = found
        .iter()
        .filter(|keyboard| !installed.iter().any(|entry| entry.id == keyboard.id));
    saved
        .chain(unsaved)
        .map(|keyboard| {
            let entry = installed.iter().find(|entry| entry.id == keyboard.id);
            KeyboardRow {
                id: keyboard.id.clone(),
                name: entry
                    .and_then(|entry| non_blank(entry.label.as_deref()))
                    .unwrap_or(keyboard.name.as_str())
                    .to_owned(),
                icon: keyboard.icon.clone(),
                hotkey: entry
                    .and_then(|entry| non_blank(entry.hotkey.as_deref()))
                    .map(str::to_owned),
                enabled: entry.is_none_or(|entry| entry.enabled),
                active: config.keyboards.active.as_deref() == Some(keyboard.id.as_str()),
            }
        })
        .collect()
}

fn non_blank(text: Option<&str>) -> Option<&str> {
    text.map(str::trim).filter(|text| !text.is_empty())
}

/// Whether `row` matches the typed filter. A blank filter matches all.
#[must_use]
pub fn matches_filter(row: &KeyboardRow, query: &str) -> bool {
    let query = query.trim().to_lowercase();
    query.is_empty()
        || row.name.to_lowercase().contains(&query)
        || row.id.to_lowercase().contains(&query)
}

/// A key the user typed into the tester.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypedKey {
    Char(char),
    Backspace,
    Enter,
}

impl TypedKey {
    /// The engine event for this key.
    #[must_use]
    pub fn event(self) -> KeyEvent {
        match self {
            Self::Char(ch) => KeyEvent {
                ch: Some(ch),
                ..KeyEvent::default()
            },
            Self::Backspace => KeyEvent {
                key: Some(VirtualKey::Back),
                ..KeyEvent::default()
            },
            Self::Enter => KeyEvent {
                key: Some(VirtualKey::Return),
                ..KeyEvent::default()
            },
        }
    }

    /// What the text shows when the session does not handle the key.
    #[must_use]
    pub const fn fallback(self) -> Fallback {
        match self {
            Self::Char(ch) => Fallback::Character(ch),
            Self::Backspace => Fallback::Backspace,
            Self::Enter => Fallback::Character('\n'),
        }
    }
}

/// What the app does with a key the session passed on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fallback {
    Character(char),
    Backspace,
}

/// What the tester shows: committed text and the current preedit.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TesterText {
    committed: String,
    preedit: String,
}

impl TesterText {
    #[must_use]
    pub fn committed(&self) -> &str {
        &self.committed
    }

    #[must_use]
    pub fn preedit(&self) -> &str {
        &self.preedit
    }

    /// Clears both parts.
    pub fn clear(&mut self) {
        self.committed.clear();
        self.preedit.clear();
    }

    /// Applies one key's answer; returns a keyboard switch it asked for.
    #[must_use]
    pub fn apply(&mut self, actions: &[HostAction], fallback: Fallback) -> Option<String> {
        let mut switch = None;
        for action in actions {
            match action {
                HostAction::Commit(text) => {
                    self.committed.push_str(text);
                    self.preedit.clear();
                }
                HostAction::UpdatePreedit(text) => self.preedit.clone_from(text),
                HostAction::ReplaceCommitted { delete, insert } => {
                    for _ in 0..*delete {
                        self.committed.pop();
                    }
                    self.committed.push_str(insert);
                }
                HostAction::Pass => match fallback {
                    Fallback::Character(ch) => self.committed.push(ch),
                    Fallback::Backspace => {
                        self.committed.pop();
                    }
                },
                HostAction::SwitchKeyboard(id) => switch = Some(id.clone()),
                HostAction::Eat | HostAction::Hud(_) => {}
            }
        }
        switch
    }
}

/// Time the editor waits after the last edit before checking, in seconds.
pub const CHECK_DELAY: f64 = 0.15;

/// Whether the editor should check its text at a given time.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Check {
    /// No edit is waiting.
    Idle,
    /// An edit is waiting; check again after this many seconds.
    Wait(f64),
    /// The quiet period has passed; check now.
    Due,
}

/// The Editor page's text and when it was last changed.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct EditorText {
    pub id: Option<String>,
    pub source: String,
    edited_at: Option<f64>,
}

impl EditorText {
    /// Starts editing keyboard `id` with `source`, clearing any pending check.
    pub fn open(&mut self, id: &str, source: String) {
        self.id = Some(id.to_owned());
        self.source = source;
        self.edited_at = None;
    }

    /// Records an edit made at `now` seconds.
    pub const fn edited(&mut self, now: f64) {
        self.edited_at = Some(now);
    }

    /// What to do at `now`.
    #[must_use]
    pub const fn check(&self, now: f64) -> Check {
        match self.edited_at {
            None => Check::Idle,
            Some(at) if now - at >= CHECK_DELAY => Check::Due,
            Some(at) => Check::Wait(CHECK_DELAY - (now - at)),
        }
    }

    /// Marks the pending check as done.
    pub const fn checked(&mut self) {
        self.edited_at = None;
    }
}

/// A diagnostic placed on a line of the source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineDiagnostic {
    /// 1-based line number.
    pub line: usize,
    pub severity: Severity,
    pub text: String,
}

/// Places each diagnostic on the line its span starts at.
#[must_use]
pub fn line_diagnostics(source: &str, diagnostics: &[Diagnostic]) -> Vec<LineDiagnostic> {
    diagnostics
        .iter()
        .map(|diagnostic| {
            let start = diagnostic.span.start.min(source.len());
            let before = source.get(..start).unwrap_or(source);
            let label = match diagnostic.severity {
                Severity::Error => "Error",
                Severity::Warning => "Warning",
            };
            LineDiagnostic {
                line: before.matches('\n').count() + 1,
                severity: diagnostic.severity,
                text: format!("{label}: {}", diagnostic.message),
            }
        })
        .collect()
}

/// Whether any diagnostic is an error.
#[must_use]
pub fn has_errors(diagnostics: &[Diagnostic]) -> bool {
    diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == Severity::Error)
}

/// Parses a hotkey typed by the user. `Ok(None)` means no hotkey.
#[doc = "# Errors\n\nReturns a message when the text is not a hotkey."]
pub fn parse_hotkey(text: &str) -> Result<Option<String>, String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    Hotkey::from_str(trimmed)
        .map(|_| Some(trimmed.to_owned()))
        .map_err(|_| format!("\"{trimmed}\" is not a hotkey. Try Ctrl+Shift+K."))
}

/// Writes a key press as `Hotkey::from_str` reads it, e.g. `Ctrl+Alt+K`.
#[must_use]
pub fn hotkey_text(modifiers: [bool; 4], key: &str) -> String {
    const NAMES: [&str; 4] = ["Ctrl", "Alt", "Shift", "Cmd"];
    let mut parts: Vec<&str> = NAMES
        .iter()
        .zip(modifiers)
        .filter(|(_, held)| *held)
        .map(|(name, _)| *name)
        .collect();
    parts.push(key);
    parts.join("+")
}

#[cfg(test)]
mod tests {
    use super::{
        Check, EditorText, Fallback, KeyboardRow, TesterText, TypedKey, hotkey_text, keyboard_rows,
        line_diagnostics, matches_filter, parse_hotkey,
    };
    use keymagic_engine::VirtualKey;
    use keymagic_host::HostAction;
    use keymagic_host::config::{Config, Installed};
    use keymagic_host::store::Found;
    use keymagic_kms::{Diagnostic, Severity};
    use std::path::PathBuf;

    fn feed(text: &mut TesterText, actions: &[HostAction], fallback: Fallback) {
        assert_eq!(text.apply(actions, fallback), None);
    }

    fn found(id: &str, name: &str) -> Found {
        Found {
            id: id.to_owned(),
            name: name.to_owned(),
            km2: PathBuf::from(format!("{id}.km2")),
            icon: None,
        }
    }

    #[test]
    fn rows_follow_scan_order_and_config_flags() {
        let mut config = Config::default();
        config.keyboards.active = Some("b".to_owned());
        config.keyboards.installed = vec![Installed {
            id: "a".to_owned(),
            name: "A".to_owned(),
            filename: "a.km2".to_owned(),
            hotkey: None,
            hash: None,
            label: None,
            enabled: false,
        }];
        let rows = keyboard_rows(&config, &[found("a", "A"), found("b", "B")]);
        assert!(!rows[0].enabled);
        assert!(!rows[0].active);
        assert!(rows[1].enabled);
        assert!(rows[1].active);
    }

    #[test]
    fn rows_follow_the_saved_order_and_show_the_label() {
        let entry = |id: &str, label: Option<&str>| Installed {
            id: id.to_owned(),
            name: id.to_uppercase(),
            filename: format!("{id}.km2"),
            hotkey: None,
            hash: None,
            label: label.map(str::to_owned),
            enabled: true,
        };
        let mut config = Config::default();
        config.keyboards.installed = vec![entry("b", Some("Bee")), entry("a", None)];
        let rows = keyboard_rows(
            &config,
            &[found("a", "A"), found("b", "B"), found("c", "C")],
        );
        let ids: Vec<&str> = rows.iter().map(|row| row.id.as_str()).collect();
        assert_eq!(ids, ["b", "a", "c"]);
        assert_eq!(rows[0].name, "Bee");
    }

    #[test]
    fn filter_matches_name_or_id_ignoring_case() {
        let row = KeyboardRow {
            id: "zg".to_owned(),
            name: "Zawgyi".to_owned(),
            icon: None,
            hotkey: None,
            enabled: true,
            active: false,
        };
        assert!(matches_filter(&row, "  ZAW "));
        assert!(matches_filter(&row, "zg"));
        assert!(matches_filter(&row, ""));
        assert!(!matches_filter(&row, "unicode"));
    }

    #[test]
    fn commit_replaces_the_preedit() {
        let mut text = TesterText::default();
        feed(
            &mut text,
            &[HostAction::UpdatePreedit("ka".to_owned())],
            Fallback::Character('x'),
        );
        assert_eq!(text.preedit(), "ka");
        feed(
            &mut text,
            &[HostAction::Commit("က".to_owned())],
            Fallback::Character('x'),
        );
        assert_eq!(text.committed(), "က");
        assert_eq!(text.preedit(), "");
    }

    #[test]
    fn replace_deletes_characters_not_bytes() {
        let mut text = TesterText::default();
        feed(
            &mut text,
            &[HostAction::Commit("ကာ".to_owned())],
            Fallback::Character('x'),
        );
        let replace = HostAction::ReplaceCommitted {
            delete: 1,
            insert: "ါ".to_owned(),
        };
        feed(&mut text, &[replace], Fallback::Character('x'));
        assert_eq!(text.committed(), "ကါ");
    }

    #[test]
    fn pass_uses_the_fallback_and_eat_does_nothing() {
        let mut text = TesterText::default();
        feed(&mut text, &[HostAction::Pass], Fallback::Character('x'));
        feed(&mut text, &[HostAction::Eat], Fallback::Character('y'));
        feed(&mut text, &[HostAction::Pass], Fallback::Backspace);
        assert_eq!(text.committed(), "");
        feed(&mut text, &[HostAction::Pass], Fallback::Character('z'));
        assert_eq!(text.committed(), "z");
    }

    #[test]
    fn switch_is_reported_to_the_caller() {
        let mut text = TesterText::default();
        let switch = HostAction::SwitchKeyboard("next".to_owned());
        assert_eq!(
            text.apply(&[switch], Fallback::Character('x')),
            Some("next".to_owned())
        );
    }

    #[test]
    fn typed_keys_map_to_engine_events() {
        assert_eq!(TypedKey::Backspace.event().key, Some(VirtualKey::Back));
        assert_eq!(TypedKey::Enter.event().key, Some(VirtualKey::Return));
        assert_eq!(TypedKey::Char('q').event().ch, Some('q'));
        assert_eq!(TypedKey::Char('q').fallback(), Fallback::Character('q'));
    }

    #[test]
    fn editor_waits_for_the_quiet_period() {
        let mut editor = EditorText::default();
        editor.open("k", "a".to_owned());
        assert_eq!(editor.check(0.0), Check::Idle);
        editor.edited(1.0);
        assert!(matches!(editor.check(1.05), Check::Wait(wait) if wait > 0.09));
        assert_eq!(editor.check(1.2), Check::Due);
        editor.checked();
        assert_eq!(editor.check(5.0), Check::Idle);
    }

    #[test]
    fn diagnostics_are_placed_on_their_line() {
        let source = "a\nb\n$x\n";
        let diagnostics = [Diagnostic {
            severity: Severity::Error,
            file: String::new(),
            span: 4..6,
            message: "unknown variable".to_owned(),
            help: None,
        }];
        let placed = line_diagnostics(source, &diagnostics);
        assert_eq!(placed[0].line, 3);
        assert_eq!(placed[0].text, "Error: unknown variable");
    }

    #[test]
    fn hotkeys_round_trip_through_the_parser() {
        let text = hotkey_text([true, false, true, false], "K");
        assert_eq!(text, "Ctrl+Shift+K");
        assert_eq!(parse_hotkey(&text), Ok(Some(text)));
        assert_eq!(parse_hotkey("  "), Ok(None));
        assert!(parse_hotkey("definitely not a hotkey").is_err());
    }
}
