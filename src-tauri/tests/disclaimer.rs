//! The first-use disclaimer: whether it is accepted, and what waits for it.

mod support;

use clawshell_lib::core::DISCLAIMER_VERSION;
use support::Harness;

fn write_settings(h: &Harness, json: &str) {
    let dir = h.fixture.local_app_data().join("clawshell");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("settings.json"), json).unwrap();
}

#[test]
fn the_disclaimer_is_unaccepted_with_no_settings_file() {
    let h = Harness::new();

    let status = h.core().disclaimer_status();

    assert!(!status.accepted);
    assert_eq!(status.version, DISCLAIMER_VERSION);
}

#[test]
fn the_disclaimer_is_accepted_at_the_current_version() {
    let h = Harness::new();
    write_settings(
        &h,
        &format!(r#"{{"disclaimerAccepted": {DISCLAIMER_VERSION}}}"#),
    );

    assert!(h.core().disclaimer_status().accepted);
}

#[test]
fn the_disclaimer_is_unaccepted_again_after_a_version_bump() {
    let h = Harness::new();
    write_settings(
        &h,
        &format!(r#"{{"disclaimerAccepted": {}}}"#, DISCLAIMER_VERSION - 1),
    );

    assert!(!h.core().disclaimer_status().accepted);
}

fn args(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| s.to_string()).collect()
}

#[test]
fn accepting_saves_the_current_version() {
    let h = Harness::new();

    h.core().accept_disclaimer().unwrap();

    assert!(h.core().disclaimer_status().accepted);
}

#[test]
fn accepting_keeps_the_other_settings() {
    let h = Harness::new();
    let core = h.core();
    let mut settings = core.settings();
    settings.start_hidden = true;
    core.update_settings(settings).unwrap();

    h.core().accept_disclaimer().unwrap();

    assert!(h.core().settings().start_hidden);
}

#[test]
fn a_general_settings_save_leaves_the_disclaimer_as_it_was() {
    let h = Harness::new();
    let core = h.core();
    core.accept_disclaimer().unwrap();
    let mut stale = core.settings();
    stale.disclaimer_accepted = None;

    core.update_settings(stale).unwrap();

    assert!(h.core().disclaimer_status().accepted);
}

#[test]
fn accepting_starts_the_background_work_exactly_once() {
    let h = Harness::new();
    let core = h.core();

    assert!(core.accept_disclaimer().unwrap().start_background);
    assert!(!core.accept_disclaimer().unwrap().start_background);
}

#[test]
fn an_accepted_launch_starts_the_background_work_exactly_once() {
    let h = Harness::new();
    h.core().accept_disclaimer().unwrap();
    let core = h.core();

    assert!(core.start_background_once());
    assert!(!core.start_background_once());
    assert!(!core.accept_disclaimer().unwrap().start_background);
}

#[test]
fn held_arguments_come_back_in_order_when_the_disclaimer_is_accepted() {
    let h = Harness::new();
    let core = h.core();
    assert!(core.hold_launch_args(args(&["clawshell.exe", "--open-account", "a"])));
    assert!(core.hold_launch_args(args(&["clawshell.exe", "claude://login"])));

    let accepted = core.accept_disclaimer().unwrap();

    assert_eq!(
        accepted.held,
        vec![
            args(&["clawshell.exe", "--open-account", "a"]),
            args(&["clawshell.exe", "claude://login"]),
        ]
    );
}

#[test]
fn arguments_are_replayed_once_and_later_ones_are_not_held() {
    let h = Harness::new();
    let core = h.core();
    core.hold_launch_args(args(&["first"]));
    core.accept_disclaimer().unwrap();

    assert!(core.accept_disclaimer().unwrap().held.is_empty());
    assert!(!core.hold_launch_args(args(&["late"])));
}

#[test]
fn arguments_are_not_held_once_the_background_work_has_started() {
    let h = Harness::new();
    h.core().accept_disclaimer().unwrap();
    let core = h.core();
    core.start_background_once();

    assert!(!core.hold_launch_args(args(&["now"])));
}
