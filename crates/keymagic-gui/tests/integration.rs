//! Integration panels: one per operating system, with status for this one.

use keymagic_gui::integration::{Os, panels};

#[test]
fn every_panel_says_not_this_system_except_the_current_one() {
    let statuses: Vec<_> = panels(Os::Linux, None)
        .into_iter()
        .map(|panel| (panel.os, panel.status))
        .collect();
    assert_eq!(statuses[0], (Os::Windows, "Not this system.".to_owned()));
    assert_eq!(statuses[1], (Os::MacOs, "Not this system.".to_owned()));
    assert_eq!(
        statuses[2],
        (
            Os::Linux,
            "Not checked yet. Press Check connection.".to_owned()
        )
    );
}

#[test]
fn the_current_panel_reports_the_connection() {
    let connected = panels(Os::MacOs, Some(true));
    assert_eq!(
        connected[1].status,
        "Connected: an input method is listening."
    );
    let missing = panels(Os::MacOs, Some(false));
    assert!(
        missing[1]
            .status
            .contains("does not register KeyMagic with macOS")
    );
}
