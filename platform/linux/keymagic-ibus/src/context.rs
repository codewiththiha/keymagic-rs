//! One `IBus` input context: key handling and the signals it emits.

use crate::{input, menu, text};
use keymagic_host::integration::ibus::engines;
use keymagic_host::session::HostAction;
use keymagic_host::{Config, KeyboardStore, Paths, Session};
use std::path::PathBuf;
use std::sync::Arc;
use zbus::Connection;
use zbus::object_server::SignalEmitter;
use zbus::{ObjectPath, Variant};

/// Engine state for one `IBus` input context.
pub struct Context {
    session: Session,
    store: Arc<KeyboardStore>,
    config_path: PathBuf,
    path: ObjectPath<'static>,
    keyboard: String,
}

impl Context {
    /// Starts a context for the engine name, such as `keymagic:unicode`.
    pub fn load(
        store: Arc<KeyboardStore>,
        config_path: PathBuf,
        engine: &str,
        path: ObjectPath<'static>,
    ) -> Result<Self, String> {
        let config = Config::load_or_default(&config_path).map_err(|error| error.to_string())?;
        let id = keyboard_id(engine, config.keyboards.active.as_deref())
            .ok_or("no keyboard is active")?;
        let session = session_for(&store, &config, &id)?;
        Ok(Self {
            session,
            store,
            config_path,
            path,
            keyboard: id,
        })
    }

    async fn switch(&mut self, id: &str, emitter: &SignalEmitter<'_>) -> zbus::Result<()> {
        let config = Config::load_or_default(&self.config_path)
            .map_err(|error| zbus::Error::Failure(error.to_string()))?;
        self.session = session_for(&self.store, &config, id).map_err(zbus::Error::Failure)?;
        self.session.focus_in("");
        id.clone_into(&mut self.keyboard);
        self.register_menu(emitter).await
    }

    /// Shows the keyboard menu, then starts composing in the field.
    async fn engage(&mut self, emitter: &SignalEmitter<'_>) -> zbus::fdo::Result<()> {
        self.session.focus_in("");
        self.register_menu(emitter)
            .await
            .map_err(|error| zbus::fdo::Error::Failed(error.to_string()))
    }

    async fn register_menu(&self, emitter: &SignalEmitter<'_>) -> zbus::Result<()> {
        let config = Config::load_or_default(&self.config_path)
            .map_err(|error| zbus::Error::Failure(error.to_string()))?;
        let paths = Paths::for_user()
            .ok_or_else(|| zbus::Error::Failure("no user folders for this account".to_owned()))?;
        let menu = menu::menu(&engines(&config, &paths), &self.keyboard)?;
        Self::register_properties(emitter, menu).await
    }

    async fn apply(&mut self, action: HostAction, emitter: &SignalEmitter<'_>) -> zbus::Result<()> {
        match action {
            HostAction::Pass | HostAction::Eat | HostAction::Hud(_) => Ok(()),
            HostAction::Commit(text) => Self::commit_text(emitter, text::ibus_text(&text)).await,
            HostAction::UpdatePreedit(text) => preedit(emitter, &text).await,
            HostAction::ReplaceCommitted { delete, insert } => {
                let offset = 0_i32.saturating_sub(i32::try_from(delete).unwrap_or(i32::MAX));
                let count = u32::try_from(delete).unwrap_or(u32::MAX);
                Self::delete_surrounding_text(emitter, offset, count).await?;
                Self::commit_text(emitter, text::ibus_text(&insert)).await
            }
            HostAction::SwitchKeyboard(id) => self.switch(&id, emitter).await,
        }
    }

    async fn finish(&mut self, emitter: &SignalEmitter<'_>) -> zbus::Result<()> {
        match self.session.focus_out() {
            Some(action) => self.apply(action, emitter).await,
            None => Ok(()),
        }
    }
}

#[zbus::interface(name = "org.freedesktop.IBus.Engine")]
impl Context {
    /// Handles one key press and returns true when the engine consumed it.
    async fn process_key_event(
        &mut self,
        keyval: u32,
        keycode: u32,
        state: u32,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> zbus::fdo::Result<bool> {
        if input::is_release(state) {
            return Ok(false);
        }
        let actions = self.session.key(input::key_event(keyval, keycode, state));
        let consumed = consumes(&actions);
        for action in actions {
            self.apply(action, &emitter)
                .await
                .map_err(|error| zbus::fdo::Error::Failed(error.to_string()))?;
        }
        Ok(consumed)
    }

    /// Starts composing in a newly focused field.
    async fn focus_in(
        &mut self,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> zbus::fdo::Result<()> {
        self.engage(&emitter).await
    }

    /// Starts composing when `IBus` enables this engine.
    async fn enable(
        &mut self,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> zbus::fdo::Result<()> {
        self.engage(&emitter).await
    }

    /// Commits pending text when `IBus` disables this engine.
    async fn disable(
        &mut self,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> zbus::fdo::Result<()> {
        self.finish(&emitter)
            .await
            .map_err(|error| zbus::fdo::Error::Failed(error.to_string()))
    }

    /// Accepts client capabilities; this engine needs none of them.
    const fn set_capabilities(&self, capabilities: u32) {
        let _ = (self, capabilities);
    }

    /// Accepts the caret position; this engine draws no candidate window.
    const fn set_cursor_location(&self, x: i32, y: i32, width: i32, height: i32) {
        let _ = (self, x, y, width, height);
    }

    /// Accepts the panel's extension keys; this engine binds none of them.
    fn panel_extension_register_keys(&self, data: Variant<'_>) {
        let _ = (self, data);
    }

    /// Switches to the keyboard whose menu option was checked.
    async fn property_activate(
        &mut self,
        name: &str,
        state: u32,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> zbus::fdo::Result<()> {
        let Some(id) = name.strip_prefix(menu::KEY_PREFIX) else {
            return Ok(());
        };
        if state != menu::CHECKED {
            return Ok(());
        }
        self.switch(id, &emitter)
            .await
            .map_err(|error| zbus::fdo::Error::Failed(error.to_string()))
    }

    /// Commits pending text when the field loses focus.
    async fn focus_out(
        &mut self,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> zbus::fdo::Result<()> {
        self.finish(&emitter)
            .await
            .map_err(|error| zbus::fdo::Error::Failed(error.to_string()))
    }

    /// Commits pending text when the application resets the context.
    async fn reset(
        &mut self,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> zbus::fdo::Result<()> {
        self.finish(&emitter)
            .await
            .map_err(|error| zbus::fdo::Error::Failed(error.to_string()))
    }

    /// Drops pending text and removes this context from the bus.
    fn destroy(&mut self, #[zbus(connection)] connection: &Connection) {
        self.session.focus_out();
        let connection = connection.clone();
        let path = self.path.clone();
        tokio::spawn(async move {
            if let Err(error) = connection.object_server().remove::<Self, _>(path).await {
                eprintln!("error: cannot remove context: {error}");
            }
        });
    }

    #[zbus(signal)]
    async fn commit_text(emitter: &SignalEmitter<'_>, text: Variant<'_>) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn register_properties(
        emitter: &SignalEmitter<'_>,
        props: Variant<'_>,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn update_preedit_text(
        emitter: &SignalEmitter<'_>,
        text: Variant<'_>,
        cursor: u32,
        visible: bool,
        mode: u32,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn hide_preedit_text(emitter: &SignalEmitter<'_>) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn delete_surrounding_text(
        emitter: &SignalEmitter<'_>,
        offset: i32,
        count: u32,
    ) -> zbus::Result<()>;
}

/// Keyboard id for an engine name such as `keymagic:unicode`.
#[must_use]
pub fn keyboard_id(engine: &str, active: Option<&str>) -> Option<String> {
    engine
        .strip_prefix("keymagic:")
        .filter(|id| !id.is_empty())
        .or(active)
        .map(str::to_owned)
}

/// True when any action means the engine handled the key.
fn consumes(actions: &[HostAction]) -> bool {
    actions
        .iter()
        .any(|action| !matches!(action, HostAction::Pass | HostAction::Hud(_)))
}

fn session_for(store: &KeyboardStore, config: &Config, id: &str) -> Result<Session, String> {
    let keyboard = store
        .load(id)
        .map_err(|error| format!("cannot load {id}: {error}"))?;
    let engine =
        keymagic_engine::Engine::new(Arc::new(keyboard)).map_err(|error| error.to_string())?;
    Ok(Session::new(engine, config))
}

/// `IBus` preedit focus mode that drops the text when focus leaves.
const PREEDIT_CLEAR: u32 = 0;

async fn preedit(emitter: &SignalEmitter<'_>, text: &str) -> zbus::Result<()> {
    if text.is_empty() {
        return Context::hide_preedit_text(emitter).await;
    }
    let cursor = u32::try_from(text.chars().count()).unwrap_or(u32::MAX);
    Context::update_preedit_text(emitter, text::ibus_text(text), cursor, true, PREEDIT_CLEAR).await
}

#[cfg(test)]
mod tests {
    use super::{consumes, keyboard_id};
    use keymagic_host::session::HostAction;

    #[test]
    fn engine_name_selects_the_keyboard() {
        assert_eq!(
            keyboard_id("keymagic:unicode", Some("zawgyi")).as_deref(),
            Some("unicode")
        );
        assert_eq!(
            keyboard_id("keymagic", Some("zawgyi")).as_deref(),
            Some("zawgyi")
        );
        assert_eq!(keyboard_id("keymagic:", None), None);
    }

    #[test]
    fn pass_and_hud_do_not_consume_the_key() {
        assert!(!consumes(&[HostAction::Pass]));
        assert!(!consumes(&[HostAction::Hud("Unicode".to_owned())]));
        assert!(consumes(&[HostAction::Eat]));
        assert!(consumes(&[HostAction::Commit("က".to_owned())]));
    }
}
