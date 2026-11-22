//! Commit, mode and hotkey rules. Adapters apply the returned actions.

use crate::config::Config;
use keymagic_engine::{Action, Engine, KeyEvent, Output, VirtualKey};
use keymagic_types::Hotkey;

/// Where typed text goes: a preedit, or committed text at once.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Text stays in a preedit until a commit key or a focus change.
    Composition,
    /// Each engine edit is applied to committed text immediately.
    Direct,
}

/// One instruction for the platform adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostAction {
    /// Let the application handle the key.
    Pass,
    /// Swallow the key because the engine used it.
    Eat,
    /// Commit this text into the application.
    Commit(String),
    /// Show this text as the preedit.
    UpdatePreedit(String),
    /// Delete `delete` characters before the caret, then insert `insert`.
    ReplaceCommitted { delete: usize, insert: String },
    /// Load the keyboard with this id.
    SwitchKeyboard(String),
    /// Show this text in the on-screen indicator.
    Hud(String),
}

/// A hotkey that switches to another keyboard.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Switch {
    hotkey: Hotkey,
    keyboard: String,
    label: String,
}

impl Switch {
    fn matches(&self, event: &KeyEvent) -> bool {
        // shortcut: no Meta in KeyEvent, so Meta hotkeys never match yet.
        let hotkey = self.hotkey;
        !hotkey.meta
            && event.key == Some(hotkey.key)
            && event.mods.ctrl == hotkey.ctrl
            && event.mods.alt == hotkey.alt
            && event.mods.shift == hotkey.shift
    }
}

/// Applications that fight composition unless the config says otherwise.
const DIRECT_DEFAULTS: &[&str] = &[
    "code",
    "code-insiders",
    "alacritty",
    "gnome-terminal",
    "iterm2",
    "kitty",
    "konsole",
    "terminal",
    "windowsterminal",
    "xterm",
];

/// Engine plus host policy for one input context.
#[derive(Debug)]
pub struct Session {
    engine: Engine,
    composition: Vec<String>,
    direct: Vec<String>,
    switches: Vec<Switch>,
    mode: Mode,
}

impl Session {
    /// Builds a session from `config`, in composition mode until `focus_in`.
    #[must_use]
    pub fn new(engine: Engine, config: &Config) -> Self {
        Self {
            engine,
            composition: normalized(&config.composition_mode.enabled_hosts),
            direct: normalized(&config.direct_mode.enabled_hosts),
            switches: switches(config),
            mode: Mode::Composition,
        }
    }

    /// Mode chosen by the last `focus_in`.
    #[must_use]
    pub const fn mode(&self) -> Mode {
        self.mode
    }

    /// Text the engine holds but has not committed.
    #[must_use]
    pub fn composing(&self) -> &str {
        self.engine.composing()
    }

    /// Starts an input context. Explicit lists win over built-in defaults.
    pub fn focus_in(&mut self, app: &str) {
        let app = normalize(app);
        let direct = !self.composition.contains(&app)
            && (self.direct.contains(&app) || DIRECT_DEFAULTS.contains(&app.as_str()));
        self.mode = if direct {
            Mode::Direct
        } else {
            Mode::Composition
        };
        self.engine.reset();
    }

    /// Ends the input context. Returns the commit for pending preedit text.
    pub fn focus_out(&mut self) -> Option<HostAction> {
        let pending = self.pending();
        self.engine.reset();
        pending.map(HostAction::Commit)
    }

    /// Replaces the engine text after the host changed the caret or the text.
    pub fn sync(&mut self, text: &str) {
        self.engine.set_composing(text);
    }

    /// Handles one key press. Switch hotkeys are checked before the engine.
    pub fn key(&mut self, event: KeyEvent) -> Vec<HostAction> {
        let hit = self
            .switches
            .iter()
            .find(|switch| switch.matches(&event))
            .cloned();
        if let Some(switch) = hit {
            let mut actions: Vec<HostAction> =
                self.pending().map(HostAction::Commit).into_iter().collect();
            self.engine.reset();
            actions.push(HostAction::SwitchKeyboard(switch.keyboard));
            actions.push(HostAction::Hud(switch.label));
            return actions;
        }
        let before = self.engine.composing().to_owned();
        let output = self.engine.process(event);
        let trigger = Trigger::of(event.key);
        match self.mode {
            Mode::Composition => self.composition(trigger, &output, &before),
            Mode::Direct => self.direct(trigger, &output),
        }
    }

    fn composition(
        &mut self,
        trigger: Option<Trigger>,
        output: &Output,
        before: &str,
    ) -> Vec<HostAction> {
        let text = output.composing.clone();
        match trigger {
            Some(Trigger::Escape) if !before.is_empty() => {
                self.engine.reset();
                vec![HostAction::UpdatePreedit(String::new()), HostAction::Eat]
            }
            Some(Trigger::Enter | Trigger::Tab) if !text.is_empty() => {
                vec![self.commit(text), HostAction::Pass]
            }
            Some(Trigger::Space) if output.consumed && text.ends_with(' ') => {
                vec![self.commit(text), HostAction::Eat]
            }
            Some(Trigger::Space) if !output.consumed && !text.is_empty() => {
                vec![self.commit(format!("{text} ")), HostAction::Eat]
            }
            _ if output.consumed => vec![HostAction::UpdatePreedit(text), HostAction::Eat],
            _ if !text.is_empty() => vec![self.commit(text), HostAction::Pass],
            _ => vec![HostAction::Pass],
        }
    }

    fn direct(&mut self, trigger: Option<Trigger>, output: &Output) -> Vec<HostAction> {
        let action = match &output.action {
            Action::None if output.consumed => HostAction::Eat,
            Action::None => HostAction::Pass,
            Action::Insert(insert) => HostAction::ReplaceCommitted {
                delete: 0,
                insert: insert.clone(),
            },
            Action::Delete(delete) => HostAction::ReplaceCommitted {
                delete: *delete,
                insert: String::new(),
            },
            Action::Replace { delete, insert } => HostAction::ReplaceCommitted {
                delete: *delete,
                insert: insert.clone(),
            },
        };
        let ends_context = match trigger {
            Some(Trigger::Enter | Trigger::Tab | Trigger::Escape) => true,
            Some(Trigger::Space) => !output.consumed || output.composing.ends_with(' '),
            None => false,
        };
        if ends_context {
            self.engine.reset();
        }
        vec![action]
    }

    fn commit(&mut self, text: String) -> HostAction {
        self.engine.reset();
        HostAction::Commit(text)
    }

    fn pending(&self) -> Option<String> {
        let text = self.engine.composing();
        (self.mode == Mode::Composition && !text.is_empty()).then(|| text.to_owned())
    }
}

/// Keys that end a composition or clear it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Trigger {
    Space,
    Enter,
    Tab,
    Escape,
}

impl Trigger {
    fn of(key: Option<VirtualKey>) -> Option<Self> {
        match key? {
            VirtualKey::Space => Some(Self::Space),
            VirtualKey::Return => Some(Self::Enter),
            VirtualKey::Tab => Some(Self::Tab),
            VirtualKey::Escape => Some(Self::Escape),
            _ => None,
        }
    }
}

fn switches(config: &Config) -> Vec<Switch> {
    config
        .keyboards
        .installed
        .iter()
        .filter(|keyboard| keyboard.enabled)
        .filter_map(|keyboard| {
            let hotkey = keyboard.hotkey.as_deref()?.parse().ok()?;
            Some(Switch {
                hotkey,
                keyboard: keyboard.id.clone(),
                label: keyboard.name.clone(),
            })
        })
        .collect()
}

fn normalized(apps: &[String]) -> Vec<String> {
    apps.iter().map(|app| normalize(app)).collect()
}

/// Lower-case name without `.exe`, so matching ignores case and suffix.
fn normalize(app: &str) -> String {
    let lower = app.trim().to_ascii_lowercase();
    lower.strip_suffix(".exe").unwrap_or(&lower).to_owned()
}
