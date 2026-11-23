//! Settings page: startup, updates, motion, per-app modes and the IME panels.

use super::Notice;
use crate::apps;
use crate::integration::{self, Health, Panel};
use crate::reveal;
use crate::state::{Host, lock};
use crate::theme;
use crate::widgets;
use egui::{Align, Button, ComboBox, Layout, RichText, Role, Ui, WidgetInfo};
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
    fn open_settings(&mut self, os: integration::Os) {
        let Some(uri) = integration::settings_uri(os) else {
            return;
        };
        if let Err(error) = reveal::open_uri(uri) {
            self.notice
                .set(format!("Could not open the input settings: {error}"));
        }
    }

    /// Draws the settings. Edits reach `config.toml` only when Save is pressed.
    pub fn show(&mut self, ui: &mut Ui, host: &Mutex<Host>) {
        let mut guard = lock(host);
        ui.horizontal(|ui| {
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if widgets::primary(ui, "Save settings").clicked() {
                    let message = match guard.save() {
                        Ok(()) => "Settings saved.".to_owned(),
                        Err(error) => format!("Settings were not saved: {error}"),
                    };
                    self.notice.set(message);
                }
            });
        });
        self.notice.show(ui);
        ui.add_space(16.0);
        let config = guard.config_mut();
        widgets::section(ui, "Startup and updates", |ui| {
            widgets::setting(
                ui,
                "Start KeyMagic when I sign in",
                "Runs in the tray after you sign in.",
                &mut config.general.start_with_system,
            );
            ui.add_space(16.0);
            widgets::setting(
                ui,
                "Check for updates at startup",
                &format!("Update channel: {}.", config.general.update_channel),
                &mut config.general.check_for_updates,
            );
        });
        widgets::section(ui, "Motion", |ui| {
            widgets::setting(
                ui,
                "Reduce motion",
                "Turns off window animations.",
                &mut config.general.reduce_motion,
            );
        });
        widgets::section(ui, "Feedback", |ui| {
            widgets::setting(
                ui,
                "Keyboard HUD",
                "Shows the keyboard name when you switch keyboards.",
                &mut config.general.show_hud,
            );
        });
        widgets::section(ui, "Per-app input modes", |ui| {
            if widgets::button(ui, "Load running apps").clicked() {
                self.running = apps::running();
            }
            ui.add_space(8.0);
            app_list(
                ui,
                "Composition apps",
                "Text is held until it is committed.",
                &mut config.composition_mode.enabled_hosts,
                &self.running,
                &mut self.composition_pick,
            );
            ui.add_space(16.0);
            app_list(
                ui,
                "Direct apps",
                "Each change is typed at once.",
                &mut config.direct_mode.enabled_hosts,
                &self.running,
                &mut self.direct_pick,
            );
        });
        widgets::section(ui, "Input method integration", |ui| {
            if widgets::button(ui, "Check connection").clicked() {
                self.connected = Some(guard.ime_connected());
            }
            ui.add_space(8.0);
            for panel in integration::panels(integration::current(), self.connected) {
                if panel_view(ui, &panel) {
                    self.open_settings(panel.os);
                }
                ui.add_space(8.0);
            }
        });
        drop(guard);
    }
}

/// One integration panel; returns true when its settings button is clicked.
fn panel_view(ui: &mut Ui, panel: &Panel) -> bool {
    ui.horizontal(|ui| {
        status_dot(ui, panel.health);
        ui.label(RichText::new(panel.title).strong());
    });
    let mut open = false;
    ui.horizontal(|ui| {
        ui.label(widgets::muted(ui, panel.status.as_str()));
        if integration::settings_uri(panel.os).is_some()
            && panel.health == Health::Missing
            && widgets::button(ui, "Open settings").clicked()
        {
            open = true;
        }
    });
    open
}

fn status_dot(ui: &mut Ui, health: Health) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(8.0, 8.0), egui::Sense::hover());
    let color = match health {
        Health::Ready => crate::theme::accent_color(),
        Health::Missing => crate::theme::color(crate::theme::ERROR),
        Health::Unchecked | Health::Elsewhere => crate::theme::muted_color(ui.visuals()),
    };
    ui.painter().circle_filled(rect.center(), 4.0, color);
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
    ui.label(RichText::new(label).strong());
    ui.label(widgets::muted(ui, title).small());
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
        widgets::focus_ring(ui, &picker.response, theme::CONTROL_ROUNDING);
        let add = ui.add_enabled(!pick.is_empty(), Button::new("Add"));
        widgets::focus_ring(ui, &add, theme::CONTROL_ROUNDING);
        if add.clicked() {
            apps.push(std::mem::take(pick));
        }
    });
}
