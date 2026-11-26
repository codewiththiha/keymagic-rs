//! `IBus` component file: one engine per enabled keyboard.

use crate::config::Config;
use crate::paths::Paths;
use std::path::{Path, PathBuf};

/// Bus name the `IBus` engine process claims.
pub const COMPONENT_NAME: &str = "org.freedesktop.IBus.KeyMagic";

/// Icon name used when no keyboard icon is cached.
const DEFAULT_ICON: &str = "input-keyboard";

/// One engine in the component file, named `keymagic:<id>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Engine {
    pub id: String,
    pub label: String,
    pub icon: Option<PathBuf>,
}

/// Engines for the enabled keyboards, in the saved order.
#[must_use]
pub fn engines(config: &Config, paths: &Paths) -> Vec<Engine> {
    config
        .keyboards
        .installed
        .iter()
        .filter(|keyboard| keyboard.enabled)
        .map(|keyboard| {
            let icon = paths.icon_cache_dir().join(format!("{}.png", keyboard.id));
            Engine {
                id: keyboard.id.clone(),
                label: keyboard
                    .label
                    .as_deref()
                    .unwrap_or(&keyboard.name)
                    .to_owned(),
                icon: icon.exists().then_some(icon),
            }
        })
        .collect()
}

/// The component XML telling `IBus` where the engine runs and what it offers.
#[must_use]
pub fn component_xml(exec: &str, engines: &[Engine]) -> String {
    let entries = engines
        .iter()
        .map(|engine| {
            let icon = engine.icon.as_deref().map_or_else(
                || DEFAULT_ICON.to_owned(),
                |path| path.to_string_lossy().into_owned(),
            );
            format!(
                "    <engine>\n      <name>keymagic:{id}</name>\n      <language>other</language>\n      <license>GPL-2.0-only</license>\n      <author>codewiththiha</author>\n      <icon>{icon}</icon>\n      <layout>default</layout>\n      <longname>{label}</longname>\n      <description>{label}</description>\n      <rank>0</rank>\n    </engine>\n",
                id = escape(&engine.id),
                icon = escape(&icon),
                label = escape(&engine.label),
            )
        })
        .collect::<Vec<String>>()
        .concat();
    format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<component>\n  <name>{COMPONENT_NAME}</name>\n  <description>KeyMagic input method</description>\n  <exec>{exec}</exec>\n  <version>{version}</version>\n  <author>codewiththiha</author>\n  <license>GPL-2.0-only</license>\n  <homepage>https://github.com/codewiththiha/keymagic-rs</homepage>\n  <engines>\n{entries}  </engines>\n</component>\n",
        exec = escape(exec),
        version = env!("CARGO_PKG_VERSION"),
    )
}

/// Where `keymagic install --user` writes the component file.
#[must_use]
pub fn user_component_path(data_home: &Path) -> PathBuf {
    data_home.join("ibus/component/keymagic.xml")
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
