//! Hotkeys page: the next-keyboard shortcut, recorded or typed.

use super::Notice;
use crate::model::{hotkey_text, parse_hotkey};
use crate::state::{Host, lock};
use egui::{Color32, Event, RichText, TextEdit, Ui};
use std::sync::Mutex;

/// View state of the Hotkeys page.
#[derive(Default)]
pub struct View {
    loaded: bool,
    draft: String,
    recording: bool,
    notice: Notice,
}

impl View {
    /// Draws the shortcut editor, its validation and the conflicts.
    pub fn show(&mut self, ui: &mut Ui, host: &Mutex<Host>) {
        let mut guard = lock(host);
        if !self.loaded {
            self.draft = guard
                .config()
                .hotkeys
                .next_keyboard
                .clone()
                .unwrap_or_default();
            self.loaded = true;
        }
        let config = guard.config_mut();
        ui.checkbox(&mut config.hotkeys.enabled, "Use keyboard shortcuts");
        ui.label(RichText::new("Next keyboard").strong());
        ui.horizontal(|ui| {
            ui.add(TextEdit::singleline(&mut self.draft).hint_text("Ctrl+Shift+K"));
            let label = if self.recording {
                "Press keys…"
            } else {
                "Record"
            };
            if ui.button(label).clicked() {
                self.recording = !self.recording;
            }
        });
        if self.recording {
            self.record(ui);
        }
        if let Err(message) = parse_hotkey(&self.draft) {
            ui.colored_label(Color32::from_rgb(0xE5, 0x48, 0x4D), message);
        }
        let conflicts: Vec<String> = config
            .hotkey_conflicts()
            .into_iter()
            .map(|(first, second)| format!("{first} and {second} use the same shortcut."))
            .collect();
        ui.separator();
        ui.label(RichText::new("Conflicts").strong());
        if conflicts.is_empty() {
            ui.label("No keyboards share a shortcut.");
        }
        for conflict in &conflicts {
            ui.colored_label(crate::theme::accent_color(), conflict.as_str());
        }
        if ui.button("Save shortcut").clicked() {
            self.save(&mut guard);
        }
        drop(guard);
        self.notice.show(ui);
    }

    /// Copies the next key press, with its modifiers, into the draft.
    fn record(&mut self, ui: &Ui) {
        let captured = ui.input(|input| {
            input.events.iter().find_map(|event| match event {
                Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    ..
                } => Some((*key, *modifiers)),
                _ => None,
            })
        });
        if let Some((key, modifiers)) = captured {
            let held = [
                modifiers.ctrl,
                modifiers.alt,
                modifiers.shift,
                modifiers.mac_cmd,
            ];
            self.draft = hotkey_text(held, key.name());
            self.recording = false;
        }
    }

    fn save(&mut self, guard: &mut Host) {
        let parsed = match parse_hotkey(&self.draft) {
            Ok(parsed) => parsed,
            Err(message) => {
                self.notice.set(message);
                return;
            }
        };
        guard.config_mut().hotkeys.next_keyboard = parsed;
        let message = match guard.save() {
            Ok(()) => "Shortcut saved.".to_owned(),
            Err(error) => format!("Shortcut was not saved: {error}"),
        };
        self.notice.set(message);
    }
}
