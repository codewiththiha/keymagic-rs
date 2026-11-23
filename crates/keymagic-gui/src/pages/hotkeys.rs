//! Hotkeys page: the next-keyboard shortcut, recorded or typed.

use super::Notice;
use crate::model::{hotkey_text, parse_hotkey};
use crate::state::{Host, lock};
use crate::theme;
use crate::widgets;
use egui::{Event, TextEdit, Ui};
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
        widgets::section(ui, "Keyboard shortcuts", |ui| {
            widgets::setting(
                ui,
                "Use keyboard shortcuts",
                "Lets the shortcut below switch keyboards.",
                &mut config.hotkeys.enabled,
            );
        });
        let conflicts: Vec<String> = config
            .hotkey_conflicts()
            .into_iter()
            .map(|(first, second)| format!("{first} and {second} use the same shortcut."))
            .collect();
        widgets::section(ui, "Next keyboard", |ui| {
            ui.horizontal(|ui| {
                ui.add(
                    TextEdit::singleline(&mut self.draft)
                        .hint_text("Ctrl+Shift+K")
                        .desired_width(240.0),
                );
                let label = if self.recording {
                    "Press keys…"
                } else {
                    "Record"
                };
                if widgets::button(ui, label).clicked() {
                    self.recording = !self.recording;
                }
            });
            if self.recording {
                self.record(ui);
            }
            if let Err(message) = parse_hotkey(&self.draft) {
                ui.colored_label(theme::color(theme::ERROR), message);
            }
            ui.add_space(8.0);
            ui.label(widgets::muted(
                ui,
                "Choose Record, then press the keys. Leave the box empty for no shortcut.",
            ));
        });
        widgets::section(ui, "Conflicts", |ui| {
            if conflicts.is_empty() {
                ui.label(widgets::muted(ui, "No keyboards share a shortcut."));
            }
            for conflict in &conflicts {
                ui.colored_label(theme::color(theme::ERROR), conflict.as_str());
            }
        });
        if widgets::primary(ui, "Save shortcut").clicked() {
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
