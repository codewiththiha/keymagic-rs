//! Integration panels: one set per operating system, with status for this one.

use keymagic_gui::integration::{Health, Os, panels};

#[test]
fn linux_shows_four_cards_with_repairs_and_no_probe_yet() {
    let cards = panels(Os::Linux, None, None);
    let titles: Vec<_> = cards.iter().map(|card| card.title).collect();
    assert_eq!(
        titles,
        [
            "Linux: IBus",
            "Linux: Wayland",
            "Linux: KDE Plasma",
            "Linux: Fcitx5"
        ]
    );
    assert!(cards.iter().all(|card| card.os == Os::Linux
        && card.health == Health::Unchecked
        && card.fix.is_some()));
}

#[test]
fn linux_cards_report_each_probe_result() {
    let cards = panels(Os::Linux, None, Some(&[true, false, true, false]));
    let health: Vec<_> = cards.iter().map(|card| card.health).collect();
    assert_eq!(
        health,
        [
            Health::Ready,
            Health::Missing,
            Health::Ready,
            Health::Missing
        ]
    );
}

#[test]
fn windows_and_macos_panels_say_not_this_system() {
    let cards = panels(Os::MacOs, None, None);
    assert_eq!(cards[0].os, Os::Windows);
    assert_eq!(cards[0].status, "Not this system.");
    assert_eq!(cards[1].os, Os::MacOs);
    assert_eq!(cards[1].status, "Not checked yet. Press Check connection.");
}

#[test]
fn the_current_panel_reports_the_connection() {
    let connected = panels(Os::MacOs, Some(true), None);
    assert_eq!(
        connected[1].status,
        "Connected: an input method is listening."
    );
    let missing = panels(Os::MacOs, Some(false), None);
    assert!(
        missing[1]
            .status
            .contains("does not register KeyMagic with macOS")
    );
}

#[test]
fn windows_and_macos_offer_a_control_that_waits_for_the_install() {
    let windows = panels(Os::Windows, None, None);
    let macos = panels(Os::MacOs, None, None);
    assert!(windows.iter().all(|card| card.deferred.is_some()));
    assert!(macos.iter().all(|card| card.deferred.is_some()));
    let action = windows.first().and_then(|card| card.deferred);
    assert_eq!(
        action.map(|deferred| deferred.action),
        Some("Apply language profiles")
    );
    assert!(
        panels(Os::Linux, None, None)
            .iter()
            .all(|card| card.deferred.is_none())
    );
}
