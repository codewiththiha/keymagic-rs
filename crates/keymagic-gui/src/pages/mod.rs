//! The six pages. Each draws from `state::Host` and keeps its own view state.

pub mod about;
pub mod editor;
pub mod hotkeys;
pub mod keyboards;
pub mod settings;
pub mod tester;

use egui::Ui;

/// Result of the last action on a page, shown under its toolbar.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Notice(Option<String>);

impl Notice {
    pub fn set(&mut self, text: impl Into<String>) {
        self.0 = Some(text.into());
    }

    pub fn clear(&mut self) {
        self.0 = None;
    }

    #[must_use]
    pub fn text(&self) -> Option<&str> {
        self.0.as_deref()
    }

    pub fn show(&self, ui: &mut Ui) {
        if let Some(text) = &self.0 {
            ui.label(crate::widgets::muted(ui, text));
        }
    }
}

/// The state of every page.
#[derive(Default)]
pub struct PageStates {
    pub keyboards: keyboards::View,
    pub tester: tester::View,
    pub editor: editor::View,
    pub settings: settings::View,
    pub hotkeys: hotkeys::View,
}
