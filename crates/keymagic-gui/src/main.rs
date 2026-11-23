//! The `KeyMagic` desktop app.

use keymagic_gui::capture::{Capture, Outcome};
use keymagic_gui::shell::KeyMagicApp;
use keymagic_gui::state::{Host, lock, watch_keyboards};
use keymagic_gui::theme;
use keymagic_gui::tray::{self, Tray};
use keymagic_host::Paths;
use std::error::Error;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

fn main() -> Result<(), Box<dyn Error>> {
    let capture_path = capture_path()?;
    let capturing = capture_path.is_some();
    let paths = Paths::for_user().ok_or("no user folders for this account")?;
    let host = Arc::new(Mutex::new(Host::open(&paths)?));
    let outcome: Outcome = Arc::new(Mutex::new(None));
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("KeyMagic")
            .with_inner_size([960.0, 640.0])
            .with_min_inner_size([720.0, 480.0]),
        renderer: eframe::Renderer::Glow,
        ..eframe::NativeOptions::default()
    };
    let capture_outcome = Arc::clone(&outcome);
    eframe::run_native(
        "KeyMagic",
        options,
        Box::new(move |cc| {
            theme::install(&cc.egui_ctx);
            let ctx = cc.egui_ctx.clone();
            watch_keyboards(&host, move || ctx.request_repaint())?;
            // The window works without a tray, e.g. with no AppIndicator.
            let tray = {
                let items = {
                    let guard = lock(&host);
                    tray::items(guard.config(), guard.keyboards())
                };
                match Tray::install(&items) {
                    Ok(tray) => Some(tray),
                    Err(error) => {
                        eprintln!("tray unavailable: {error}");
                        None
                    }
                }
            };
            let capture = capture_path.map(|path| Capture::new(path, capture_outcome));
            let app = KeyMagicApp::new(host)
                .with_launch_check()
                .with_tray(tray)
                .with_capture(capture);
            Ok(Box::new(app))
        }),
    )?;
    if capturing {
        let saved = outcome
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take();
        match saved {
            Some(result) => result?,
            None => return Err("the window closed before the screenshot was taken".into()),
        }
    }
    Ok(())
}

/// `--capture <file.png>` saves the window after a few frames and exits.
fn capture_path() -> Result<Option<PathBuf>, Box<dyn Error>> {
    let mut args = std::env::args_os().skip(1);
    match args.next() {
        None => Ok(None),
        Some(flag) if flag.to_str() == Some("--capture") => {
            let path = args.next().ok_or("--capture needs a PNG file name")?;
            Ok(Some(PathBuf::from(path)))
        }
        Some(other) => Err(format!("unknown argument {}", other.to_string_lossy()).into()),
    }
}
