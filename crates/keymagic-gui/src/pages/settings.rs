//! Settings page: startup, updates, motion and per-app input modes.

use super::Notice;
use crate::state::{Host, lock};
use egui::{RichText, TextEdit, Ui};
use std::sync::Mutex;

/// View state of the Settings page.
#[derive(Default)]
pub struct View {
    notice: Notice,
    composition_draft: String,
    direct_draft: String,
}

impl View {
    /// Draws the settings. Edits reach `config.toml` only when Save is pressed.
    pub fn show(&mut self, ui: &mut Ui, host: &Mutex<Host>) {
        let mut guard = lock(host);
        let config = guard.config_mut();
        ui.label(RichText::new("Startup and updates").strong());
        ui.checkbox(
            &mut config.general.start_with_system,
            "Start KeyMagic when I sign in",
        );
        ui.checkbox(
            &mut config.general.check_for_updates,
            "Check for updates at startup",
        );
        ui.label(format!("Update channel: {}", config.general.update_channel));
        ui.checkbox(&mut config.general.reduce_motion, "Reduce motion");
        ui.separator();
        ui.label(RichText::new("Per-app input modes").strong());
        app_list(
            ui,
            "Composition mode: text is held until it is committed",
            &mut config.composition_mode.enabled_hosts,
            &mut self.composition_draft,
        );
        app_list(
            ui,
            "Direct mode: each change is typed at once",
            &mut config.direct_mode.enabled_hosts,
            &mut self.direct_draft,
        );
        if ui.button("Save settings").clicked() {
            let message = match guard.save() {
                Ok(()) => "Settings saved.".to_owned(),
                Err(error) => format!("Settings were not saved: {error}"),
            };
            drop(guard);
            self.notice.set(message);
        }
        self.notice.show(ui);
    }
}

/// A list of app names with a remove button per row and an add field.
fn app_list(ui: &mut Ui, title: &str, apps: &mut Vec<String>, draft: &mut String) {
    ui.label(title);
    let mut removed = None;
    for (index, app) in apps.iter().enumerate() {
        ui.horizontal(|ui| {
            ui.label(app.as_str());
            if ui.small_button("Remove").clicked() {
                removed = Some(index);
            }
        });
    }
    if let Some(index) = removed {
        apps.remove(index);
    }
    ui.horizontal(|ui| {
        ui.add(TextEdit::singleline(&mut *draft).hint_text("App name"));
        let name = draft.trim().to_owned();
        if ui.button("Add").clicked() && !name.is_empty() {
            apps.push(name);
            draft.clear();
        }
    });
}
