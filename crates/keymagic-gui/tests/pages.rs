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
const OTHER: &str = "\"b\" => \"y\"\n";

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
    app_sized(host, 960.0, 640.0)
}

fn app_sized(host: Arc<Mutex<Host>>, width: f32, height: f32) -> Harness<'static, KeyMagicApp> {
    Harness::builder()
        .with_size(egui::vec2(width, height))
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
fn bundled_banner_installs_the_missing_keyboards() -> TestResult {
    let dir = tempfile::tempdir()?;
    let host = Arc::new(Mutex::new(Host::open(&paths_in(&dir))?));
    let mut harness = app(host.clone());
    harness.run();
    harness.get_by_label("Install").click();
    harness.run();
    let missing = host
        .lock()
        .map_err(|_| "host lock poisoned")?
        .bundled_missing()?;
    assert_eq!(missing, 0);
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

/// Opens the Editor page, picks `name` and compiles it.
fn compile_in_editor(harness: &mut Harness<'static, KeyMagicApp>, name: &str) {
    open_page(harness, "Editor");
    harness.get_by_label("Keyboard").click();
    harness.run();
    harness.get_by_label(name).click();
    harness.run();
    harness.get_by_label("Compile").click();
    harness.run();
}

#[test]
fn editor_compiles_the_selected_keyboard() -> TestResult {
    let dir = tempfile::tempdir()?;
    let (host, name) = host_with_tiny(&dir)?;
    let mut harness = app(host);
    compile_in_editor(&mut harness, &name);
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
fn editor_fits_a_compact_window() -> TestResult {
    let dir = tempfile::tempdir()?;
    let (host, name) = host_with_tiny(&dir)?;
    let mut harness = app_sized(host, 720.0, 480.0);
    compile_in_editor(&mut harness, &name);
    harness.snapshot("page_editor_compact");
    Ok(())
}

#[test]
fn keyboards_use_a_wide_window() -> TestResult {
    let dir = tempfile::tempdir()?;
    let mut harness = app_sized(host_with_tiny(&dir)?.0, 1280.0, 800.0);
    harness.run();
    harness.snapshot("page_keyboards_wide");
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
fn reduce_motion_setting_reaches_the_window() -> TestResult {
    let dir = tempfile::tempdir()?;
    let host = Arc::new(Mutex::new(Host::open(&paths_in(&dir))?));
    let mut harness = app(host);
    open_page(&mut harness, "Settings");
    harness.run();
    assert!(harness.ctx.global_style().animation_time > f32::EPSILON);
    harness.get_by_label("Reduce motion").click();
    harness.run();
    assert!(harness.ctx.global_style().animation_time < f32::EPSILON);
    Ok(())
}

#[test]
fn shared_shortcut_is_reported_on_the_hotkeys_page() -> TestResult {
    let dir = tempfile::tempdir()?;
    let (host, _) = host_with_tiny(&dir)?;
    {
        let mut guard = host.lock().map_err(|_| "host lock poisoned")?;
        let other = dir.path().join("other.kms");
        fs::write(&other, OTHER)?;
        guard.import(&other)?;
        for entry in &mut guard.config_mut().keyboards.installed {
            entry.hotkey = Some("Ctrl+Shift+K".to_owned());
        }
    }
    let mut harness = app(host);
    open_page(&mut harness, "Hotkeys");
    harness.run();
    assert!(
        harness
            .query_by_label_contains("Shortcut conflict")
            .is_some()
    );
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

#[test]
fn editor_layers_show_shift_and_altgr_outputs() -> TestResult {
    let dir = tempfile::tempdir()?;
    let (host, name) = host_with_tiny(&dir)?;
    let mut harness = app(host);
    open_page(&mut harness, "Editor");
    compile_in_editor(&mut harness, &name);
    harness.event(Event::ModifiersChanged(Modifiers::SHIFT));
    harness.run();
    assert!(harness.query_by_label_contains("Shift layer").is_some());
    harness.snapshot("page_editor_shift");
    harness.event(Event::ModifiersChanged(Modifiers {
        ctrl: true,
        alt: true,
        ..Default::default()
    }));
    harness.run();
    assert!(harness.query_by_label_contains("AltGr layer").is_some());
    harness.snapshot("page_editor_altgr");
    Ok(())
}

#[test]
fn tester_uses_the_rules_of_a_recompiled_keyboard() -> TestResult {
    let dir = tempfile::tempdir()?;
    let (host, _) = host_with_tiny(&dir)?;
    let shared = Arc::clone(&host);
    let mut harness = app(host);
    open_page(&mut harness, "Tester");
    harness.event(Event::Text("a".to_owned()));
    harness.run();
    let id = shared
        .lock()
        .map_err(|_| "host lock poisoned")?
        .config()
        .keyboards
        .active
        .clone()
        .ok_or("no active keyboard")?;
    let diagnostics = shared
        .lock()
        .map_err(|_| "host lock poisoned")?
        .save_source(&id, "\"a\" => \"y\"\n")?;
    assert!(diagnostics.is_empty());
    harness.run();
    harness.event(Event::Text("a".to_owned()));
    harness.run();
    let text = harness.state().pages().tester.text();
    assert_eq!(format!("{}{}", text.committed(), text.preedit()), "y");
    Ok(())
}

#[test]
fn author_edits_formats_saves_compiles_and_tests() -> TestResult {
    let dir = tempfile::tempdir()?;
    let (host, name) = host_with_tiny(&dir)?;
    let mut harness = app(host);
    compile_in_editor(&mut harness, &name);
    harness.get_by_label("Format").click();
    harness.run();
    harness.get_by_label("Save").click();
    harness.run();
    assert!(
        harness
            .state()
            .pages()
            .editor
            .notice()
            .is_some_and(|text| text.starts_with("Saved")),
    );
    harness.get_by_label("Compile").click();
    harness.run();
    open_page(&mut harness, "Tester");
    harness.event(Event::Text("a".to_owned()));
    harness.run();
    let text = harness.state().pages().tester.text();
    assert_eq!(format!("{}{}", text.committed(), text.preedit()), "x");
    Ok(())
}

/// Types a rule in the editor, then compiles and tests in one window.
#[test]
fn author_types_a_rule_then_compiles_and_tests_in_one_window() -> TestResult {
    let dir = tempfile::tempdir()?;
    let (host, name) = host_with_tiny(&dir)?;
    let mut harness = app(host);
    compile_in_editor(&mut harness, &name);
    harness.get_by_label("KMS source").focus();
    harness.run();
    harness
        .get_by_label("KMS source")
        .replace_selected_text_accesskit("\"b\" => \"y\"\n");
    harness.run();
    harness.get_by_label("Save").click();
    harness.run();
    harness.get_by_label("Compile").click();
    harness.run();
    open_page(&mut harness, "Tester");
    harness.event(Event::Text("b".to_owned()));
    harness.run();
    let text = harness.state().pages().tester.text();
    assert_eq!(format!("{}{}", text.committed(), text.preedit()), "y");
    Ok(())
}

#[test]
fn record_case_writes_the_typed_case() -> TestResult {
    let dir = tempfile::tempdir()?;
    let (shared, _) = host_with_tiny(&dir)?;
    let (id, folder) = {
        let host = shared.lock().map_err(|_| "host lock poisoned")?;
        (
            host.keyboards()[0].id.clone(),
            host.keyboards_dir().to_owned(),
        )
    };
    let mut harness = app(Arc::clone(&shared));
    open_page(&mut harness, "Tester");
    harness.event(Event::Text("a".to_owned()));
    harness.run();
    harness.get_by_label("Record case").click();
    harness.run();
    let text = fs::read_to_string(folder.join(format!("{id}.cases.toml")))?;
    assert!(text.contains("input = \"a\""), "{text}");
    assert!(text.contains("expected = \"x\""), "{text}");
    Ok(())
}
