//! Keyboards page: install, enable, switch and remove keyboards.

use super::Notice;
use crate::model::{KeyboardRow, keyboard_rows};
use crate::state::{Host, lock};
use crate::theme::{self, ROUNDING};
use crate::widgets;
use egui::{Align, Context, CornerRadius, Frame, Label, Layout, Margin, RichText, Sense, Ui};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Extensions the import dialog and drag and drop accept.
const EXTENSIONS: [&str; 2] = ["km2", "kms"];

/// A change requested from a keyboard row.
enum Action {
    Use(String),
    SetEnabled(String, bool),
    Remove(String),
}

/// View state of the Keyboards page.
#[derive(Default)]
pub struct View {
    notice: Notice,
}

impl View {
    /// Draws the toolbar, the keyboard cards and the empty state.
    pub fn show(&mut self, ui: &mut Ui, host: &Mutex<Host>) {
        for path in dropped_paths(ui.ctx()) {
            self.install(host, &path);
        }
        let rows = {
            let guard = lock(host);
            keyboard_rows(guard.config(), guard.keyboards())
        };
        ui.horizontal(|ui| {
            if widgets::primary(ui, "Import…").clicked() {
                self.import_from_dialog(host);
            }
            if widgets::button(ui, "Scan").clicked() {
                self.scan(host);
            }
            if !rows.is_empty() {
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(widgets::muted(ui, &format!("{} installed", rows.len())));
                });
            }
        });
        self.notice.show(ui);
        ui.add_space(16.0);
        if rows.is_empty() {
            widgets::card(ui, |ui| {
                ui.label(RichText::new("No keyboards installed").strong());
                ui.add_space(4.0);
                ui.label(widgets::muted(
                    ui,
                    "Import a .km2 or .kms file, or drop one onto this window.",
                ));
            });
            return;
        }
        let mut requested = None;
        for row in &rows {
            requested = keyboard_card(ui, row).or(requested);
            ui.add_space(8.0);
        }
        if let Some(action) = requested {
            self.apply(host, action);
        }
    }

    fn import_from_dialog(&mut self, host: &Mutex<Host>) {
        let picked = rfd::FileDialog::new()
            .add_filter("KeyMagic keyboards", &EXTENSIONS)
            .pick_files();
        for path in picked.unwrap_or_default() {
            self.install(host, &path);
        }
    }

    fn scan(&mut self, host: &Mutex<Host>) {
        let mut guard = lock(host);
        let message = match guard.rescan() {
            Ok(()) => format!("Found {} keyboards.", guard.keyboards().len()),
            Err(error) => format!("Could not read the keyboard folder: {error}"),
        };
        drop(guard);
        self.notice.set(message);
    }

    fn install(&mut self, host: &Mutex<Host>, path: &Path) {
        let mut guard = lock(host);
        let message = match guard.import(path) {
            Ok(id) => match guard.save() {
                Ok(()) => format!("Installed {id}."),
                Err(error) => format!("Installed {id}, but settings were not saved: {error}"),
            },
            Err(error) => format!("Could not install {}: {error}", path.display()),
        };
        drop(guard);
        self.notice.set(message);
    }

    fn apply(&mut self, host: &Mutex<Host>, action: Action) {
        let mut guard = lock(host);
        let message = match action {
            Action::Use(id) => {
                if guard.use_keyboard(&id) {
                    format!("{id} is now the active keyboard.")
                } else {
                    format!("{id} is not installed.")
                }
            }
            Action::SetEnabled(id, enabled) => {
                if guard.set_enabled(&id, enabled) {
                    let state = if enabled { "enabled" } else { "disabled" };
                    format!("{id} is {state}.")
                } else {
                    format!("{id} is not installed.")
                }
            }
            Action::Remove(id) => match guard.remove(&id) {
                Ok(true) => format!("Removed {id}."),
                Ok(false) => format!("{id} is not installed."),
                Err(error) => format!("Could not remove {id}: {error}"),
            },
        };
        let saved = guard.save();
        drop(guard);
        self.notice.set(match saved {
            Ok(()) => message,
            Err(error) => format!("{message} Settings were not saved: {error}"),
        });
    }
}

/// Keyboard files dropped onto the window this frame.
fn dropped_paths(ctx: &Context) -> Vec<PathBuf> {
    ctx.input(|input| {
        input
            .raw
            .dropped_files
            .iter()
            .map(|file| file.path().to_path_buf())
            .filter(|path| has_keyboard_extension(path))
            .collect()
    })
}

fn has_keyboard_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| EXTENSIONS.contains(&ext.to_ascii_lowercase().as_str()))
}

/// One keyboard card; returns the change the user asked for, if any.
fn keyboard_card(ui: &mut Ui, row: &KeyboardRow) -> Option<Action> {
    let mut requested = None;
    let fill = if row.active {
        ui.visuals().widgets.inactive.weak_bg_fill
    } else {
        ui.visuals().window_fill
    };
    let frame = Frame::new()
        .fill(fill)
        .corner_radius(CornerRadius::same(ROUNDING))
        .inner_margin(Margin::symmetric(16, 12));
    frame.show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                let name =
                    ui.add(Label::new(RichText::new(&row.name).strong()).sense(Sense::click()));
                name.context_menu(|ui| {
                    if widgets::button(ui, "Use this keyboard").clicked() {
                        requested = Some(Action::Use(row.id.clone()));
                        ui.close();
                    }
                    let toggle = if row.enabled { "Disable" } else { "Enable" };
                    if widgets::button(ui, toggle).clicked() {
                        requested = Some(Action::SetEnabled(row.id.clone(), !row.enabled));
                        ui.close();
                    }
                    if widgets::button(ui, "Remove").clicked() {
                        requested = Some(Action::Remove(row.id.clone()));
                        ui.close();
                    }
                });
                ui.label(widgets::muted(ui, &row.id).small());
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let mut enabled = row.enabled;
                let label = format!("Enable {}", row.name);
                if widgets::switch(ui, &mut enabled, &label).changed() {
                    requested = Some(Action::SetEnabled(row.id.clone(), enabled));
                }
                let state = if row.enabled { "Enabled" } else { "Disabled" };
                ui.label(widgets::muted(ui, state));
                if row.active {
                    ui.label(RichText::new("In use").color(theme::accent_color()));
                } else if widgets::button(ui, "Use").clicked() {
                    requested = Some(Action::Use(row.id.clone()));
                }
            });
        });
    });
    requested
}
