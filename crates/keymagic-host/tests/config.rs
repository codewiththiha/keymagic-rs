//! Config round-trip, KM3 migration and the keyboard list rules.

use keymagic_host::config::{Config, Installed};
use std::error::Error;

type TestResult = Result<(), Box<dyn Error>>;

const KM3_FIXTURE: &str = include_str!("fixtures/km3_config.toml");

fn installed(id: &str, enabled: bool) -> Installed {
    Installed {
        id: id.to_owned(),
        name: id.to_owned(),
        filename: format!("{id}.km2"),
        hotkey: None,
        hash: None,
        enabled,
    }
}

#[test]
fn km3_config_loads_with_its_keys_and_defaults() -> TestResult {
    let config = Config::from_toml(KM3_FIXTURE)?;
    assert!(config.general.start_with_system);
    assert!(!config.general.check_for_updates);
    assert_eq!(
        config.general.update_remind_after.as_deref(),
        Some("2026-12-01")
    );
    assert_eq!(config.keyboards.active.as_deref(), Some("myanmar3"));
    assert_eq!(config.keyboards.last_used, ["myanmar3", "zawgyi"]);
    let zawgyi = &config.keyboards.installed[1];
    assert_eq!(zawgyi.hotkey.as_deref(), Some(""));
    assert!(!zawgyi.enabled);
    assert!(
        config.keyboards.installed[2].enabled,
        "enabled defaults to true"
    );
    assert_eq!(config.keyboards.installed[2].hash, None);
    assert_eq!(config.composition_mode.enabled_hosts, ["notepad.exe"]);
    assert_eq!(config.direct_mode.enabled_hosts, ["code"]);
    Ok(())
}

#[test]
fn config_round_trips_through_toml() -> TestResult {
    let config = Config::from_toml(KM3_FIXTURE)?;
    let text = config.to_toml()?;
    assert_eq!(Config::from_toml(&text)?, config);
    Ok(())
}

#[test]
fn missing_file_gives_defaults_and_save_creates_folders() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("nested").join("config.toml");
    let defaults = Config::load_or_default(&path)?;
    assert!(defaults.general.check_for_updates);
    defaults.save(&path)?;
    assert_eq!(Config::load_or_default(&path)?, defaults);
    Ok(())
}

#[test]
fn malformed_file_is_an_error_and_is_not_overwritten() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("config.toml");
    std::fs::write(&path, "[general\n")?;
    assert!(Config::load_or_default(&path).is_err());
    assert_eq!(std::fs::read_to_string(&path)?, "[general\n");
    Ok(())
}

#[test]
fn set_active_needs_an_enabled_installed_keyboard() {
    let mut config = Config::default();
    config.keyboards.installed = vec![installed("a", true), installed("b", false)];
    assert!(config.keyboards.set_active("a"));
    assert!(!config.keyboards.set_active("b"));
    assert!(!config.keyboards.set_active("missing"));
    assert_eq!(config.keyboards.active.as_deref(), Some("a"));
}

#[test]
fn disabling_the_active_keyboard_moves_to_the_next_enabled_one() {
    let mut config = Config::default();
    config.keyboards.installed = vec![
        installed("a", true),
        installed("b", false),
        installed("c", true),
    ];
    config.keyboards.active = Some("a".to_owned());
    assert!(config.keyboards.set_enabled("a", false));
    assert_eq!(config.keyboards.active.as_deref(), Some("c"));
    assert!(config.keyboards.set_enabled("c", false));
    assert_eq!(config.keyboards.active, None);
    assert!(!config.keyboards.set_enabled("missing", true));
}

#[test]
fn move_to_reorders_and_clamps_the_index() {
    let mut config = Config::default();
    config.keyboards.installed = vec![installed("a", true), installed("b", true)];
    assert!(config.keyboards.move_to("a", 9));
    let order: Vec<_> = config
        .keyboards
        .installed
        .iter()
        .map(|k| k.id.as_str())
        .collect();
    assert_eq!(order, ["b", "a"]);
    assert!(!config.keyboards.move_to("missing", 0));
}

#[test]
fn hotkey_conflicts_match_after_normalising_and_skip_disabled_keyboards() {
    let mut config = Config::default();
    let with_hotkey = |id: &str, hotkey: &str, enabled: bool| Installed {
        hotkey: Some(hotkey.to_owned()),
        ..installed(id, enabled)
    };
    config.keyboards.installed = vec![
        with_hotkey("zawgyi", "Ctrl+Space", true),
        with_hotkey("myanmar3", "ctrl + SPACE", true),
        with_hotkey("unicode", "Alt+Space", true),
        with_hotkey("off", "Ctrl+Space", false),
        with_hotkey("broken", "Ctrl+", true),
    ];
    assert_eq!(config.hotkey_conflicts(), [("zawgyi", "myanmar3")]);
}

#[test]
fn hotkey_table_defaults_on_and_reads_its_keys() -> TestResult {
    assert!(Config::default().hotkeys.enabled);
    let config = Config::from_toml("[hotkeys]\nenabled = false\nnext_keyboard = \"Ctrl+Tab\"\n")?;
    assert!(!config.hotkeys.enabled);
    assert_eq!(config.hotkeys.next_keyboard.as_deref(), Some("Ctrl+Tab"));
    Ok(())
}
