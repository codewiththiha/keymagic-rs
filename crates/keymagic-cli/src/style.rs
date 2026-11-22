//! Colours, on only for a terminal without `NO_COLOR`.

use owo_colors::OwoColorize;
use std::io::IsTerminal;

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
    std::io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none()
}
