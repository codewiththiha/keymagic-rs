//! The app window: title bar, nav rail, keyboard shortcuts and the active page.

use crate::capture::Capture;
use crate::nav::Page;
use crate::pages::{Notice, PageStates, about};
use crate::state::{Host, lock};
use crate::theme::{self, Variant};
use crate::tray::{self, Command, Tray};
use egui::{Context, Key, Layout, Modifiers, Panel, ScrollArea, Ui, ViewportCommand};
use std::sync::{Arc, Mutex};

const NAV_WIDTH: f32 = 176.0;
const DIGIT_KEYS: [Key; 6] = [
    Key::Num1,
    Key::Num2,
    Key::Num3,
    Key::Num4,
    Key::Num5,
    Key::Num6,
];

/// The eframe app that draws every page.
pub struct KeyMagicApp {
    host: Arc<Mutex<Host>>,
    page: Page,
    variant: Variant,
    pages: PageStates,
    tray: Option<Tray>,
    capture: Option<Capture>,
    notice: Notice,
}

impl KeyMagicApp {
    /// Opens on the first page in dark mode, after `theme::install`.
    #[must_use]
    pub fn new(host: Arc<Mutex<Host>>) -> Self {
        Self {
            host,
            page: Page::default(),
            variant: Variant::Dark,
            pages: PageStates::default(),
            tray: None,
            capture: None,
            notice: Notice::default(),
        }
    }

    /// Adds the tray icon, when the system has one.
    #[must_use]
    pub fn with_tray(mut self, tray: Option<Tray>) -> Self {
        self.tray = tray;
        self
    }

    /// Makes the app save a screenshot and close, for the launch check.
    #[must_use]
    pub fn with_capture(mut self, capture: Option<Capture>) -> Self {
        self.capture = capture;
        self
    }

    /// The page on screen.
    #[must_use]
    pub const fn page(&self) -> Page {
        self.page
    }

    /// The state of every page.
    #[must_use]
    pub const fn pages(&self) -> &PageStates {
        &self.pages
    }

    /// The appearance on screen.
    #[must_use]
    pub const fn variant(&self) -> Variant {
        self.variant
    }

    /// Draws the whole window into `ui`.
    pub fn show(&mut self, ui: &mut Ui) {
        let reduce = lock(&self.host).config().general.reduce_motion;
        theme::set_reduced_motion(ui.ctx(), reduce);
        self.handle_keys(ui);
        self.run_tray(ui.ctx());
        if let Some(capture) = self.capture.as_mut() {
            capture.step(ui.ctx());
        }
        Panel::top("title_bar").show(ui, |ui| self.title_bar(ui));
        Panel::left("nav_rail")
            .exact_size(NAV_WIDTH)
            .resizable(false)
            .show(ui, |ui| self.nav_rail(ui));
        egui::CentralPanel::default().show(ui, |ui| self.page_body(ui));
    }

    /// Applies the tray menu's choice made since the last frame.
    fn run_tray(&mut self, ctx: &Context) {
        let Some(tray) = self.tray.as_mut() else {
            return;
        };
        let items = {
            let host = lock(&self.host);
            tray::items(host.config(), host.keyboards())
        };
        if let Err(error) = tray.set_items(&items) {
            self.notice
                .set(format!("The tray menu could not be updated: {error}"));
        }
        while let Some(command) = tray.poll() {
            match command {
                Command::Use(id) => {
                    let result = lock(&self.host).switch_keyboard(&id);
                    match result {
                        Ok(_) => self.notice.clear(),
                        Err(error) => self.notice.set(format!(
                            "Switched to {id}, but settings were not saved: {error}"
                        )),
                    }
                }
                Command::Open => {
                    ctx.send_viewport_cmd(ViewportCommand::Visible(true));
                    ctx.send_viewport_cmd(ViewportCommand::Focus);
                }
                Command::Quit => ctx.send_viewport_cmd(ViewportCommand::Close),
            }
        }
    }

    fn handle_keys(&mut self, ui: &Ui) {
        let jump = ui.ctx().input_mut(|input| {
            DIGIT_KEYS
                .iter()
                .position(|key| input.consume_key(Modifiers::COMMAND, *key))
        });
        if let Some(index) = jump.and_then(|index| Page::from_number(index + 1)) {
            self.page = index;
        }
        let (backward, forward) = ui.ctx().input_mut(|input| {
            (
                input.consume_key(Modifiers::COMMAND | Modifiers::SHIFT, Key::Tab),
                input.consume_key(Modifiers::COMMAND, Key::Tab),
            )
        });
        if backward {
            self.page = self.page.prev();
        } else if forward {
            self.page = self.page.next();
        }
    }

    fn title_bar(&mut self, ui: &mut Ui) {
        ui.horizontal_centered(|ui| {
            ui.heading("KeyMagic");
            ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(self.variant.switch_label()).clicked() {
                    self.variant = self.variant.toggled();
                    theme::select(ui.ctx(), self.variant);
                }
            });
        });
    }

    fn nav_rail(&mut self, ui: &mut Ui) {
        for page in Page::ALL {
            if ui
                .selectable_label(self.page == page, page.label())
                .clicked()
            {
                self.page = page;
            }
        }
    }

    fn page_body(&mut self, ui: &mut Ui) {
        ui.heading(self.page.summary());
        self.notice.show(ui);
        ScrollArea::vertical().show(ui, |ui| match self.page {
            Page::Keyboards => self.pages.keyboards.show(ui, &self.host),
            Page::Tester => self.pages.tester.show(ui, &self.host),
            Page::Editor => self.pages.editor.show(ui, &self.host),
            Page::Settings => self.pages.settings.show(ui, &self.host),
            Page::Hotkeys => self.pages.hotkeys.show(ui, &self.host),
            Page::About => about::show(ui),
        });
    }
}

impl eframe::App for KeyMagicApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        self.show(ui);
    }
}
