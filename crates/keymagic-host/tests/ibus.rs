//! `IBus` component file: engines for enabled keyboards, and XML escaping.

use keymagic_host::Config;
use keymagic_host::Paths;
use keymagic_host::integration::ibus::{Engine, component_xml, engines, user_component_path};
use std::path::{Path, PathBuf};

#[test]
fn one_engine_per_enabled_keyboard_in_saved_order() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::from_toml(
        r#"
        [[keyboards.installed]]
        id = "zawgyi"
        name = "Zawgyi"

        [[keyboards.installed]]
        id = "off"
        name = "Off"
        enabled = false

        [[keyboards.installed]]
        id = "unicode"
        name = "Unicode"
        label = "Myanmar Unicode"
        "#,
    )?;
    let paths = Paths::under(
        Path::new("/nonexistent/config"),
        Path::new("/nonexistent/cache"),
    );
    let found = engines(&config, &paths);
    let ids: Vec<&str> = found.iter().map(|engine| engine.id.as_str()).collect();
    assert_eq!(ids, ["zawgyi", "unicode"]);
    assert_eq!(found[1].label, "Myanmar Unicode");
    assert_eq!(found[0].icon, None);
    Ok(())
}

#[test]
fn component_lists_each_engine_with_its_name() {
    let engines = [Engine {
        id: "unicode".to_owned(),
        label: "Myanmar & Unicode".to_owned(),
        icon: Some(PathBuf::from("/cache/icons/unicode.png")),
    }];
    let xml = component_xml("keymagic-ibus --ibus", &engines);
    assert!(xml.contains("<name>org.freedesktop.IBus.KeyMagic</name>"));
    assert!(xml.contains("<exec>keymagic-ibus --ibus</exec>"));
    assert!(xml.contains("<name>keymagic:unicode</name>"));
    assert!(xml.contains("<longname>Myanmar &amp; Unicode</longname>"));
    assert!(xml.contains("<icon>/cache/icons/unicode.png</icon>"));
}

#[test]
fn user_file_sits_in_the_ibus_component_folder() {
    assert_eq!(
        user_component_path(Path::new("/home/me/.local/share")),
        Path::new("/home/me/.local/share/ibus/component/keymagic.xml")
    );
}

#[test]
fn engine_without_cached_icon_names_the_default_icon() {
    let engines = [Engine {
        id: "probe".to_owned(),
        label: "Probe".to_owned(),
        icon: None,
    }];
    let xml = component_xml("keymagic-ibus --ibus", &engines);
    assert!(xml.contains("<icon>input-keyboard</icon>"));
}
