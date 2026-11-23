//! Editor page: edit KMS source with live checks, compile, format, save.

use super::Notice;
use crate::model::{Check, EditorText, has_errors, line_diagnostics};
use crate::state::{Host, lock};
use egui::{
    Align2, Color32, CornerRadius, FontId, Pos2, RichText, Sense, TextEdit, TextStyle, Ui, Vec2,
};
use keymagic_engine::{Engine, KeyEvent};
use keymagic_kms::{Diagnostic, Severity};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Width reserved left of the text for the diagnostic gutter.
const GUTTER: f32 = 18.0;
/// Layout preview rows: indent in keys, then the keys. Last row is Space.
const LAYOUT: [(f32, &str); 5] = [
    (0.0, "1234567890-="),
    (0.5, "qwertyuiop[]"),
    (0.75, "asdfghjkl;'"),
    (1.25, "zxcvbnm,./"),
    (0.0, " "),
];
const KEY_SIZE: f32 = 36.0;
const ERROR_COLOUR: Color32 = Color32::from_rgb(0xE5, 0x48, 0x4D);

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

    /// Draws the keyboard picker, toolbar, editor and diagnostics.
    pub fn show(&mut self, ui: &mut Ui, host: &Mutex<Host>) {
        self.choose_keyboard(ui, host);
        if self.text.id.is_none() {
            ui.label("Choose a keyboard that has a .kms source to edit.");
            self.notice.show(ui);
            return;
        }
        let now = ui.input(|input| input.time);
        self.toolbar(ui, host, now);
        self.editor(ui, now);
        self.run_check(ui, host, now);
        for line in line_diagnostics(&self.text.source, &self.diagnostics) {
            let colour = match line.severity {
                Severity::Error => ERROR_COLOUR,
                Severity::Warning => crate::theme::accent_color(),
            };
            ui.label(RichText::new(format!("Line {}: {}", line.line, line.text)).color(colour));
        }
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

    fn toolbar(&mut self, ui: &mut Ui, host: &Mutex<Host>, now: f64) {
        ui.horizontal(|ui| {
            if ui.button("Compile").clicked() {
                self.compile(host);
            }
            if ui.button("Format").clicked() {
                self.format(host);
            }
            if ui.button("Save").clicked() {
                self.save(host);
            }
        });
        ui.horizontal_wrapped(|ui| {
            ui.label("Insert:");
            for glyph in glyphs() {
                if ui.small_button(glyph.to_string()).clicked() {
                    self.text.source.push(glyph);
                    self.text.edited(now);
                }
            }
        });
        self.notice.show(ui);
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
                        .desired_rows(12),
                )
            })
            .inner;
        response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::Role::MultilineTextInput, true, "KMS source")
        });
        if response.changed() {
            self.text.edited(now);
        }
        let painter = ui.painter();
        for mark in &marks {
            let line = f32::from(u16::try_from(mark.line).unwrap_or(u16::MAX));
            let y = (line - 0.5).mul_add(row_height, response.rect.top());
            let x = response.rect.left() - GUTTER / 2.0;
            let colour = match mark.severity {
                Severity::Error => ERROR_COLOUR,
                Severity::Warning => crate::theme::accent_color(),
            };
            painter.circle_filled(Pos2::new(x, y), 4.0, colour);
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

    /// Draws five rows of keys, each labelled with its output.
    fn layout_preview(&self, ui: &mut Ui) {
        ui.label(RichText::new("Layout preview").strong());
        let Some(engine) = &self.preview else {
            ui.label("Compile without errors to see the layout.");
            return;
        };
        for (indent, row) in LAYOUT {
            ui.horizontal(|ui| {
                ui.add_space(indent * KEY_SIZE);
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
                    key_cap(ui, &label, width);
                }
            });
        }
    }
}

/// A square keycap showing `label`, `width` keys wide.
fn key_cap(ui: &mut Ui, label: &str, width: f32) {
    let size = Vec2::new(KEY_SIZE * width, KEY_SIZE);
    let (rect, _) = ui.allocate_exact_size(size + Vec2::new(4.0, 4.0), Sense::hover());
    let cap = rect.shrink(2.0);
    let visuals = ui.visuals();
    let painter = ui.painter();
    painter.rect(
        cap,
        CornerRadius::same(crate::theme::ROUNDING),
        visuals.widgets.inactive.weak_bg_fill,
        visuals.widgets.inactive.bg_stroke,
        egui::StrokeKind::Inside,
    );
    painter.text(
        cap.center(),
        Align2::CENTER_CENTER,
        label,
        FontId::proportional(18.0),
        visuals.text_color(),
    );
}

/// Myanmar consonants, then the common vowel signs, marks and the virama.
fn glyphs() -> impl Iterator<Item = char> {
    ('\u{1000}'..='\u{1021}').chain([
        '\u{102B}', '\u{102C}', '\u{102D}', '\u{102E}', '\u{102F}', '\u{1030}', '\u{1031}',
        '\u{1032}', '\u{103A}', '\u{1036}', '\u{1037}', '\u{1038}', '\u{1039}',
    ])
}
