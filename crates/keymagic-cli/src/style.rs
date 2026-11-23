//! Colours, chosen once at start-up from `--color`.

use owo_colors::OwoColorize;
use std::io::IsTerminal;
use std::sync::OnceLock;

static ENABLED: OnceLock<bool> = OnceLock::new();

/// Sets the colour mode: `always`, `never`, or `auto` for a terminal.
pub fn choose(mode: &str) {
    let enabled = match mode {
        "always" => true,
        "never" => false,
        _ => std::io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none(),
    };
    let _ = ENABLED.set(enabled);
}

/// Returns `text` in green when colours are on.
pub fn pass(text: &str) -> String {
    if enabled() {
        text.green().to_string()
    } else {
        text.to_owned()
    }
}

/// Returns `text` in red when colours are on.
pub fn fail(text: &str) -> String {
    if enabled() {
        text.red().to_string()
    } else {
        text.to_owned()
    }
}

fn enabled() -> bool {
    ENABLED.get().copied().unwrap_or(false)
}
