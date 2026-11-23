//! About page: version, licence and the third-party licence texts.

use egui::{RichText, ScrollArea, Ui};

/// Licence texts of the bundled dependencies and fonts.
const LICENCES: &str = include_str!("../../assets/THIRD_PARTY.txt");

/// Draws the version, the project licence and the bundled licence texts.
pub fn show(ui: &mut Ui) {
    ui.label(format!("KeyMagic {}", env!("CARGO_PKG_VERSION")));
    ui.label("Licensed under GPL-2.0-only.");
    ui.label("Source: https://github.com/codewiththiha/keymagic-rs");
    ui.separator();
    ui.label(RichText::new("Third-party licences").strong());
    ScrollArea::vertical().show(ui, |ui| {
        ui.label(RichText::new(LICENCES).monospace());
    });
}
