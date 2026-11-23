//! Commit, preedit and direct-mode rules: key × composing state × mode.

use keymagic_engine::{Engine, KeyEvent, Modifiers, VirtualKey};
use keymagic_host::config::{Config, Installed};
use keymagic_host::{HostAction, Mode, Session};
use keymagic_kms::{Resolver, compile};
use std::error::Error;
use std::sync::Arc;

type TestResult = Result<(), Box<dyn Error>>;

const SCRIPT: &str = "\"a\" => \"x\"\n\"c\" => \"y \"\n";
const COMPOSITION_APP: &str = "notepad.exe";
const DIRECT_APP: &str = "Code.exe";

struct NoFiles;

impl Resolver for NoFiles {
    fn read(&self, _path: &str) -> Option<Vec<u8>> {
        None
    }
}

#[derive(Clone, Copy, Debug)]
enum Key {
    A,
    B,
    Space,
    Enter,
    Tab,
    Esc,
    Back,
}

impl Key {
    fn event(self) -> KeyEvent {
        let none = Modifiers::default();
        match self {
            Self::A => KeyEvent {
                key: None,
                ch: Some('a'),
                mods: none,
            },
            Self::B => KeyEvent {
                key: None,
                ch: Some('b'),
                mods: none,
            },
            Self::Space => KeyEvent {
                key: Some(VirtualKey::Space),
                ch: Some(' '),
                mods: none,
            },
            Self::Enter => KeyEvent {
                key: Some(VirtualKey::Return),
                ch: None,
                mods: none,
            },
            Self::Tab => KeyEvent {
                key: Some(VirtualKey::Tab),
                ch: None,
                mods: none,
            },
            Self::Esc => KeyEvent {
                key: Some(VirtualKey::Escape),
                ch: None,
                mods: none,
            },
            Self::Back => KeyEvent {
                key: Some(VirtualKey::Back),
                ch: None,
                mods: none,
            },
        }
    }
}

struct Row {
    key: Key,
    composing: &'static str,
    actions: Vec<HostAction>,
    after: &'static str,
}

const fn row(
    key: Key,
    composing: &'static str,
    actions: Vec<HostAction>,
    after: &'static str,
) -> Row {
    Row {
        key,
        composing,
        actions,
        after,
    }
}

fn up(text: &str) -> HostAction {
    HostAction::UpdatePreedit(text.to_owned())
}

fn commit(text: &str) -> HostAction {
    HostAction::Commit(text.to_owned())
}

fn replace(delete: usize, insert: &str) -> HostAction {
    HostAction::ReplaceCommitted {
        delete,
        insert: insert.to_owned(),
    }
}

fn engine() -> Result<Engine, Box<dyn Error>> {
    let keyboard = compile(SCRIPT, &NoFiles).map_err(|errors| format!("{errors:?}"))?;
    Ok(Engine::new(Arc::new(keyboard))?)
}

fn start(app: &str) -> Result<Session, Box<dyn Error>> {
    let mut session = Session::new(engine()?, &Config::default());
    session.focus_in(app);
    Ok(session)
}

/// Composition rows, with the composing text before and after each key.
fn composition_rows() -> Vec<Row> {
    vec![
        row(Key::A, "", vec![up("x"), HostAction::Eat], "x"),
        row(Key::A, "x", vec![up("xx"), HostAction::Eat], "xx"),
        row(Key::A, "y ", vec![up("y x"), HostAction::Eat], "y x"),
        row(Key::B, "", vec![up("b"), HostAction::Eat], "b"),
        row(Key::B, "x", vec![up("xb"), HostAction::Eat], "xb"),
        row(Key::B, "y ", vec![up("y b"), HostAction::Eat], "y b"),
        row(Key::Space, "", vec![commit(" "), HostAction::Eat], ""),
        row(Key::Space, "x", vec![commit("x "), HostAction::Eat], ""),
        row(Key::Space, "y ", vec![commit("y  "), HostAction::Eat], ""),
        row(Key::Enter, "", vec![HostAction::Pass], ""),
        row(Key::Enter, "x", vec![commit("x"), HostAction::Pass], ""),
        row(Key::Enter, "y ", vec![commit("y "), HostAction::Pass], ""),
        row(Key::Tab, "", vec![HostAction::Pass], ""),
        row(Key::Tab, "x", vec![commit("x"), HostAction::Pass], ""),
        row(Key::Tab, "y ", vec![commit("y "), HostAction::Pass], ""),
        row(Key::Esc, "", vec![HostAction::Pass], ""),
        row(Key::Esc, "x", vec![up(""), HostAction::Eat], ""),
        row(Key::Esc, "y ", vec![up(""), HostAction::Eat], ""),
        row(Key::Back, "", vec![HostAction::Pass], ""),
        row(Key::Back, "x", vec![up(""), HostAction::Eat], ""),
        row(Key::Back, "y ", vec![up("y"), HostAction::Eat], "y"),
    ]
}

/// Direct rows: edits go straight to the application as replacements.
fn direct_rows() -> Vec<Row> {
    vec![
        row(Key::A, "", vec![replace(0, "x")], "x"),
        row(Key::A, "x", vec![replace(0, "x")], "xx"),
        row(Key::A, "y ", vec![replace(0, "x")], "y x"),
        row(Key::B, "", vec![replace(0, "b")], "b"),
        row(Key::B, "x", vec![replace(0, "b")], "xb"),
        row(Key::B, "y ", vec![replace(0, "b")], "y b"),
        row(Key::Space, "", vec![replace(0, " ")], ""),
        row(Key::Space, "x", vec![replace(0, " ")], ""),
        row(Key::Space, "y ", vec![replace(0, " ")], ""),
        row(Key::Enter, "", vec![HostAction::Pass], ""),
        row(Key::Enter, "x", vec![HostAction::Pass], ""),
        row(Key::Enter, "y ", vec![HostAction::Pass], ""),
        row(Key::Tab, "", vec![HostAction::Pass], ""),
        row(Key::Tab, "x", vec![HostAction::Pass], ""),
        row(Key::Tab, "y ", vec![HostAction::Pass], ""),
        row(Key::Esc, "", vec![HostAction::Pass], ""),
        row(Key::Esc, "x", vec![HostAction::Pass], ""),
        row(Key::Esc, "y ", vec![HostAction::Pass], ""),
        row(Key::Back, "", vec![HostAction::Pass], ""),
        row(Key::Back, "x", vec![replace(1, "")], ""),
        row(Key::Back, "y ", vec![replace(1, "")], "y"),
    ]
}

fn run_rows(app: &str, mode: Mode, rows: &[Row]) -> TestResult {
    for row in rows {
        let mut session = start(app)?;
        assert_eq!(session.mode(), mode, "mode for {app}");
        session.sync(row.composing);
        let actions = session.key(row.key.event());
        assert_eq!(actions, row.actions, "{:?} on {:?}", row.key, row.composing);
        assert_eq!(
            session.composing(),
            row.after,
            "{:?} on {:?}",
            row.key,
            row.composing
        );
    }
    Ok(())
}

#[test]
fn composition_matrix() -> TestResult {
    run_rows(COMPOSITION_APP, Mode::Composition, &composition_rows())
}

#[test]
fn direct_matrix() -> TestResult {
    run_rows(DIRECT_APP, Mode::Direct, &direct_rows())
}

#[test]
fn matrix_has_at_least_forty_cases() {
    assert!(composition_rows().len() + direct_rows().len() >= 40);
}

#[test]
fn terminals_and_vs_code_default_to_direct() -> TestResult {
    for app in ["Code.exe", "code", "gnome-terminal", "WindowsTerminal.exe"] {
        assert_eq!(start(app)?.mode(), Mode::Direct, "{app}");
    }
    assert_eq!(start("notepad.exe")?.mode(), Mode::Composition);
    Ok(())
}

#[test]
fn explicit_lists_override_the_defaults() -> TestResult {
    let mut config = Config::default();
    config.composition_mode.enabled_hosts = vec!["code".to_owned()];
    config.direct_mode.enabled_hosts = vec!["notepad".to_owned()];
    let mut session = Session::new(engine()?, &config);
    session.focus_in("Code.exe");
    assert_eq!(session.mode(), Mode::Composition);
    session.focus_in("notepad.exe");
    assert_eq!(session.mode(), Mode::Direct);
    Ok(())
}

#[test]
fn focus_out_commits_only_pending_composition() -> TestResult {
    let mut session = start(COMPOSITION_APP)?;
    session.sync("x");
    assert_eq!(session.focus_out(), Some(commit("x")));
    assert_eq!(session.composing(), "");

    let mut session = start(DIRECT_APP)?;
    session.sync("x");
    assert_eq!(session.focus_out(), None);
    Ok(())
}

fn keyboard(hotkey: Option<&str>, enabled: bool) -> Config {
    let mut config = Config::default();
    config.keyboards.installed = vec![Installed {
        id: "zawgyi".to_owned(),
        name: "Zawgyi".to_owned(),
        filename: "zawgyi.km2".to_owned(),
        hotkey: hotkey.map(str::to_owned),
        hash: None,
        label: None,
        enabled,
    }];
    config
}

#[test]
fn switch_hotkey_is_checked_before_the_engine() -> TestResult {
    let config = keyboard(Some("Ctrl+Space"), true);
    let mut session = Session::new(engine()?, &config);
    session.focus_in(COMPOSITION_APP);
    session.sync("x");
    let ctrl_space = KeyEvent {
        mods: Modifiers {
            ctrl: true,
            ..Modifiers::default()
        },
        ..Key::Space.event()
    };
    assert_eq!(
        session.key(ctrl_space),
        [
            commit("x"),
            HostAction::SwitchKeyboard("zawgyi".to_owned()),
            HostAction::Hud("Zawgyi".to_owned()),
        ]
    );
    Ok(())
}

#[test]
fn empty_or_disabled_hotkeys_do_not_switch() -> TestResult {
    for config in [
        keyboard(Some(""), true),
        keyboard(Some("Ctrl+Space"), false),
    ] {
        let mut session = Session::new(engine()?, &config);
        session.focus_in(COMPOSITION_APP);
        let ctrl_space = KeyEvent {
            mods: Modifiers {
                ctrl: true,
                ..Modifiers::default()
            },
            ..Key::Space.event()
        };
        let actions = session.key(ctrl_space);
        assert!(
            !actions
                .iter()
                .any(|action| matches!(action, HostAction::SwitchKeyboard(_))),
            "{actions:?}"
        );
    }
    Ok(())
}

fn installed_as(id: &str, name: &str, enabled: bool) -> Installed {
    Installed {
        id: id.to_owned(),
        name: name.to_owned(),
        filename: format!("{id}.km2"),
        hotkey: None,
        hash: None,
        label: None,
        enabled,
    }
}

fn ctrl(key: Key) -> KeyEvent {
    KeyEvent {
        mods: Modifiers {
            ctrl: true,
            ..Modifiers::default()
        },
        ..key.event()
    }
}

#[test]
fn next_keyboard_hotkey_cycles_through_enabled_keyboards() -> TestResult {
    let mut config = Config::default();
    config.keyboards.installed = vec![
        installed_as("zawgyi", "Zawgyi", true),
        installed_as("off", "Off", false),
        installed_as("myanmar3", "Myanmar 3", true),
    ];
    config.keyboards.active = Some("zawgyi".to_owned());
    config.hotkeys.next_keyboard = Some("Ctrl+Tab".to_owned());
    let mut session = Session::new(engine()?, &config);
    session.focus_in(COMPOSITION_APP);
    assert_eq!(
        session.key(ctrl(Key::Tab)),
        [
            HostAction::SwitchKeyboard("myanmar3".to_owned()),
            HostAction::Hud("Myanmar 3".to_owned()),
        ]
    );
    assert_eq!(
        session.key(ctrl(Key::Tab)),
        [
            HostAction::SwitchKeyboard("zawgyi".to_owned()),
            HostAction::Hud("Zawgyi".to_owned()),
        ]
    );
    Ok(())
}

#[test]
fn disabled_global_hotkeys_let_keys_through() -> TestResult {
    let mut config = keyboard(Some("Ctrl+Space"), true);
    config.hotkeys.enabled = false;
    config.hotkeys.next_keyboard = Some("Ctrl+Tab".to_owned());
    let mut session = Session::new(engine()?, &config);
    session.focus_in(COMPOSITION_APP);
    for key in [Key::Space, Key::Tab] {
        let actions = session.key(ctrl(key));
        assert!(
            !actions
                .iter()
                .any(|action| matches!(action, HostAction::SwitchKeyboard(_))),
            "{actions:?}"
        );
    }
    Ok(())
}
