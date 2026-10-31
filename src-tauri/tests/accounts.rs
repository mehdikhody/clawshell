mod support;

use clawshell_lib::core::Status;
use support::Harness;

const ID_A: &str = "3f2b8c1e-5a4d-4e7b-9c10-2d6f8a9b0c11";
const ID_B: &str = "a0b1c2d3-e4f5-4a6b-8c7d-9e0f1a2b3c4d";

fn statuses(h: &Harness) -> Vec<(String, Status)> {
    h.core()
        .list_accounts()
        .unwrap()
        .accounts
        .into_iter()
        .map(|a| (a.id, a.status))
        .collect()
}

#[test]
fn lists_only_clawshell_accounts() {
    let h = Harness::new();
    h.fixture.add_profile(ID_A);
    h.fixture.add_folder("ClaudeProfile-Account 2");
    h.fixture.add_folder("Claude-Data");
    h.fixture.add_folder("clawshell");

    let list = h.core().list_accounts().unwrap();

    let ids: Vec<_> = list.accounts.iter().map(|a| a.id.as_str()).collect();
    assert_eq!(ids, vec![ID_A]);
}

#[test]
fn a_profile_without_a_sign_in_is_signed_out() {
    let h = Harness::new();
    h.fixture.add_profile(ID_A);

    assert_eq!(statuses(&h), vec![(ID_A.to_string(), Status::SignedOut)]);
}

#[test]
fn a_signed_in_profile_is_idle_until_claude_runs_on_it() {
    let h = Harness::new();
    h.fixture.add_profile(ID_A);
    h.fixture.sign_in(ID_A);
    assert_eq!(statuses(&h), vec![(ID_A.to_string(), Status::Idle)]);

    h.processes
        .0
        .lock()
        .unwrap()
        .insert(h.fixture.profile_path(ID_A));
    assert_eq!(statuses(&h), vec![(ID_A.to_string(), Status::Running)]);
}

#[test]
fn running_only_marks_the_profile_claude_is_using() {
    let h = Harness::new();
    h.fixture.add_profile(ID_A);
    h.fixture.add_profile(ID_B);
    h.fixture.sign_in(ID_A);
    h.fixture.sign_in(ID_B);
    h.processes
        .0
        .lock()
        .unwrap()
        .insert(h.fixture.profile_path(ID_B));

    assert_eq!(
        statuses(&h),
        vec![
            (ID_A.to_string(), Status::Idle),
            (ID_B.to_string(), Status::Running)
        ]
    );
}

#[test]
fn signed_out_takes_precedence_over_running() {
    // A freshly added account is running at the sign-in screen: Signed out.
    let h = Harness::new();
    h.fixture.add_profile(ID_A);
    h.processes
        .0
        .lock()
        .unwrap()
        .insert(h.fixture.profile_path(ID_A));

    assert_eq!(statuses(&h), vec![(ID_A.to_string(), Status::SignedOut)]);
}

#[test]
fn a_running_account_reports_running_even_when_signed_out() {
    let h = Harness::new();
    h.fixture.add_profile(ID_A);
    h.processes
        .0
        .lock()
        .unwrap()
        .insert(h.fixture.profile_path(ID_A));

    let list = h.core().list_accounts().unwrap();

    assert!(list.accounts[0].running);
}

#[test]
fn sign_in_falls_back_to_the_token_cache_length() {
    let h = Harness::new();
    h.fixture.add_profile(ID_A);
    h.fixture.add_profile(ID_B);
    let long = "x".repeat(101);
    h.fixture
        .write_config(ID_A, &format!(r#"{{"oauth:tokenCacheV2": "{long}"}}"#));
    h.fixture
        .write_config(ID_B, r#"{"oauth:tokenCacheV2": "short"}"#);

    assert_eq!(
        statuses(&h),
        vec![
            (ID_A.to_string(), Status::Idle),
            (ID_B.to_string(), Status::SignedOut)
        ]
    );
}

#[test]
fn window_size_flag_wins_over_the_token_cache() {
    let h = Harness::new();
    h.fixture.add_profile(ID_A);
    let long = "x".repeat(200);
    h.fixture.write_config(
        ID_A,
        &format!(r#"{{"windowSizeWasSignedIn": false, "oauth:tokenCacheV2": "{long}"}}"#),
    );

    assert_eq!(statuses(&h), vec![(ID_A.to_string(), Status::SignedOut)]);
}

#[test]
fn an_unreadable_config_counts_as_signed_out() {
    let h = Harness::new();
    h.fixture.add_profile(ID_A);
    h.fixture.write_config(ID_A, "not json");

    assert_eq!(statuses(&h), vec![(ID_A.to_string(), Status::SignedOut)]);
}

#[test]
fn reports_whether_claude_desktop_was_found() {
    let h = Harness::new();
    assert!(h.core().list_accounts().unwrap().claude_found);

    *h.os.install.lock().unwrap() = None;
    assert!(!h.core().list_accounts().unwrap().claude_found);
}

// ---- add -------------------------------------------------------------------

use clawshell_lib::core::AddError;

fn folder_names(h: &Harness) -> Vec<String> {
    let mut names: Vec<_> = std::fs::read_dir(h.fixture.local_app_data())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

#[test]
fn add_creates_a_clawshell_uuid_folder_and_launches_claude_on_it() {
    let h = Harness::new();

    let account = h.core().add_account().unwrap();

    let folder = h
        .fixture
        .local_app_data()
        .join(format!("clawshell-{}", account.id));
    assert!(folder.is_dir());
    assert!(uuid::Uuid::parse_str(&account.id).is_ok());
    let launched = h.os.launched.lock().unwrap().clone();
    assert_eq!(
        launched,
        vec![(std::path::PathBuf::from("C:/Claude/app/Claude.exe"), folder)]
    );
}

#[test]
fn a_new_account_lands_on_sign_in() {
    let h = Harness::new();

    let account = h.core().add_account().unwrap();

    assert_eq!(account.status, Status::SignedOut);
}

#[test]
fn every_added_account_gets_its_own_folder() {
    let h = Harness::new();
    let core = h.core();

    let a = core.add_account().unwrap();
    let b = core.add_account().unwrap();

    assert_ne!(a.id, b.id);
    let ids: Vec<_> = core
        .list_accounts()
        .unwrap()
        .accounts
        .into_iter()
        .map(|a| a.id)
        .collect();
    assert_eq!(ids.len(), 2);
}

#[test]
fn new_accounts_are_labelled_account_2_then_3_with_no_duplicates() {
    let h = Harness::new();
    let core = h.core();

    let labels: Vec<_> = (0..3).map(|_| core.add_account().unwrap().label).collect();

    assert_eq!(labels, vec!["Account 2", "Account 3", "Account 4"]);
}

#[test]
fn labels_persist_across_restarts_and_keep_numbering_unique() {
    let h = Harness::new();
    h.core().add_account().unwrap();

    // A fresh Core over the same machine, as after an app restart.
    let again = h.core();
    let labels: Vec<_> = again
        .list_accounts()
        .unwrap()
        .accounts
        .into_iter()
        .map(|a| a.label)
        .collect();
    assert_eq!(labels, vec!["Account 2"]);
    assert_eq!(again.add_account().unwrap().label, "Account 3");
}

#[test]
fn an_unrecorded_clawshell_folder_is_adopted_without_clashing_labels() {
    let h = Harness::new();
    h.fixture.add_profile(ID_A);
    let core = h.core();

    let added = core.add_account().unwrap();

    let labels: Vec<_> = core
        .list_accounts()
        .unwrap()
        .accounts
        .into_iter()
        .map(|a| a.label)
        .collect();
    let mut sorted = labels.clone();
    sorted.sort();
    assert_eq!(sorted, vec!["Account 2", "Account 3"]);
    assert!(labels.contains(&added.label));
}

#[test]
fn add_without_claude_installed_fails_cleanly_and_creates_nothing() {
    let h = Harness::new();
    *h.os.install.lock().unwrap() = None;

    let err = h.core().add_account().unwrap_err();

    assert!(matches!(err, AddError::ClaudeNotFound));
    assert!(folder_names(&h).is_empty());
    assert!(h.os.launched.lock().unwrap().is_empty());
}

#[test]
fn add_never_touches_other_claude_folders() {
    let h = Harness::new();
    h.fixture.add_folder("ClaudeProfile-Account 2");
    h.fixture.add_folder("Claude-Data");

    h.core().add_account().unwrap();

    let names = folder_names(&h);
    assert!(names.contains(&"ClaudeProfile-Account 2".to_string()));
    assert!(names.contains(&"Claude-Data".to_string()));
    assert!(names.contains(&"clawshell".to_string()));
    assert_eq!(names.len(), 4);
}

// ---- launch or focus ---------------------------------------------------------

use clawshell_lib::core::{LaunchError, LaunchOutcome};

fn run_claude_on(h: &Harness, id: &str) {
    h.processes
        .0
        .lock()
        .unwrap()
        .insert(h.fixture.profile_path(id));
}

#[test]
fn launching_a_stopped_account_starts_claude_on_its_folder() {
    let h = Harness::new();
    h.fixture.add_profile(ID_A);
    h.fixture.sign_in(ID_A);

    let outcome = h.core().launch_account(ID_A).unwrap();

    assert_eq!(outcome, LaunchOutcome::Started);
    assert_eq!(
        h.os.launched.lock().unwrap().clone(),
        vec![(
            std::path::PathBuf::from("C:/Claude/app/Claude.exe"),
            h.fixture.profile_path(ID_A)
        )]
    );
}

#[test]
fn launching_a_running_account_focuses_its_window_and_starts_no_second_copy() {
    let h = Harness::new();
    h.fixture.add_profile(ID_A);
    h.fixture.sign_in(ID_A);
    run_claude_on(&h, ID_A);

    let outcome = h.core().launch_account(ID_A).unwrap();

    assert_eq!(outcome, LaunchOutcome::Focused);
    assert_eq!(
        h.os.focused.lock().unwrap().clone(),
        vec![h.fixture.profile_path(ID_A)]
    );
    assert!(h.os.launched.lock().unwrap().is_empty());
}

#[test]
fn a_running_account_with_no_window_is_reopened_through_claude_itself() {
    // Claude hidden in its tray has no window to focus. Starting Claude on the
    // same folder hands over to the running copy (single instance), which
    // shows its window.
    let h = Harness::new();
    h.fixture.add_profile(ID_A);
    h.fixture.sign_in(ID_A);
    run_claude_on(&h, ID_A);
    *h.os.has_window.lock().unwrap() = false;

    let outcome = h.core().launch_account(ID_A).unwrap();

    assert_eq!(outcome, LaunchOutcome::Started);
    assert_eq!(h.os.launched.lock().unwrap().len(), 1);
}

#[test]
fn a_signed_out_running_account_is_focused_too() {
    let h = Harness::new();
    h.fixture.add_profile(ID_A);
    run_claude_on(&h, ID_A);

    assert_eq!(
        h.core().launch_account(ID_A).unwrap(),
        LaunchOutcome::Focused
    );
}

#[test]
fn launching_an_unknown_account_is_refused() {
    let h = Harness::new();
    h.fixture.add_profile(ID_A);

    let err = h.core().launch_account(ID_B).unwrap_err();

    assert!(matches!(err, LaunchError::UnknownAccount));
    assert!(h.os.launched.lock().unwrap().is_empty());
}

#[test]
fn an_id_that_is_not_a_clawshell_profile_is_refused_even_if_the_folder_exists() {
    let h = Harness::new();
    h.fixture.add_folder("ClaudeProfile-Account 2");
    h.fixture.add_folder("Claude-Data");

    for id in ["../Claude-Data", "ClaudeProfile-Account 2", ".."] {
        assert!(matches!(
            h.core().launch_account(id).unwrap_err(),
            LaunchError::UnknownAccount
        ));
    }
    assert!(h.os.launched.lock().unwrap().is_empty());
}

#[test]
fn launching_without_claude_installed_says_so() {
    let h = Harness::new();
    h.fixture.add_profile(ID_A);
    *h.os.install.lock().unwrap() = None;

    let err = h.core().launch_account(ID_A).unwrap_err();

    assert!(matches!(err, LaunchError::ClaudeNotFound));
}
