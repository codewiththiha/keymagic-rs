//! Editor page: edit KMS source with live checks, compile, format, save.

use super::Notice;
use crate::model::{Check, EditorText, has_errors, line_diagnostics};
use crate::state::{Host, lock};
use crate::theme;
use crate::widgets;
use egui::{
    Align, Button, CornerRadius, Layout, Pos2, Rect, RichText, TextEdit, TextStyle, Ui, Vec2,
};
use keymagic_engine::{Engine, KeyEvent};
use keymagic_kms::{Diagnostic, Severity};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Width reserved left of the text for the diagnostic markers.
const GUTTER: f32 = 18.0;
/// Layout preview rows: indent in keys, then the keys. Last row is Space.
const LAYOUT: [(f32, &str); 5] = [
    (0.0, "1234567890-="),
    (0.5, "qwertyuiop[]"),
    (0.75, "asdfghjkl;'"),
    (1.25, "zxcvbnm,./"),
    (0.0, " "),
];

/// View state of the Editor page.
#[derive(Default)]
pub struct View {
    text: EditorText,
    diagnostics: Vec<Diagnostic>,
    preview: Option<Engine>,
    notice: Notice,
}

impl View {
    /// The last message shown on the page.
    #[must_use]
    pub fn notice(&self) -> Option<&str> {
        self.notice.text()
    }

    /// Draws the keyboard picker, the actions, the editor and the diagnostics.
    pub fn show(&mut self, ui: &mut Ui, host: &Mutex<Host>) {
        let now = ui.input(|input| input.time);
        ui.horizontal(|ui| {
            ui.label(widgets::muted(ui, "Editing"));
            self.choose_keyboard(ui, host);
            if self.text.id.is_some() {
                self.actions(ui, host);
            }
        });
        if self.text.id.is_none() {
            ui.add_space(8.0);
            ui.label(widgets::muted(
                ui,
                "Choose a keyboard that has a .kms source to edit.",
            ));
            self.notice.show(ui);
            return;
        }
        self.notice.show(ui);
        ui.add_space(8.0);
        self.editor(ui, now);
        self.run_check(ui, host, now);
        self.diagnostics_list(ui);
        ui.add_space(16.0);
        self.glyph_picker(ui, now);
        ui.add_space(16.0);
        self.layout_preview(ui);
    }

    fn choose_keyboard(&mut self, ui: &mut Ui, host: &Mutex<Host>) {
        let installed: Vec<(String, String)> = lock(host)
            .keyboards()
            .iter()
            .map(|found| (found.id.clone(), found.name.clone()))
            .collect();
        let mut chosen = None;
        let selected = self.text.id.as_deref().unwrap_or("Choose a keyboard");
        let picker = egui::ComboBox::from_id_salt("editor_keyboard")
            .selected_text(selected)
            .show_ui(ui, |ui| {
                for (id, name) in &installed {
                    let current = self.text.id.as_deref() == Some(id.as_str());
                    if ui.selectable_label(current, name).clicked() {
                        chosen = Some(id.clone());
                    }
                }
            });
        picker
            .response
            .widget_info(|| egui::WidgetInfo::labeled(egui::Role::ComboBox, true, "Keyboard"));
        if let Some(id) = chosen {
            self.open(host, &id);
        }
    }

    fn open(&mut self, host: &Mutex<Host>, id: &str) {
        let guard = lock(host);
        match guard.source(id) {
            Ok(Some(source)) => {
                self.text.open(id, source);
                self.refresh(&guard);
                self.notice.clear();
            }
            Ok(None) => self.notice.set(format!(
                "{id} was imported as .km2 only, so it has no source to edit."
            )),
            Err(error) => self.notice.set(format!("Could not read {id}: {error}")),
        }
    }

    /// Recomputes diagnostics and, if there are no errors, the preview.
    fn refresh(&mut self, host: &Host) {
        self.diagnostics = host.check(&self.text.source);
        self.preview = if has_errors(&self.diagnostics) {
            None
        } else {
            host.compile(&self.text.source)
                .ok()
                .and_then(|keyboard| Engine::new(Arc::new(keyboard)).ok())
        };
    }

    /// Compile is the one primary action. Format and Save sit to its left.
    fn actions(&mut self, ui: &mut Ui, host: &Mutex<Host>) {
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if widgets::primary(ui, "Compile").clicked() {
                self.compile(host);
            }
            if ui.button("Format").clicked() {
                self.format(host);
            }
            if ui.button("Save").clicked() {
                self.save(host);
            }
        });
    }

    /// Myanmar characters and marks, in a collapsed grid of fixed-size cells.
    fn glyph_picker(&mut self, ui: &mut Ui, now: f64) {
        ui.collapsing("Insert a character", |ui| {
            widgets::card(ui, |ui| {
                ui.spacing_mut().item_spacing = Vec2::splat(4.0);
                ui.horizontal_wrapped(|ui| {
                    for glyph in glyphs() {
                        let cell = Button::new(RichText::new(glyph.to_string()).size(18.0))
                            .min_size(Vec2::new(36.0, 32.0));
                        let hint = format!("U+{:04X}", u32::from(glyph));
                        if ui.add(cell).on_hover_text(hint).clicked() {
                            self.text.source.push(glyph);
                            self.text.edited(now);
                        }
                    }
                });
            });
        });
    }

    fn editor(&mut self, ui: &mut Ui, now: f64) {
        let row_height = ui.text_style_height(&TextStyle::Monospace);
        let marks = line_diagnostics(&self.text.source, &self.diagnostics);
        let response = ui
            .horizontal_top(|ui| {
                ui.add_space(GUTTER);
                ui.add(
                    TextEdit::multiline(&mut self.text.source)
                        .font(TextStyle::Monospace)
                        .code_editor()
                        .desired_width(f32::INFINITY)
                        .desired_rows(14),
                )
            })
            .inner;
        response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::Role::MultilineTextInput, true, "KMS source")
        });
        if response.changed() {
            self.text.edited(now);
        }
        let warning = theme::muted_color(ui.visuals());
        let painter = ui.painter();
        for mark in &marks {
            let line = f32::from(u16::try_from(mark.line).unwrap_or(u16::MAX));
            let top = (line - 1.0).mul_add(row_height, response.rect.top());
            let bar = Rect::from_min_size(
                Pos2::new(response.rect.left() - 10.0, top),
                Vec2::new(3.0, row_height),
            );
            let colour = match mark.severity {
                Severity::Error => theme::color(theme::ERROR),
                Severity::Warning => warning,
            };
            painter.rect_filled(bar, CornerRadius::same(1), colour);
        }
    }

    fn run_check(&mut self, ui: &Ui, host: &Mutex<Host>, now: f64) {
        match self.text.check(now) {
            Check::Idle => {}
            Check::Wait(seconds) => ui
                .ctx()
                .request_repaint_after(Duration::from_secs_f64(seconds)),
            Check::Due => {
                self.refresh(&lock(host));
                self.text.checked();
            }
        }
    }

    /// One line per problem. Errors are red; warnings use the muted colour.
    fn diagnostics_list(&self, ui: &mut Ui) {
        for line in line_diagnostics(&self.text.source, &self.diagnostics) {
            let text = format!("Line {}: {}", line.line, line.text);
            match line.severity {
                Severity::Error => {
                    ui.colored_label(theme::color(theme::ERROR), text);
                }
                Severity::Warning => {
                    ui.label(widgets::muted(ui, &text));
                }
            }
        }
    }

    fn compile(&mut self, host: &Mutex<Host>) {
        let Some(id) = self.text.id.clone() else {
            return;
        };
        let message = {
            let mut guard = lock(host);
            match guard.save_source(&id, &self.text.source) {
                Ok(found) if found.is_empty() => {
                    if let Err(error) = guard.rescan() {
                        self.notice
                            .set(format!("Could not read the keyboard folder: {error}"));
                    }
                    self.refresh(&guard);
                    match guard.save() {
                        Ok(()) => format!("Compiled and installed {id}."),
                        Err(error) => {
                            format!("Compiled {id}, but settings were not saved: {error}")
                        }
                    }
                }
                Ok(found) => {
                    let count = found.len();
                    self.diagnostics = found;
                    self.preview = None;
                    format!("Not compiled: {count} problem(s). Fix them first.")
                }
                Err(error) => format!("Could not save {id}: {error}"),
            }
        };
        self.notice.set(message);
    }

    fn format(&mut self, host: &Mutex<Host>) {
        let guard = lock(host);
        match guard.format(&self.text.source) {
            Ok(formatted) => {
                self.text.source = formatted;
                self.refresh(&guard);
                self.notice.set("Formatted.");
            }
            Err(diagnostics) => {
                self.diagnostics = diagnostics;
                self.notice.set("Fix the errors before formatting.");
            }
        }
    }

    fn save(&mut self, host: &Mutex<Host>) {
        let Some(id) = self.text.id.clone() else {
            return;
        };
        let result = lock(host).write_source(&id, &self.text.source);
        self.notice.set(match result {
            Ok(()) => format!("Saved {id}.kms. Compile to use the change."),
            Err(error) => format!("Could not save {id}: {error}"),
        });
    }

    /// Five rows of keys, each keycap labelled with the character it produces.
    fn layout_preview(&self, ui: &mut Ui) {
        widgets::section(ui, "Layout preview", |ui| {
            let Some(engine) = &self.preview else {
                ui.label(widgets::muted(
                    ui,
                    "Compile without errors to see the layout.",
                ));
                return;
            };
            for (indent, row) in LAYOUT {
                ui.horizontal(|ui| {
                    ui.add_space(indent * widgets::KEY_UNIT);
                    for ch in row.chars() {
                        let output = engine.peek(KeyEvent {
                            ch: Some(ch),
                            ..KeyEvent::default()
                        });
                        let label = if output.consumed {
                            output.composing
                        } else {
                            ch.to_string()
                        };
                        let width = if ch == ' ' { 6.0 } else { 1.0 };
                        widgets::key_cap(ui, &label, width);
                    }
                });
            }
        });
    }
}

/// Myanmar consonants, then the common vowel signs, marks and the virama.
fn glyphs() -> impl Iterator<Item = char> {
    ('\u{1000}'..='\u{1021}').chain([
        '\u{102B}', '\u{102C}', '\u{102D}', '\u{102E}', '\u{102F}', '\u{1030}', '\u{1031}',
        '\u{1032}', '\u{103A}', '\u{1036}', '\u{1037}', '\u{1038}', '\u{1039}',
    ])
}
