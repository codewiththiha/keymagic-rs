//! Settings page: startup, updates, motion, per-app modes and the IME panels.

use super::Notice;
use crate::apps;
use crate::integration::{self, Panel};
use crate::state::{Host, lock};
use egui::{Button, ComboBox, RichText, Role, Ui, WidgetInfo};
use std::sync::Mutex;

/// View state of the Settings page.
#[derive(Default)]
pub struct View {
    notice: Notice,
    running: Vec<String>,
    composition_pick: String,
    direct_pick: String,
    connected: Option<bool>,
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
        if ui.button("Load running apps").clicked() {
            self.running = apps::running();
        }
        app_list(
            ui,
            "Composition apps",
            "Composition mode: text is held until it is committed",
            &mut config.composition_mode.enabled_hosts,
            &self.running,
            &mut self.composition_pick,
        );
        app_list(
            ui,
            "Direct apps",
            "Direct mode: each change is typed at once",
            &mut config.direct_mode.enabled_hosts,
            &self.running,
            &mut self.direct_pick,
        );
        ui.separator();
        ui.label(RichText::new("Input method integration").strong());
        if ui.button("Check connection").clicked() {
            self.connected = Some(guard.ime_connected());
        }
        for panel in integration::panels(integration::current(), self.connected) {
            panel_view(ui, &panel);
        }
        ui.separator();
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

fn panel_view(ui: &mut Ui, panel: &Panel) {
    ui.label(RichText::new(panel.title).strong());
    ui.label(panel.status.as_str());
}

/// App names with a remove button per row, and a picker of running apps.
fn app_list(
    ui: &mut Ui,
    label: &str,
    title: &str,
    apps: &mut Vec<String>,
    running: &[String],
    pick: &mut String,
) {
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
    let options = apps::choices(running, apps);
    ui.horizontal(|ui| {
        let picker = ComboBox::from_id_salt(label)
            .selected_text(pick.as_str())
            .show_ui(ui, |ui| {
                for name in &options {
                    ui.selectable_value(pick, name.clone(), name.as_str());
                }
            });
        picker
            .response
            .widget_info(|| WidgetInfo::labeled(Role::ComboBox, true, label));
        if ui
            .add_enabled(!pick.is_empty(), Button::new("Add"))
            .clicked()
        {
            apps.push(std::mem::take(pick));
        }
    });
}
