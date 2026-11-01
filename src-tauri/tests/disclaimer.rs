//! The first-use disclaimer: whether it is accepted, and what waits for it.

mod support;

use clawshell_lib::core::{Core, DISCLAIMER_VERSION};
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

/// Starts the background work with a stand-in that succeeds, and returns the
/// held arguments in the order they were replayed.
fn start(core: &Core) -> Vec<Vec<String>> {
    let mut replayed = Vec::new();
    core.start_background_once(|| Ok::<(), String>(()), |held| replayed.push(held))
        .unwrap();
    replayed
}

#[test]
fn the_background_work_starts_exactly_once() {
    let h = Harness::new();
    let core = h.core();
    let mut starts = 0;

    for _ in 0..2 {
        core.start_background_once(
            || {
                starts += 1;
                Ok::<(), String>(())
            },
            |_| {},
        )
        .unwrap();
    }

    assert_eq!(starts, 1);
}

#[test]
fn held_arguments_come_back_in_order_when_the_background_work_starts() {
    let h = Harness::new();
    let core = h.core();
    assert!(core.hold_launch_args(args(&["clawshell.exe", "--open-account", "a"])));
    assert!(core.hold_launch_args(args(&["clawshell.exe", "claude://login"])));

    assert_eq!(
        start(&core),
        vec![
            args(&["clawshell.exe", "--open-account", "a"]),
            args(&["clawshell.exe", "claude://login"]),
        ]
    );
}

#[test]
fn a_second_instance_that_arrives_early_does_not_jump_the_first_launch() {
    // The first launch queues its own arguments before the core is shared, so
    // a second instance that races setup can only queue behind them.
    let h = Harness::new();
    let core = h.core();
    core.hold_launch_args(args(&["first launch"]));
    core.hold_launch_args(args(&["second instance"]));

    assert_eq!(
        start(&core),
        vec![args(&["first launch"]), args(&["second instance"])]
    );
}

#[test]
fn arguments_are_replayed_once_and_later_ones_are_not_held() {
    let h = Harness::new();
    let core = h.core();
    core.hold_launch_args(args(&["first"]));
    start(&core);

    assert!(start(&core).is_empty());
    assert!(!core.hold_launch_args(args(&["late"])));
}

#[test]
fn a_failed_start_keeps_the_held_arguments_and_a_retry_runs_it() {
    let h = Harness::new();
    let core = h.core();
    core.hold_launch_args(args(&["first"]));

    let failed = core.start_background_once(|| Err("no tray"), |_| panic!("replayed"));

    assert_eq!(failed, Err("no tray"));
    assert!(core.hold_launch_args(args(&["second"])));
    assert_eq!(start(&core), vec![args(&["first"]), args(&["second"])]);
}
