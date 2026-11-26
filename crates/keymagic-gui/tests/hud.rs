//! Keyboard-switch notices from a fake IME are drawn in the window.

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use keymagic_gui::hud::Hud;
use keymagic_gui::shell::KeyMagicApp;
use keymagic_gui::state::{Host, lock};
use keymagic_gui::theme;
use keymagic_gui::update::{Checker, Status};
use keymagic_host::Paths;
use keymagic_host::ipc::{self, Message};
use std::error::Error;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

type TestResult = Result<(), Box<dyn Error>>;

#[test]
fn notice_from_a_fake_ime_is_drawn_in_the_window() -> TestResult {
    let dir = tempfile::tempdir()?;
    let paths = Paths::under(&dir.path().join("config"), &dir.path().join("cache"));
    let host = Arc::new(Mutex::new(Host::open(&paths)?));
    let socket = paths.hud_socket_path();
    let hud = Hud::listen(&socket, || {})?;
    let mut fake_ime = ipc::connect(&socket, std::iter::empty::<Duration>())?;
    fake_ime.send(&Message::Hud {
        text: "Zawgyi".to_owned(),
    })?;
    assert!(
        !lock(&host).ime_connected(),
        "notices must not count as an IME"
    );
    let app = KeyMagicApp::new(host)
        .with_update(Checker::with_status(Status::Current))
        .with_hud(Some(hud));
    let mut harness = Harness::builder()
        .with_size(egui::vec2(960.0, 640.0))
        .with_context_setup(theme::install)
        .wgpu()
        .build_ui_state(|ui, app: &mut KeyMagicApp| app.show(ui), app);
    for _ in 0..100 {
        harness.run();
        if harness.query_by_label("Zawgyi").is_some() {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(20));
    }
    Err("the notice was not drawn".into())
}
