//! The app picker's name lists, without reading the process table.

use keymagic_gui::apps::{choices, normalise};

#[test]
fn names_are_sorted_and_repeats_and_blanks_are_dropped() {
    let names = ["Notepad", "chrome", "", "Chrome"].map(String::from);
    assert_eq!(normalise(names), ["chrome", "Notepad"]);
}

#[test]
fn choices_skip_names_already_listed_in_any_case() {
    let running = ["Chrome", "Slack", "Zoom"].map(String::from);
    let listed = ["chrome".to_owned()];
    assert_eq!(choices(&running, &listed), ["Slack", "Zoom"]);
}
