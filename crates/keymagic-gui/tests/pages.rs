//! Page screenshots and interaction tests, rendered with wgpu.

use egui::{Event, Modifiers};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use keymagic_gui::nav::Page;
use keymagic_gui::shell::KeyMagicApp;
use keymagic_gui::state::Host;
use keymagic_gui::theme;
use keymagic_host::Paths;
use std::error::Error;
use std::fs;
use std::sync::{Arc, Mutex};

type TestResult = Result<(), Box<dyn Error>>;

const TINY: &str = "\"a\" => \"x\"\n";

fn paths_in(dir: &tempfile::TempDir) -> Paths {
    Paths::under(&dir.path().join("config"), &dir.path().join("cache"))
}

/// A shared host and the name of its installed `tiny.kms` keyboard.
type TinyHost = (Arc<Mutex<Host>>, String);

/// A host with `tiny.kms` installed and active, plus its name.
fn host_with_tiny(dir: &tempfile::TempDir) -> Result<TinyHost, Box<dyn Error>> {
    let mut host = Host::open(&paths_in(dir))?;
    let source = dir.path().join("tiny.kms");
    fs::write(&source, TINY)?;
    let id = host.import(&source)?;
    assert!(host.use_keyboard(&id));
    host.save()?;
    let name = host.keyboards()[0].name.clone();
    Ok((Arc::new(Mutex::new(host)), name))
}

fn app(host: Arc<Mutex<Host>>) -> Harness<'static, KeyMagicApp> {
    Harness::builder()
        .with_size(egui::vec2(960.0, 640.0))
        .with_context_setup(theme::install)
        .wgpu()
        .build_ui_state(
            |ui, app: &mut KeyMagicApp| app.show(ui),
            KeyMagicApp::new(host),
        )
}

fn open_page(harness: &mut Harness<'static, KeyMagicApp>, label: &str) {
    harness.get_by_label(label).click();
    harness.run();
}

#[test]
fn keyboards_empty_state() -> TestResult {
    let dir = tempfile::tempdir()?;
    let host = Arc::new(Mutex::new(Host::open(&paths_in(&dir))?));
    let mut harness = app(host);
    harness.run();
    harness.snapshot("page_keyboards_empty");
    Ok(())
}

#[test]
fn imported_kms_appears_in_the_keyboard_list() -> TestResult {
    let dir = tempfile::tempdir()?;
    let mut harness = app(host_with_tiny(&dir)?.0);
    harness.run();
    harness.snapshot("page_keyboards_with_tiny");
    Ok(())
}

#[test]
fn tester_types_through_the_active_keyboard() -> TestResult {
    let dir = tempfile::tempdir()?;
    let mut harness = app(host_with_tiny(&dir)?.0);
    open_page(&mut harness, "Tester");
    harness.event(Event::Text("a".to_owned()));
    harness.run();
    let text = harness.state().pages().tester.text();
    assert_eq!(format!("{}{}", text.committed(), text.preedit()), "x");
    harness.snapshot("page_tester_typed");
    Ok(())
}

#[test]
fn editor_compiles_the_selected_keyboard() -> TestResult {
    let dir = tempfile::tempdir()?;
    let (host, name) = host_with_tiny(&dir)?;
    let mut harness = app(host);
    open_page(&mut harness, "Editor");
    harness.get_by_label("Keyboard").click();
    harness.run();
    harness.get_by_label(&name).click();
    harness.run();
    harness.get_by_label("Compile").click();
    harness.run();
    assert!(
        harness
            .state()
            .pages()
            .editor
            .notice()
            .is_some_and(|text| text.contains("Compiled"))
    );
    harness.snapshot("page_editor_compiled");
    Ok(())
}

#[test]
fn settings_survive_a_restart() -> TestResult {
    let dir = tempfile::tempdir()?;
    let host = Arc::new(Mutex::new(Host::open(&paths_in(&dir))?));
    let mut harness = app(host);
    open_page(&mut harness, "Settings");
    harness.get_by_label("Reduce motion").click();
    harness.run();
    harness.get_by_label("Save settings").click();
    harness.run();
    let reopened = Host::open(&paths_in(&dir))?;
    assert!(reopened.config().general.reduce_motion);
    Ok(())
}

#[test]
fn every_page_renders() -> TestResult {
    let dir = tempfile::tempdir()?;
    let mut harness = app(host_with_tiny(&dir)?.0);
    for page in Page::ALL {
        open_page(&mut harness, page.label());
        assert_eq!(harness.state().page(), page);
        harness.snapshot(format!("page_{}", page.label().to_lowercase()));
    }
    Ok(())
}

#[test]
fn ctrl_one_returns_to_the_keyboards_page() -> TestResult {
    let dir = tempfile::tempdir()?;
    let mut harness = app(host_with_tiny(&dir)?.0);
    open_page(&mut harness, "About");
    harness.key_press_modifiers(Modifiers::COMMAND, egui::Key::Num1);
    harness.run();
    assert_eq!(harness.state().page(), Page::Keyboards);
    Ok(())
}
