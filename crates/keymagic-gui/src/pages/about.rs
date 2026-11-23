//! About page: version, licence and the third-party licence texts.

use crate::widgets;
use egui::{RichText, ScrollArea, Ui};

/// Licence texts of the bundled dependencies and fonts.
const LICENCES: &str = include_str!("../../assets/THIRD_PARTY.txt");

/// Draws the version, the project licence and the bundled licence texts.
pub fn show(ui: &mut Ui) {
    widgets::card(ui, |ui| {
        ui.label(RichText::new(format!("KeyMagic {}", env!("CARGO_PKG_VERSION"))).strong());
        ui.label(widgets::muted(ui, "Licensed under GPL-2.0-only."));
        ui.add_space(8.0);
        ui.hyperlink_to(
            "Source code",
            "https://github.com/codewiththiha/keymagic-rs",
        );
    });
    ui.add_space(24.0);
    ui.label(RichText::new("Third-party licences").strong());
    ui.add_space(8.0);
    widgets::card(ui, |ui| {
        ScrollArea::vertical().max_height(360.0).show(ui, |ui| {
            ui.label(RichText::new(LICENCES).monospace());
        });
    });
}
