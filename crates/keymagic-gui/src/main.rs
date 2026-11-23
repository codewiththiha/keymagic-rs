//! The `KeyMagic` desktop app.

use keymagic_gui::shell::KeyMagicApp;
use keymagic_gui::state::{Host, watch_keyboards};
use keymagic_gui::theme;
use keymagic_host::Paths;
use std::error::Error;
use std::sync::{Arc, Mutex};

fn main() -> Result<(), Box<dyn Error>> {
    let paths = Paths::for_user().ok_or("no user folders for this account")?;
    let host = Arc::new(Mutex::new(Host::open(&paths)?));
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("KeyMagic")
            .with_inner_size([960.0, 640.0])
            .with_min_inner_size([720.0, 480.0]),
        renderer: eframe::Renderer::Glow,
        ..eframe::NativeOptions::default()
    };
    eframe::run_native(
        "KeyMagic",
        options,
        Box::new(move |cc| {
            theme::install(&cc.egui_ctx);
            let ctx = cc.egui_ctx.clone();
            watch_keyboards(&host, move || ctx.request_repaint())?;
            Ok(Box::new(KeyMagicApp::new(host)))
        }),
    )?;
    Ok(())
}
