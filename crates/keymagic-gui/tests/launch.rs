//! Headless launch check: the app builds and runs frames on every OS.

use egui_kittest::Harness;
use keymagic_gui::nav::Page;
use keymagic_gui::shell::KeyMagicApp;
use keymagic_gui::state::Host;
use keymagic_gui::theme;
use keymagic_host::Paths;
use std::error::Error;
use std::sync::{Arc, Mutex};

#[test]
fn launch_runs_frames_on_the_first_page() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let paths = Paths::under(&dir.path().join("config"), &dir.path().join("cache"));
    let host = Arc::new(Mutex::new(Host::open(&paths)?));
    let mut harness = Harness::builder()
        .with_size(egui::vec2(960.0, 640.0))
        .with_context_setup(theme::install)
        .build_ui_state(
            |ui, app: &mut KeyMagicApp| app.show(ui),
            KeyMagicApp::new(host),
        );
    harness.run_steps(3);
    assert_eq!(harness.state().page(), Page::Keyboards);
    Ok(())
}
