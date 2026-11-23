//! Keyboards page: install, enable, switch, label, hotkey and remove keyboards.

use super::Notice;
use crate::model::{KeyboardRow, keyboard_rows, matches_filter, parse_hotkey};
use crate::reveal;
use crate::state::{Host, lock};
use crate::theme::{self, CONTROL_ROUNDING, ROUNDING};
use crate::widgets;
use egui::{
    Align, Align2, ColorImage, Context, CornerRadius, FontId, Frame, Image, Layout, Margin, Rect,
    RichText, Sense, TextEdit, TextureHandle, TextureOptions, Ui, vec2,
};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Extensions the import dialog and drag and drop accept.
const EXTENSIONS: [&str; 2] = ["km2", "kms"];

/// A change requested from a keyboard card.
enum Action {
    Use(String),
    SetEnabled(String, bool),
    SetHotkey(String, Option<String>),
    Rename(String, Option<String>),
    Move(String, bool),
    Reveal(String),
    Remove(String),
}

/// An inline editor open under one keyboard card.
enum Editing {
    Hotkey { id: String, draft: String },
    Rename { id: String, draft: String },
    Remove { id: String },
}

impl Editing {
    fn id(&self) -> &str {
        match self {
            Self::Hotkey { id, .. } | Self::Rename { id, .. } | Self::Remove { id } => id,
        }
    }
}

/// View state of the Keyboards page.
#[derive(Default)]
pub struct View {
    notice: Notice,
    filter: String,
    editing: Option<Editing>,
    icons: HashMap<PathBuf, Option<TextureHandle>>,
}

impl View {
    /// Draws the toolbar, the keyboard cards and the empty state.
    pub fn show(&mut self, ui: &mut Ui, host: &Mutex<Host>) {
        for path in dropped_paths(ui.ctx()) {
            self.install(host, &path);
        }
        let (rows, folder) = {
            let guard = lock(host);
            (
                keyboard_rows(guard.config(), guard.keyboards()),
                guard.keyboards_dir().to_path_buf(),
            )
        };
        ui.horizontal(|ui| {
            if widgets::primary(ui, "Import…").clicked() {
                self.import_from_dialog(host);
            }
            if widgets::button(ui, "Open folder").clicked() {
                self.open_folder(&folder);
            }
            if widgets::button(ui, "Scan").clicked() {
                self.scan(host);
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if !rows.is_empty() {
                    ui.label(widgets::muted(ui, &format!("{} installed", rows.len())));
                    ui.add(
                        TextEdit::singleline(&mut self.filter)
                            .hint_text("Filter keyboards")
                            .desired_width(200.0),
                    );
                }
            });
        });
        self.notice.show(ui);
        ui.add_space(16.0);
        if rows.is_empty() {
            widgets::card(ui, |ui| {
                keyboard_art(ui);
                ui.add_space(12.0);
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
        let mut shown = 0;
        for row in rows.iter().filter(|row| matches_filter(row, &self.filter)) {
            shown += 1;
            let icon = row
                .icon
                .as_deref()
                .and_then(|path| icon_texture(ui.ctx(), &mut self.icons, path));
            requested = keyboard_card(ui, row, icon.as_ref(), &mut self.editing).or(requested);
            ui.add_space(8.0);
        }
        if shown == 0 {
            ui.label(widgets::muted(
                ui,
                &format!("No keyboards match \u{201c}{}\u{201d}.", self.filter.trim()),
            ));
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

    fn open_folder(&mut self, folder: &Path) {
        let message = match fs::create_dir_all(folder).and_then(|()| reveal::open_folder(folder)) {
            Ok(()) => return,
            Err(error) => format!("Could not open {}: {error}", folder.display()),
        };
        self.notice.set(message);
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
            Action::SetHotkey(id, hotkey) => {
                if guard.config_mut().keyboards.set_hotkey(&id, hotkey) {
                    format!("Updated the hotkey for {id}.")
                } else {
                    format!("{id} is not installed.")
                }
            }
            Action::Rename(id, label) => {
                if guard.config_mut().keyboards.rename(&id, label) {
                    format!("Renamed {id}.")
                } else {
                    format!("{id} is not installed.")
                }
            }
            Action::Move(id, up) => {
                let keyboards = &mut guard.config_mut().keyboards;
                match keyboards
                    .installed
                    .iter()
                    .position(|keyboard| keyboard.id == id)
                {
                    Some(index) => {
                        let target = if up {
                            index.saturating_sub(1)
                        } else {
                            index + 1
                        };
                        keyboards.move_to(&id, target);
                        format!("Moved {id}.")
                    }
                    None => format!("{id} is not installed."),
                }
            }
            Action::Reveal(id) => {
                let path = guard
                    .keyboards()
                    .iter()
                    .find(|keyboard| keyboard.id == id)
                    .map(|keyboard| keyboard.km2.clone());
                match path {
                    Some(path) => match reveal::show_file(&path) {
                        Ok(()) => format!("Opened {id} in the file manager."),
                        Err(error) => format!("Could not show {id}: {error}"),
                    },
                    None => format!("{id} is not installed."),
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

/// Loads a keyboard icon once and keeps the texture.
fn icon_texture(
    ctx: &Context,
    cache: &mut HashMap<PathBuf, Option<TextureHandle>>,
    path: &Path,
) -> Option<TextureHandle> {
    cache
        .entry(path.to_path_buf())
        .or_insert_with(|| {
            let rgba = image::open(path).ok()?.to_rgba8();
            let size = [rgba.width(), rgba.height()].map(|side| usize::try_from(side).unwrap_or(0));
            let image = ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
            Some(ctx.load_texture(
                format!("keyboard-icon:{}", path.display()),
                image,
                TextureOptions::LINEAR,
            ))
        })
        .clone()
}

/// One keyboard card; returns the change the user asked for, if any.
fn keyboard_card(
    ui: &mut Ui,
    row: &KeyboardRow,
    icon: Option<&TextureHandle>,
    editing: &mut Option<Editing>,
) -> Option<Action> {
    let mut requested = None;
    let fill = if row.active {
        ui.visuals().widgets.inactive.weak_bg_fill
    } else {
        ui.visuals().window_fill
    };
    Frame::new()
        .fill(fill)
        .corner_radius(CornerRadius::same(ROUNDING))
        .inner_margin(Margin::symmetric(16, 12))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                icon_tile(ui, icon, &row.name);
                ui.add_space(12.0);
                ui.vertical(|ui| {
                    ui.label(RichText::new(&row.name).strong());
                    ui.label(widgets::muted(ui, &format!("{}.km2", row.id)).small());
                });
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    requested = card_menu(ui, row, editing).or(requested);
                    if ui.radio(row.active, "Active").clicked() && !row.active {
                        requested = Some(Action::Use(row.id.clone()));
                    }
                    let mut enabled = row.enabled;
                    let label = format!("Enable {}", row.name);
                    if widgets::switch(ui, &mut enabled, &label).changed() {
                        requested = Some(Action::SetEnabled(row.id.clone(), enabled));
                    }
                    let state = if row.enabled { "Enabled" } else { "Disabled" };
                    ui.label(widgets::muted(ui, state));
                    match &row.hotkey {
                        Some(hotkey) => {
                            ui.label(RichText::new(hotkey).monospace());
                        }
                        None => {
                            ui.label(widgets::muted(ui, "No hotkey"));
                        }
                    }
                });
            });
            requested = editor(ui, row, editing).or(requested);
        });
    requested
}

/// The Options menu of one card; editors and confirmations set `editing`.
fn card_menu(ui: &mut Ui, row: &KeyboardRow, editing: &mut Option<Editing>) -> Option<Action> {
    let mut requested = None;
    let menu = ui.menu_button("Options", |ui| {
        if widgets::button(ui, "Set hotkey").clicked() {
            *editing = Some(Editing::Hotkey {
                id: row.id.clone(),
                draft: row.hotkey.clone().unwrap_or_default(),
            });
            ui.close();
        }
        if widgets::button(ui, "Rename").clicked() {
            *editing = Some(Editing::Rename {
                id: row.id.clone(),
                draft: row.name.clone(),
            });
            ui.close();
        }
        if widgets::button(ui, "Show in folder").clicked() {
            requested = Some(Action::Reveal(row.id.clone()));
            ui.close();
        }
        if widgets::button(ui, "Move up").clicked() {
            requested = Some(Action::Move(row.id.clone(), true));
            ui.close();
        }
        if widgets::button(ui, "Move down").clicked() {
            requested = Some(Action::Move(row.id.clone(), false));
            ui.close();
        }
        if widgets::button(ui, "Remove").clicked() {
            *editing = Some(Editing::Remove { id: row.id.clone() });
            ui.close();
        }
    });
    widgets::focus_ring(ui, &menu.response, CONTROL_ROUNDING);
    requested
}

/// The inline editor under a card, when one is open for it.
fn editor(ui: &mut Ui, row: &KeyboardRow, editing: &mut Option<Editing>) -> Option<Action> {
    let current = editing.as_mut()?;
    if current.id() != row.id {
        return None;
    }
    let mut action = None;
    let mut done = false;
    ui.add_space(8.0);
    match current {
        Editing::Hotkey { draft, .. } => {
            ui.horizontal(|ui| {
                ui.add(
                    TextEdit::singleline(draft)
                        .hint_text("Ctrl+Shift+K")
                        .desired_width(200.0),
                );
                if widgets::primary(ui, "Save").clicked() {
                    match parse_hotkey(draft.as_str()) {
                        Ok(hotkey) => {
                            action = Some(Action::SetHotkey(row.id.clone(), hotkey));
                            done = true;
                        }
                        Err(message) => {
                            ui.label(RichText::new(message).color(theme::color(theme::ERROR)));
                        }
                    }
                }
                if widgets::button(ui, "Cancel").clicked() {
                    done = true;
                }
            });
        }
        Editing::Rename { draft, .. } => {
            ui.horizontal(|ui| {
                ui.add(
                    TextEdit::singleline(draft)
                        .hint_text("Display name")
                        .desired_width(240.0),
                );
                if widgets::primary(ui, "Save").clicked() {
                    action = Some(Action::Rename(row.id.clone(), Some(draft.clone())));
                    done = true;
                }
                if widgets::button(ui, "Use original name").clicked() {
                    action = Some(Action::Rename(row.id.clone(), None));
                    done = true;
                }
                if widgets::button(ui, "Cancel").clicked() {
                    done = true;
                }
            });
        }
        Editing::Remove { .. } => {
            ui.horizontal(|ui| {
                ui.label(format!(
                    "Remove {}? Its .km2 and .kms files are deleted.",
                    row.name
                ));
                if widgets::button(
                    ui,
                    RichText::new("Remove").color(theme::color(theme::ERROR)),
                )
                .clicked()
                {
                    action = Some(Action::Remove(row.id.clone()));
                    done = true;
                }
                if widgets::button(ui, "Cancel").clicked() {
                    done = true;
                }
            });
        }
    }
    if done {
        *editing = None;
    }
    action
}

/// The keyboard's icon, or its first letter on a tile when it has none.
fn icon_tile(ui: &mut Ui, icon: Option<&TextureHandle>, name: &str) {
    if let Some(icon) = icon {
        ui.add(Image::new(icon).fit_to_exact_size(vec2(40.0, 40.0)));
        return;
    }
    let (rect, _) = ui.allocate_exact_size(vec2(40.0, 40.0), Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(
        rect,
        CornerRadius::same(ROUNDING),
        ui.visuals().widgets.inactive.bg_fill,
    );
    let initial = name
        .chars()
        .next()
        .map_or_else(String::new, |letter| letter.to_uppercase().to_string());
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        initial,
        FontId::proportional(18.0),
        theme::accent_color(),
    );
}

/// A small keyboard drawn from shapes for the empty state.
fn keyboard_art(ui: &mut Ui) {
    let (rect, _) = ui.allocate_exact_size(vec2(240.0, 64.0), Sense::hover());
    let painter = ui.painter_at(rect);
    let body = ui.visuals().widgets.inactive.weak_bg_fill;
    let key = ui.visuals().widgets.inactive.bg_fill;
    let accent = theme::accent_color();
    painter.rect_filled(rect, CornerRadius::same(ROUNDING), body);
    for (row, top) in [12.0_f32, 26.0, 40.0].into_iter().enumerate() {
        for column in 0..10_u8 {
            let left = 16.0 + f32::from(column) * 20.0;
            let fill = if row == 1 && column == 4 { accent } else { key };
            let cap = Rect::from_min_size(rect.min + vec2(left, top), vec2(14.0, 10.0));
            painter.rect_filled(cap, CornerRadius::same(3), fill);
        }
    }
}
