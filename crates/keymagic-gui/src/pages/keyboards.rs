//! Keyboards page: install, enable, switch and remove keyboards.

use super::Notice;
use crate::model::{KeyboardRow, keyboard_rows};
use crate::state::{Host, lock};
use egui::{Context, RichText, Ui};
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
        ui.horizontal(|ui| {
            if ui.button("Import…").clicked() {
                self.import_from_dialog(host);
            }
            if ui.button("Scan").clicked() {
                self.scan(host);
            }
        });
        self.notice.show(ui);
        let rows = {
            let guard = lock(host);
            keyboard_rows(guard.config(), guard.keyboards())
        };
        if rows.is_empty() {
            ui.add_space(16.0);
            ui.label("No keyboards installed yet.");
            ui.label("Import a .km2 or .kms file, or drop one onto this window.");
            return;
        }
        let mut requested = None;
        for row in &rows {
            requested = card(ui, row).or(requested);
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

/// One keyboard card. Returns the change the user asked for, if any.
fn card(ui: &mut Ui, row: &KeyboardRow) -> Option<Action> {
    let mut requested = None;
    let response = ui
        .horizontal(|ui| {
            let mut enabled = row.enabled;
            if ui.checkbox(&mut enabled, "On").changed() {
                requested = Some(Action::SetEnabled(row.id.clone(), enabled));
            }
            ui.label(RichText::new(&row.name).strong());
            ui.label(RichText::new(&row.id).weak());
            if row.active {
                ui.label(RichText::new("Active").color(crate::theme::accent_color()));
            } else if ui.button("Use").clicked() {
                requested = Some(Action::Use(row.id.clone()));
            }
        })
        .response;
    response.context_menu(|ui| {
        if ui.button("Use this keyboard").clicked() {
            requested = Some(Action::Use(row.id.clone()));
            ui.close();
        }
        let toggle = if row.enabled { "Disable" } else { "Enable" };
        if ui.button(toggle).clicked() {
            requested = Some(Action::SetEnabled(row.id.clone(), !row.enabled));
            ui.close();
        }
        if ui.button("Remove").clicked() {
            requested = Some(Action::Remove(row.id.clone()));
            ui.close();
        }
    });
    requested
}
