//! About page: version, licence and the third-party licence texts.

use crate::widgets;
use egui::{RichText, ScrollArea, Ui};

/// Licence texts of the bundled dependencies and fonts.
const LICENCES: &str = include_str!("../../assets/THIRD_PARTY.txt");

const CHANGELOG: &str = include_str!("../../../../CHANGELOG.md");

/// Turns the changelog into plain text: no title, heading marks or dashes.
fn plain(markdown: &str) -> String {
    markdown
        .lines()
        .filter(|line| !line.starts_with("# "))
        .map(|line| {
            let text = line.trim_start_matches('#').trim_start();
            match text.strip_prefix("- ") {
                Some(item) => format!("\u{2022} {item}"),
                None => text.to_owned(),
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Draws the version, the project licence and the bundled licence texts.
pub fn show(ui: &mut Ui) {
    crate::widgets::section(ui, "Changelog", |ui| {
        ui.label(crate::widgets::muted(ui, &plain(CHANGELOG)));
    });
    ui.add_space(16.0);
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

#[cfg(test)]
mod tests {
    use super::plain;

    #[test]
    fn changelog_drops_the_title_and_marks_bullets() {
        assert_eq!(
            plain("# Changelog\n\n## Unreleased\n\n- one"),
            "\nUnreleased\n\n\u{2022} one"
        );
    }
}
