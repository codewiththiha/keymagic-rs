//! Tray menu. `items` is the policy: which keyboards to offer and what each
//! choice asks the app to do. `Tray` only translates it to the OS menu.

use crate::theme::ACCENT;
use keymagic_host::Config;
use keymagic_host::store::Found;
use std::error::Error;
use std::path::{Path, PathBuf};
use tray_icon::menu::{
    CheckMenuItem, Icon as MenuIcon, IconMenuItem, Menu, MenuEvent, MenuId, MenuItem,
};
use tray_icon::{BadIcon, Icon, TrayIcon, TrayIconBuilder};

/// What a tray menu entry asks the app to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// Turn `KeyMagic` on with the last keyboard used, or off.
    Toggle,
    /// Make the keyboard with this id active.
    Use(String),
    /// Show the window.
    Open,
    /// Close the app.
    Quit,
}

/// One entry of the tray menu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    pub label: String,
    pub command: Command,
    /// The keyboard's icon, drawn before the label.
    pub icon: Option<PathBuf>,
    /// Shows a tick mark when set. The on/off entry uses it.
    pub checked: Option<bool>,
}

/// The on/off entry, enabled keyboards with their icons, then Open and Quit.
#[must_use]
pub fn items(config: &Config, found: &[Found]) -> Vec<Item> {
    let active = config.keyboards.active.as_deref();
    let mut items = vec![Item {
        label: "KeyMagic on".to_owned(),
        command: Command::Toggle,
        icon: None,
        checked: Some(active.is_some()),
    }];
    items.extend(
        found
            .iter()
            .filter(|keyboard| {
                config
                    .keyboards
                    .installed
                    .iter()
                    .any(|installed| installed.id == keyboard.id && installed.enabled)
            })
            .map(|keyboard| {
                let label = if active == Some(keyboard.id.as_str()) {
                    format!("{} (active)", keyboard.name)
                } else {
                    keyboard.name.clone()
                };
                Item {
                    label,
                    command: Command::Use(keyboard.id.clone()),
                    icon: keyboard.icon.clone(),
                    checked: None,
                }
            }),
    );
    items.push(Item {
        label: "Open KeyMagic".to_owned(),
        command: Command::Open,
        icon: None,
        checked: None,
    });
    items.push(Item {
        label: "Quit".to_owned(),
        command: Command::Quit,
        icon: None,
        checked: None,
    });
    items
}

/// The system tray icon and its menu.
pub struct Tray {
    icon: TrayIcon,
    commands: Vec<(MenuId, Command)>,
    shown: Vec<Item>,
    active: bool,
}

impl Tray {
    /// Creates the tray icon with `items`. Call after the event loop starts.
    #[doc = "# Errors\n\nReturns an error when the system has no tray or the icon cannot be drawn."]
    pub fn install(items: &[Item]) -> Result<Self, Box<dyn Error>> {
        #[cfg(target_os = "linux")]
        gtk::init()?;
        let icon = TrayIconBuilder::new()
            .with_tooltip("KeyMagic")
            .with_icon(badge(false)?)
            .build()?;
        let mut tray = Self {
            icon,
            commands: Vec::new(),
            shown: Vec::new(),
            active: false,
        };
        tray.set_items(items)?;
        Ok(tray)
    }

    /// Shows the accent badge while a keyboard is active, a grey one otherwise.
    #[doc = "# Errors\n\nReturns an error when the icon cannot be drawn."]
    pub fn set_active(&mut self, active: bool) -> Result<(), Box<dyn Error>> {
        if self.active == active {
            return Ok(());
        }
        self.icon.set_icon(Some(badge(active)?))?;
        self.active = active;
        Ok(())
    }

    /// Rebuilds the menu when `items` differ from the menu on screen.
    #[doc = "# Errors\n\nReturns an error when a menu entry cannot be created."]
    pub fn set_items(&mut self, items: &[Item]) -> Result<(), Box<dyn Error>> {
        if self.shown.as_slice() == items {
            return Ok(());
        }
        let menu = Menu::new();
        let mut commands = Vec::with_capacity(items.len());
        for (index, item) in items.iter().enumerate() {
            let id = MenuId::new(format!("item-{index}"));
            let icon = item.icon.as_deref().and_then(menu_icon);
            match (item.checked, icon) {
                (Some(checked), _) => menu.append(&CheckMenuItem::with_id(
                    id.clone(),
                    &item.label,
                    true,
                    checked,
                    None,
                ))?,
                (None, Some(icon)) => menu.append(&IconMenuItem::with_id(
                    id.clone(),
                    &item.label,
                    true,
                    Some(icon),
                    None,
                ))?,
                (None, None) => {
                    menu.append(&MenuItem::with_id(id.clone(), &item.label, true, None))?;
                }
            }
            commands.push((id, item.command.clone()));
        }
        self.icon.set_menu(Some(Box::new(menu)));
        self.commands = commands;
        self.shown = items.to_vec();
        Ok(())
    }

    /// The next command chosen in the menu, if any. Call once per frame.
    #[must_use]
    pub fn poll(&self) -> Option<Command> {
        #[cfg(target_os = "linux")]
        pump_toolkit();
        let event = MenuEvent::receiver().try_recv().ok()?;
        self.commands
            .iter()
            .find(|(id, _)| *id == *event.id())
            .map(|(_, command)| command.clone())
    }
}

/// Linux menus are GTK widgets, so their events need GTK to run.
#[cfg(target_os = "linux")]
fn pump_toolkit() {
    while gtk::events_pending() {
        gtk::main_iteration_do(false);
    }
}

/// A 16 by 16 menu icon read from `path`. None when the file cannot be read.
fn menu_icon(path: &Path) -> Option<MenuIcon> {
    const SIZE: u32 = 16;
    let rgba = image::open(path)
        .ok()?
        .resize(SIZE, SIZE, image::imageops::FilterType::Lanczos3)
        .to_rgba8();
    let (width, height) = rgba.dimensions();
    MenuIcon::from_rgba(rgba.into_raw(), width, height).ok()
}

const IDLE: [u8; 3] = [0x7A, 0x75, 0x6B];

/// A 32 by 32 disc: amber while active, grey otherwise.
fn badge(active: bool) -> Result<Icon, BadIcon> {
    const SIZE: u16 = 32;
    let [red, green, blue] = if active { ACCENT } else { IDLE };
    let mut rgba = Vec::with_capacity(usize::from(SIZE) * usize::from(SIZE) * 4);
    for y in 0..SIZE {
        for x in 0..SIZE {
            let dx = f32::from(x) - 15.5;
            let dy = f32::from(y) - 15.5;
            let inside = dx.mul_add(dx, dy * dy) <= 15.0 * 15.0;
            rgba.extend(if inside {
                [red, green, blue, 255]
            } else {
                [0, 0, 0, 0]
            });
        }
    }
    Icon::from_rgba(rgba, u32::from(SIZE), u32::from(SIZE))
}
