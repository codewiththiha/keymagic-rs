//! Shell screenshots, rendered with wgpu on the self-hosted runner.

use egui::{Key, Modifiers};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use keymagic_gui::nav::Page;
use keymagic_gui::shell::KeyMagicApp;
use keymagic_gui::state::Host;
use keymagic_gui::theme;
use keymagic_host::Paths;
use std::error::Error;
use std::sync::{Arc, Mutex};

type TestResult = Result<(), Box<dyn Error>>;

fn shell(dir: &tempfile::TempDir) -> Result<Harness<'static, KeyMagicApp>, Box<dyn Error>> {
    let paths = Paths::under(&dir.path().join("config"), &dir.path().join("cache"));
    let host = Arc::new(Mutex::new(Host::open(&paths)?));
    Ok(Harness::builder()
        .with_size(egui::vec2(960.0, 640.0))
        .with_context_setup(theme::install)
        .wgpu()
        .build_ui_state(
            |ui, app: &mut KeyMagicApp| app.show(ui),
            KeyMagicApp::new(host),
        ))
}

#[test]
fn shell_dark() -> TestResult {
    let dir = tempfile::tempdir()?;
    let mut harness = shell(&dir)?;
    harness.run();
    harness.snapshot("shell_dark");
    Ok(())
}

#[test]
fn shell_light() -> TestResult {
    let dir = tempfile::tempdir()?;
    let mut harness = shell(&dir)?;
    harness.get_by_label("Light mode").click();
    harness.run();
    harness.snapshot("shell_light");
    Ok(())
}

#[test]
fn nav_rail_and_shortcuts_change_the_page() -> TestResult {
    let dir = tempfile::tempdir()?;
    let mut harness = shell(&dir)?;
    harness.get_by_label("Settings").click();
    harness.run();
    assert_eq!(harness.state().page(), Page::Settings);
    harness.key_press_modifiers(Modifiers::COMMAND, Key::Num2);
    harness.run();
    assert_eq!(harness.state().page(), Page::Tester);
    Ok(())
}
