mod support;

use clawshell_lib::core::launch_options::{EnvVar, LaunchOptions};
use clawshell_lib::core::link::LinkError;
use clawshell_lib::core::ports::ClaudeInstall;
use clawshell_lib::core::routing::Target;
use clawshell_lib::core::{RouteError, RoutingError};
use std::path::PathBuf;
use support::{CLAUDE_HANDLER, CLAWSHELL_HANDLER, Harness};

const ID_A: &str = "3f2b8c1e-5a4d-4e7b-9c10-2d6f8a9b0c11";
const ID_B: &str = "a0b1c2d3-e4f5-4a6b-8c7d-9e0f1a2b3c4d";
const LINK: &str = "claude://login/callback?code=one-time-code";

fn claude_state() -> (Option<String>, bool) {
    (Some(CLAUDE_HANDLER.to_string()), false)
}

fn store_build(h: &Harness) {
    *h.os.install.lock().unwrap() = Some(ClaudeInstall {
        exe: PathBuf::from("C:/Program Files/WindowsApps/Claude_1.0_x64__abc/app/Claude.exe"),
        root: PathBuf::from("C:/Program Files/WindowsApps/Claude_1.0_x64__abc"),
    });
}

fn run(h: &Harness, id: &str) {
    h.processes
        .0
        .lock()
        .unwrap()
        .insert(h.fixture.profile_path(id));
}

fn forwarded_profile(h: &Harness) -> Option<String> {
    let all = h.os.forwarded.lock().unwrap();
    all.last()?
        .args
        .iter()
        .find_map(|a| a.strip_prefix("--user-data-dir=").map(str::to_string))
}

// Turning routing on and off

#[test]
fn routing_starts_off_and_the_registry_untouched() {
    let h = Harness::new();
    assert!(!h.core().settings().sign_in_routing);
    assert!(h.os.registry_calls.lock().unwrap().is_empty());
}

#[test]
fn turning_routing_on_registers_clawshell_after_backing_up_claudes_handler() {
    let h = Harness::new();
    let core = h.core();

    let change = core.set_sign_in_routing(true).unwrap();

    assert!(change.settings.sign_in_routing);
    assert!(core.settings().sign_in_routing);
    assert_eq!(
        h.os.registry_state(),
        (Some(CLAWSHELL_HANDLER.to_string()), true)
    );
    assert_eq!(
        *h.os.registry_calls.lock().unwrap(),
        ["snapshot", "register"]
    );
}

#[test]
fn the_installer_build_needs_no_default_apps_step() {
    let h = Harness::new();

    let change = h.core().set_sign_in_routing(true).unwrap();

    assert!(!change.needs_default_apps_choice);
    assert_eq!(*h.os.default_apps_opened.lock().unwrap(), 0);
}

#[test]
fn the_store_build_opens_default_apps_and_says_a_choice_is_left() {
    let h = Harness::new();
    store_build(&h);

    let change = h.core().set_sign_in_routing(true).unwrap();

    assert!(change.needs_default_apps_choice);
    assert_eq!(*h.os.default_apps_opened.lock().unwrap(), 1);
}

#[test]
fn turning_routing_off_restores_claudes_handler_exactly() {
    let h = Harness::new();
    let before = h.os.registry_state();
    let core = h.core();
    core.set_sign_in_routing(true).unwrap();

    let change = core.set_sign_in_routing(false).unwrap();

    assert!(!change.settings.sign_in_routing);
    assert!(!core.settings().sign_in_routing);
    assert_eq!(h.os.registry_state(), before);
}

#[test]
fn turning_routing_off_restores_a_missing_handler_as_missing() {
    let h = Harness::new();
    *h.os.handler.lock().unwrap() = None;
    let core = h.core();
    core.set_sign_in_routing(true).unwrap();
    assert_eq!(h.os.registry_state().0.as_deref(), Some(CLAWSHELL_HANDLER));

    core.set_sign_in_routing(false).unwrap();

    assert_eq!(h.os.registry_state(), (None, false));
}

#[test]
fn the_backup_survives_a_restart() {
    let h = Harness::new();
    h.core().set_sign_in_routing(true).unwrap();

    // A fresh core, as after quitting and starting clawshell again.
    h.core().set_sign_in_routing(false).unwrap();

    assert_eq!(h.os.registry_state(), claude_state());
}

#[test]
fn turning_routing_on_again_keeps_the_first_backup() {
    let h = Harness::new();
    let core = h.core();
    core.set_sign_in_routing(true).unwrap();

    core.set_sign_in_routing(true).unwrap();
    core.set_sign_in_routing(false).unwrap();

    assert_eq!(h.os.registry_state(), claude_state());
}

#[test]
fn a_refused_registration_leaves_routing_off_and_the_registry_as_it_was() {
    let h = Harness::new();
    *h.os.register_fails.lock().unwrap() = true;
    let core = h.core();

    let result = core.set_sign_in_routing(true);

    assert!(matches!(result, Err(RoutingError::Registry(_))));
    assert!(!core.settings().sign_in_routing);
    assert_eq!(h.os.registry_state(), claude_state());
    // No stale backup is left: a later success takes a fresh one.
    *h.os.register_fails.lock().unwrap() = false;
    h.os.claude_rewrites_handler("Claude-new.exe \"%1\"");
    core.set_sign_in_routing(true).unwrap();
    core.set_sign_in_routing(false).unwrap();
    assert_eq!(
        h.os.registry_state(),
        (Some("Claude-new.exe \"%1\"".to_string()), false)
    );
}

#[test]
fn a_refused_restore_keeps_routing_on() {
    let h = Harness::new();
    let core = h.core();
    core.set_sign_in_routing(true).unwrap();
    *h.os.restore_fails.lock().unwrap() = true;

    let result = core.set_sign_in_routing(false);

    assert!(matches!(result, Err(RoutingError::Registry(_))));
    assert!(core.settings().sign_in_routing);
    // Nothing is lost: once the registry allows it, the restore still works.
    *h.os.restore_fails.lock().unwrap() = false;
    core.set_sign_in_routing(false).unwrap();
    assert_eq!(h.os.registry_state(), claude_state());
}

#[test]
fn a_general_settings_save_does_not_flip_routing() {
    let h = Harness::new();
    let core = h.core();
    let mut stale = core.settings();
    core.set_sign_in_routing(true).unwrap();

    stale.close_to_tray = false;
    let saved = core.update_settings(stale).unwrap();

    assert!(saved.sign_in_routing);
    assert!(core.settings().sign_in_routing);
    assert!(!core.settings().close_to_tray);
}

#[test]
fn any_build_opens_default_apps_when_another_app_is_chosen_for_links() {
    let h = Harness::new();
    *h.os.chosen_elsewhere.lock().unwrap() = true;

    let change = h.core().set_sign_in_routing(true).unwrap();

    assert!(change.needs_default_apps_choice);
    assert_eq!(*h.os.default_apps_opened.lock().unwrap(), 1);
}

// The Default Apps choice

#[test]
fn routing_needs_the_choice_while_another_app_gets_the_links() {
    let h = Harness::new();
    let core = h.core();
    core.set_sign_in_routing(true).unwrap();
    *h.os.chosen_elsewhere.lock().unwrap() = true;

    assert!(core.routing_needs_choice());

    // The user picks clawshell on the Default Apps page.
    *h.os.chosen_elsewhere.lock().unwrap() = false;
    assert!(!core.routing_needs_choice());
}

#[test]
fn routing_that_is_off_needs_no_choice() {
    let h = Harness::new();
    *h.os.chosen_elsewhere.lock().unwrap() = true;

    assert!(!h.core().routing_needs_choice());
}

#[test]
fn the_default_apps_page_opens_on_request() {
    let h = Harness::new();

    h.core().open_default_apps().unwrap();

    assert_eq!(*h.os.default_apps_opened.lock().unwrap(), 1);
}

// Handler upkeep

#[test]
fn clawshell_takes_the_handler_back_when_claude_rewrites_it() {
    let h = Harness::new();
    let core = h.core();
    core.set_sign_in_routing(true).unwrap();
    h.os.claude_rewrites_handler("Claude-v2.exe \"%1\"");

    core.keep_link_handler();

    assert_eq!(
        h.os.registry_state(),
        (Some(CLAWSHELL_HANDLER.to_string()), true)
    );
    // Turning routing off now restores what Claude wrote, not the older backup.
    core.set_sign_in_routing(false).unwrap();
    assert_eq!(
        h.os.registry_state(),
        (Some("Claude-v2.exe \"%1\"".to_string()), false)
    );
}

#[test]
fn upkeep_does_nothing_while_clawshell_still_holds_the_handler() {
    let h = Harness::new();
    let core = h.core();
    core.set_sign_in_routing(true).unwrap();
    let calls = h.os.registry_calls.lock().unwrap().len();

    core.keep_link_handler();

    assert_eq!(h.os.registry_calls.lock().unwrap().len(), calls);
}

#[test]
fn upkeep_does_nothing_while_routing_is_off() {
    let h = Harness::new();
    h.os.claude_rewrites_handler("Claude-v2.exe \"%1\"");

    h.core().keep_link_handler();

    assert!(h.os.registry_calls.lock().unwrap().is_empty());
    assert_eq!(
        h.os.registry_state(),
        (Some("Claude-v2.exe \"%1\"".to_string()), false)
    );
}

// The offer on the first Add

#[test]
fn routing_is_offered_once() {
    let h = Harness::new();
    let core = h.core();

    assert!(core.take_routing_offer());
    assert!(!core.take_routing_offer());
    // Not even after a restart.
    assert!(!h.core().take_routing_offer());
}

#[test]
fn routing_is_not_offered_when_it_is_already_on() {
    let h = Harness::new();
    let core = h.core();
    core.set_sign_in_routing(true).unwrap();

    assert!(!core.take_routing_offer());
}

// Routing a link

#[test]
fn a_link_goes_to_the_account_just_added_while_it_is_signed_out() {
    let h = Harness::new();
    let core = h.core();
    core.set_sign_in_routing(true).unwrap();
    let added = core.add_account().unwrap();
    // Another account is open and signed in.
    h.fixture.add_profile(ID_B);
    h.fixture.sign_in(ID_B);
    run(&h, ID_B);

    let target = core.route_link(LINK).unwrap();

    assert_eq!(target, Target::Account(added.id));
    assert_eq!(
        forwarded_profile(&h),
        Some(added.path.display().to_string())
    );
    let forwarded = h.os.forwarded.lock().unwrap();
    assert_eq!(forwarded[0].link.as_str(), LINK);
    assert_eq!(forwarded[0].exe, PathBuf::from("C:/Claude/app/Claude.exe"));
}

#[test]
fn a_link_goes_to_the_account_launched_while_signed_out() {
    let h = Harness::new();
    let core = h.core();
    h.fixture.add_profile(ID_A);
    h.fixture.add_profile(ID_B);
    h.fixture.sign_in(ID_B);
    run(&h, ID_B);
    core.launch_account(ID_A).unwrap();
    core.launch_account(ID_B).unwrap();
    core.set_sign_in_routing(true).unwrap();

    assert_eq!(core.route_link(LINK).unwrap(), Target::Account(ID_A.into()));
}

#[test]
fn a_link_goes_to_the_only_open_signed_out_account() {
    let h = Harness::new();
    let core = h.core();
    h.fixture.add_profile(ID_A);
    h.fixture.add_profile(ID_B);
    h.fixture.sign_in(ID_B);
    run(&h, ID_A); // opened outside clawshell, signed out
    run(&h, ID_B);
    core.set_sign_in_routing(true).unwrap();

    assert_eq!(core.route_link(LINK).unwrap(), Target::Account(ID_A.into()));
}

#[test]
fn a_link_goes_to_the_window_used_last() {
    let h = Harness::new();
    let core = h.core();
    for id in [ID_A, ID_B] {
        h.fixture.add_profile(id);
        h.fixture.sign_in(id);
        run(&h, id);
    }
    core.launch_account(ID_A).unwrap();
    core.launch_account(ID_B).unwrap();
    core.set_sign_in_routing(true).unwrap();

    assert_eq!(core.route_link(LINK).unwrap(), Target::Account(ID_B.into()));

    core.launch_account(ID_A).unwrap();
    assert_eq!(core.route_link(LINK).unwrap(), Target::Account(ID_A.into()));
}

#[test]
fn a_link_goes_to_the_original_claude_when_no_account_qualifies() {
    let h = Harness::new();
    let core = h.core();
    h.fixture.add_profile(ID_A);
    h.fixture.sign_in(ID_A);
    core.set_sign_in_routing(true).unwrap();

    assert_eq!(core.route_link(LINK).unwrap(), Target::OriginalClaude);

    let forwarded = h.os.forwarded.lock().unwrap();
    assert!(forwarded[0].args.is_empty());
    assert!(forwarded[0].env.is_empty());
    assert_eq!(forwarded[0].exe, PathBuf::from("C:/Claude/app/Claude.exe"));
}

#[test]
fn a_link_with_routing_off_goes_to_the_original_claude() {
    let h = Harness::new();
    let core = h.core();
    h.fixture.add_profile(ID_A);
    core.launch_account(ID_A).unwrap();

    assert_eq!(core.route_link(LINK).unwrap(), Target::OriginalClaude);
}

#[test]
fn a_routed_link_starts_the_account_with_its_flags_and_variables() {
    let h = Harness::new();
    let core = h.core();
    h.fixture.add_profile(ID_A);
    core.set_launch_options(
        ID_A,
        LaunchOptions {
            env: vec![EnvVar {
                name: "HTTPS_PROXY".into(),
                value: "http://proxy.test:8080".into(),
            }],
            flags: vec!["--disable-gpu".into()],
        },
    )
    .unwrap();
    core.launch_account(ID_A).unwrap();
    core.set_sign_in_routing(true).unwrap();

    core.route_link(LINK).unwrap();

    let forwarded = h.os.forwarded.lock().unwrap();
    assert_eq!(
        forwarded[0].args.last().map(String::as_str),
        Some("--disable-gpu")
    );
    assert_eq!(
        forwarded[0].env,
        [(
            "HTTPS_PROXY".to_string(),
            "http://proxy.test:8080".to_string()
        )]
    );
}

#[test]
fn links_that_are_not_claude_links_are_refused_and_go_nowhere() {
    let h = Harness::new();
    let core = h.core();
    core.set_sign_in_routing(true).unwrap();

    for raw in ["https://claude.ai/login", "claude://bad link", "", "--flag"] {
        let result = core.route_link(raw);
        assert!(matches!(result, Err(RouteError::Refused(_))), "{raw:?}");
    }

    assert!(h.os.forwarded.lock().unwrap().is_empty());
}

#[test]
fn a_refusal_names_the_reason_but_never_the_link() {
    let h = Harness::new();
    let core = h.core();

    let error = core.route_link("claude://secret-code bad").unwrap_err();

    assert!(matches!(error, RouteError::Refused(LinkError::Malformed)));
    assert!(!format!("{error:?}").contains("secret"));
}

#[test]
fn a_link_cannot_be_routed_without_claude_installed() {
    let h = Harness::new();
    *h.os.install.lock().unwrap() = None;

    let result = h.core().route_link(LINK);

    assert!(matches!(result, Err(RouteError::ClaudeNotFound)));
}
