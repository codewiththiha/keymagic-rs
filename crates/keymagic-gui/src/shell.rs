//! The app window: title bar, nav rail, keyboard shortcuts and the active page.

use crate::capture::Capture;
use crate::nav::Page;
use crate::pages::{Notice, PageStates, about};
use crate::state::{Host, lock};
use crate::theme::{self, Variant};
use crate::tray::{self, Command, Tray};
use crate::update::Checker;
use crate::widgets;
use egui::{
    Context, Frame, Key, Layout, Margin, Modifiers, Panel, RichText, ScrollArea, Ui,
    ViewportCommand,
};
use std::sync::{Arc, Mutex};

const NAV_WIDTH: f32 = 176.0;
/// Widest line length for page content, so wide windows keep readable rows.
const CONTENT_WIDTH: f32 = 880.0;
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
    update: Checker,
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
            update: Checker::default(),
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

    /// Starts the update check, when the settings allow it at launch.
    #[must_use]
    pub fn with_launch_check(self) -> Self {
        self.update.start_at_launch(&self.host);
        self
    }

    /// Replaces the update check, for tests and snapshots.
    #[must_use]
    pub fn with_update(mut self, update: Checker) -> Self {
        self.update = update;
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
        let background = ui.visuals().panel_fill;
        let sidebar = ui.visuals().window_fill;
        Panel::top("title_bar")
            .show_separator_line(false)
            .frame(
                Frame::new()
                    .fill(background)
                    .inner_margin(Margin::symmetric(24, 12)),
            )
            .show(ui, |ui| self.title_bar(ui));
        Panel::left("nav_rail")
            .exact_size(NAV_WIDTH)
            .resizable(false)
            .show_separator_line(false)
            .frame(
                Frame::new()
                    .fill(sidebar)
                    .inner_margin(Margin::symmetric(0, 16)),
            )
            .show(ui, |ui| self.nav_rail(ui));
        egui::CentralPanel::default()
            .frame(
                Frame::new()
                    .fill(background)
                    .inner_margin(Margin::symmetric(32, 24)),
            )
            .show(ui, |ui| self.page_body(ui));
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
        let active = lock(&self.host).config().keyboards.active.is_some();
        if let Err(error) = tray.set_active(active) {
            self.notice
                .set(format!("The tray icon could not be updated: {error}"));
        }
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
                Command::Toggle => {
                    let result = {
                        let mut host = lock(&self.host);
                        if host.config().keyboards.active.is_some() {
                            host.turn_off()
                        } else {
                            host.turn_on()
                        }
                    };
                    match result {
                        Ok(true) => self.notice.clear(),
                        Ok(false) => self.notice.set("Enable a keyboard first.".to_owned()),
                        Err(error) => self
                            .notice
                            .set(format!("KeyMagic was not switched: {error}")),
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
            ui.label(RichText::new("KeyMagic").size(16.0).strong());
            ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                if widgets::button(ui, self.variant.switch_label()).clicked() {
                    self.variant = self.variant.toggled();
                    theme::select(ui.ctx(), self.variant);
                }
            });
        });
    }

    fn nav_rail(&mut self, ui: &mut Ui) {
        ui.spacing_mut().item_spacing.y = 2.0;
        for page in Page::ALL {
            let response = widgets::nav_item(ui, page, self.page == page);
            if response.clicked() {
                self.page = page;
            }
        }
    }

    fn page_body(&mut self, ui: &mut Ui) {
        ui.heading(self.page.summary());
        ui.add_space(16.0);
        self.notice.show(ui);
        let width = ui.available_width().min(CONTENT_WIDTH);
        ScrollArea::vertical().show(ui, |ui| {
            ui.set_max_width(width);
            match self.page {
                Page::Keyboards => self.pages.keyboards.show(ui, &self.host),
                Page::Tester => self.pages.tester.show(ui, &self.host),
                Page::Editor => self.pages.editor.show(ui, &self.host),
                Page::Settings => self.pages.settings.show(ui, &self.host),
                Page::Hotkeys => self.pages.hotkeys.show(ui, &self.host),
                Page::About => about::show(ui, &self.host, &self.update),
            }
        });
    }
}

impl eframe::App for KeyMagicApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        self.show(ui);
    }
}
