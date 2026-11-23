//! Tester page: type with the active keyboard and see what it produces.

use super::Notice;
use crate::model::{TesterText, TypedKey};
use crate::state::{Host, lock};
use crate::widgets;
use egui::{Align, Context, Event, Key, Layout, RichText, Ui};
use keymagic_engine::Engine;
use keymagic_host::Session;
use std::sync::{Arc, Mutex};

/// View state of the Tester page: its session and typed text.
#[derive(Default)]
pub struct View {
    keyboard: Option<String>,
    session: Option<Session>,
    text: TesterText,
    notice: Notice,
}

impl View {
    /// The committed text, for tests and status display.
    #[must_use]
    pub const fn text(&self) -> &TesterText {
        &self.text
    }

    /// Draws the output area and sends typed keys to the session.
    pub fn show(&mut self, ui: &mut Ui, host: &Mutex<Host>) {
        self.sync_session(host);
        self.capture(ui, host);
        ui.horizontal(|ui| {
            let status = self.keyboard.as_ref().map_or_else(
                || "No keyboard is active. Choose one on the Keyboards page.".to_owned(),
                |id| format!("Keyboard: {id}"),
            );
            ui.label(widgets::muted(ui, &status));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if widgets::button(ui, "Clear").clicked() {
                    self.clear();
                }
            });
        });
        self.notice.show(ui);
        ui.add_space(8.0);
        widgets::card(ui, |ui| {
            ui.set_min_size(egui::vec2(ui.available_width(), 200.0));
            ui.label(RichText::new(self.text.committed()).size(28.0));
            ui.label(widgets::muted(ui, self.text.preedit()).size(28.0));
        });
        ui.add_space(8.0);
        ui.label(widgets::muted(
            ui,
            "Click the page and type. Keys go to the active keyboard.",
        ));
    }

    /// Reloads the session when the active keyboard changes.
    fn sync_session(&mut self, host: &Mutex<Host>) {
        let active = lock(host).config().keyboards.active.clone();
        if active == self.keyboard {
            return;
        }
        self.keyboard.clone_from(&active);
        self.session = None;
        self.text.clear();
        let Some(id) = active else {
            return;
        };
        let opened = open_session(&lock(host), &id);
        match opened {
            Ok(session) => self.session = Some(session),
            Err(error) => self.notice.set(format!("Could not load {id}: {error}")),
        }
    }

    fn capture(&mut self, ui: &Ui, host: &Mutex<Host>) {
        if ui.ctx().memory(|memory| memory.focused().is_some()) {
            return;
        }
        for key in typed_keys(ui.ctx()) {
            let Some(session) = self.session.as_mut() else {
                return;
            };
            let actions = session.key(key.event());
            if let Some(id) = self.text.apply(&actions, key.fallback()) {
                self.switch(host, &id);
            }
        }
    }

    fn clear(&mut self) {
        self.text.clear();
        if let Some(session) = self.session.as_mut() {
            session.sync("");
        }
    }

    fn switch(&mut self, host: &Mutex<Host>, id: &str) {
        let result = lock(host).switch_keyboard(id);
        if let Err(error) = result {
            self.notice.set(format!(
                "Switched to {id}, but settings were not saved: {error}"
            ));
        }
    }
}

fn open_session(host: &Host, id: &str) -> Result<Session, String> {
    let keyboard = host.load_keyboard(id).map_err(|error| error.to_string())?;
    let engine = Engine::new(Arc::new(keyboard)).map_err(|error| error.message.to_owned())?;
    let mut session = Session::new(engine, host.config());
    session.focus_in("KeyMagic Tester");
    Ok(session)
}

/// The printable characters, Backspace and Enter typed this frame.
fn typed_keys(ctx: &Context) -> Vec<TypedKey> {
    ctx.input(|input| {
        // Shortcuts such as Ctrl+1 send no text; keep Alt for AltGr.
        let shortcut = input.modifiers.command && !input.modifiers.alt;
        let mut keys = Vec::new();
        for event in &input.events {
            match event {
                Event::Text(text) if !shortcut => keys.extend(text.chars().map(TypedKey::Char)),
                Event::Key {
                    key: Key::Backspace,
                    pressed: true,
                    ..
                } => keys.push(TypedKey::Backspace),
                Event::Key {
                    key: Key::Enter,
                    pressed: true,
                    ..
                } => keys.push(TypedKey::Enter),
                _ => {}
            }
        }
        keys
    })
}
