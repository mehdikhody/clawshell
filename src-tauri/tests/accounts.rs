mod support;

use clawshell_lib::core::AccountError;
use clawshell_lib::core::Status;
use support::{Added, Harness, add_default};

const ID_A: &str = "3f2b8c1e-5a4d-4e7b-9c10-2d6f8a9b0c11";
const ID_B: &str = "a0b1c2d3-e4f5-4a6b-8c7d-9e0f1a2b3c4d";

fn statuses(h: &Harness) -> Vec<(String, Status)> {
    h.core()
        .list_accounts()
        .unwrap()
        .added()
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

    // Claude's own profile is the default account, listed first.
    let ids: Vec<_> = list.accounts.iter().map(|a| a.id.as_str()).collect();
    assert_eq!(ids, vec!["default", ID_A]);
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

    assert!(list.added()[0].running);
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
fn add_creates_a_clawshell_uuid_folder_without_starting_claude() {
    let h = Harness::new();

    let account = add_default(&h.core());

    let folder = h
        .fixture
        .local_app_data()
        .join(format!("clawshell-{}", account.id));
    assert!(folder.is_dir());
    assert!(uuid::Uuid::parse_str(&account.id).is_ok());
    assert!(h.os.launched.lock().unwrap().is_empty());
}

#[test]
fn a_new_account_lands_on_sign_in() {
    let h = Harness::new();

    let account = add_default(&h.core());

    assert_eq!(account.status, Status::SignedOut);
}

#[test]
fn add_uses_the_name_and_colour_it_is_given() {
    let h = Harness::new();

    let account = h.core().add_account("  Work  ", "#DC2626").unwrap();

    assert_eq!(account.label, "Work");
    assert_eq!(account.badge, 'W');
    assert_eq!(account.colour, "#dc2626");
    let listed = h.core().list_accounts().unwrap().added();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].label, "Work");
    assert_eq!(listed[0].colour, "#dc2626");
}

#[test]
fn the_dialog_starts_from_the_default_name_and_colour() {
    let h = Harness::new();

    let defaults = h.core().new_account_defaults().unwrap();

    assert_eq!(defaults.label, "Account 2");
    assert_eq!(defaults.colour, "#2563eb");
    assert!(h.core().list_accounts().unwrap().added().is_empty());
}

#[test]
fn add_refuses_a_blank_name_and_creates_nothing() {
    let h = Harness::new();

    let err = h.core().add_account("   ", "#2563eb").unwrap_err();

    assert!(matches!(err, AddError::EmptyLabel));
    assert!(folder_names(&h).is_empty());
}

#[test]
fn add_refuses_a_colour_that_is_not_hex_and_creates_nothing() {
    let h = Harness::new();

    let err = h.core().add_account("Work", "blue").unwrap_err();

    assert!(matches!(err, AddError::InvalidColour));
    assert!(folder_names(&h).is_empty());
}

#[test]
fn two_accounts_may_share_a_name() {
    let h = Harness::new();
    let core = h.core();

    let a = core.add_account("Work", "#2563eb").unwrap();
    let b = core.add_account("Work", "#dc2626").unwrap();

    assert_ne!(a.id, b.id);
    assert_eq!(core.list_accounts().unwrap().added().len(), 2);
}

#[test]
fn every_added_account_gets_its_own_folder() {
    let h = Harness::new();
    let core = h.core();

    let a = add_default(&core);
    let b = add_default(&core);

    assert_ne!(a.id, b.id);
    let ids: Vec<_> = core
        .list_accounts()
        .unwrap()
        .added()
        .into_iter()
        .map(|a| a.id)
        .collect();
    assert_eq!(ids.len(), 2);
}

#[test]
fn new_accounts_are_labelled_account_2_then_3_with_no_duplicates() {
    let h = Harness::new();
    let core = h.core();

    let labels: Vec<_> = (0..3).map(|_| add_default(&core).label).collect();

    assert_eq!(labels, vec!["Account 2", "Account 3", "Account 4"]);
}

#[test]
fn labels_persist_across_restarts_and_keep_numbering_unique() {
    let h = Harness::new();
    add_default(&h.core());

    // A fresh Core over the same machine, as after an app restart.
    let again = h.core();
    let labels: Vec<_> = again
        .list_accounts()
        .unwrap()
        .added()
        .into_iter()
        .map(|a| a.label)
        .collect();
    assert_eq!(labels, vec!["Account 2"]);
    assert_eq!(add_default(&again).label, "Account 3");
}

#[test]
fn an_unrecorded_clawshell_folder_is_adopted_without_clashing_labels() {
    let h = Harness::new();
    h.fixture.add_profile(ID_A);
    let core = h.core();

    let added = add_default(&core);

    let labels: Vec<_> = core
        .list_accounts()
        .unwrap()
        .added()
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

    let err = h.core().add_account("Work", "#2563eb").unwrap_err();

    assert!(matches!(err, AddError::ClaudeNotFound));
    assert!(folder_names(&h).is_empty());
    assert!(h.os.launched.lock().unwrap().is_empty());
}

#[test]
fn add_never_touches_other_claude_folders() {
    let h = Harness::new();
    h.fixture.add_folder("ClaudeProfile-Account 2");
    h.fixture.add_folder("Claude-Data");

    add_default(&h.core());

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

    assert!(matches!(err, LaunchError::Account(AccountError::Unknown)));
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
            LaunchError::Account(AccountError::Unknown)
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

// ---- rename ------------------------------------------------------------------

use clawshell_lib::core::RenameError;

fn labels(h: &Harness) -> Vec<(String, String)> {
    h.core()
        .list_accounts()
        .unwrap()
        .added()
        .into_iter()
        .map(|a| (a.id, a.label))
        .collect()
}

#[test]
fn rename_changes_the_label_and_survives_a_restart() {
    let h = Harness::new();
    h.fixture.add_profile(ID_A);

    let renamed = h.core().rename_account(ID_A, "Work").unwrap();

    assert_eq!(renamed.label, "Work");
    // A fresh Core over the same machine, as after an app restart.
    assert_eq!(labels(&h), vec![(ID_A.to_string(), "Work".to_string())]);
}

#[test]
fn rename_never_touches_the_folder_or_the_id() {
    let h = Harness::new();
    h.fixture.add_profile(ID_A);
    h.fixture.sign_in(ID_A);
    h.core().list_accounts().unwrap(); // records land in the app-data folder
    let before = folder_names(&h);

    let renamed = h.core().rename_account(ID_A, "Personal").unwrap();

    assert_eq!(renamed.id, ID_A);
    assert_eq!(folder_names(&h), before);
    assert!(h.fixture.profile_path(ID_A).join("config.json").is_file());
    assert!(h.os.launched.lock().unwrap().is_empty());
}

#[test]
fn rename_trims_and_refuses_an_empty_label() {
    let h = Harness::new();
    h.fixture.add_profile(ID_A);

    assert_eq!(
        h.core().rename_account(ID_A, "  Work  ").unwrap().label,
        "Work"
    );
    assert!(matches!(
        h.core().rename_account(ID_A, "   ").unwrap_err(),
        RenameError::EmptyLabel
    ));
    assert_eq!(labels(&h), vec![(ID_A.to_string(), "Work".to_string())]);
}

#[test]
fn rename_only_changes_the_named_account() {
    let h = Harness::new();
    h.fixture.add_profile(ID_A);
    h.fixture.add_profile(ID_B);
    let before = labels(&h);

    h.core().rename_account(ID_B, "Work").unwrap();

    let after = labels(&h);
    assert_eq!(after[0], before[0]);
    assert_eq!(after[1], (ID_B.to_string(), "Work".to_string()));
}

#[test]
fn renaming_an_unknown_account_is_refused() {
    let h = Harness::new();
    h.fixture.add_profile(ID_A);

    assert!(matches!(
        h.core().rename_account(ID_B, "Work").unwrap_err(),
        RenameError::Account(AccountError::Unknown)
    ));
}

#[test]
fn a_renamed_account_frees_its_default_label_for_the_next_one() {
    // Account 2 renamed to Work: the next default label is Account 2 again;
    // no two accounts end up sharing a default label.
    let h = Harness::new();
    let core = h.core();
    let first = add_default(&core);
    core.rename_account(&first.id, "Work").unwrap();

    let second = add_default(&core);

    assert_eq!(second.label, "Account 2");
}

// ---- open folder -------------------------------------------------------------

use clawshell_lib::core::{DeleteError, OpenFolderError};

#[test]
fn open_folder_shows_the_accounts_own_profile_folder() {
    let h = Harness::new();
    h.fixture.add_profile(ID_A);
    h.fixture.add_profile(ID_B);

    h.core().open_account_folder(ID_B).unwrap();

    assert_eq!(
        h.os.opened_folders.lock().unwrap().clone(),
        vec![h.fixture.profile_path(ID_B)]
    );
}

#[test]
fn open_folder_for_an_unknown_account_opens_nothing() {
    let h = Harness::new();
    h.fixture.add_profile(ID_A);

    for id in [ID_B, "..", "../Claude-Data"] {
        assert!(matches!(
            h.core().open_account_folder(id).unwrap_err(),
            OpenFolderError::Account(AccountError::Unknown)
        ));
    }
    assert!(h.os.opened_folders.lock().unwrap().is_empty());
}

#[test]
fn open_folder_for_a_missing_folder_says_so_and_opens_nothing() {
    let h = Harness::new();
    h.fixture.add_profile(ID_A);
    h.core().list_accounts().unwrap();
    std::fs::remove_dir_all(h.fixture.profile_path(ID_A)).unwrap();

    assert!(matches!(
        h.core().open_account_folder(ID_A).unwrap_err(),
        OpenFolderError::FolderMissing
    ));
    assert!(h.os.opened_folders.lock().unwrap().is_empty());
}

// ---- quit --------------------------------------------------------------------

#[test]
fn quit_ends_only_the_processes_of_that_accounts_profile() {
    let h = Harness::new();
    h.fixture.add_profile(ID_A);
    h.fixture.add_profile(ID_B);
    h.processes
        .run(&h.fixture.profile_path(ID_A), &[10, 11, 12]);
    h.processes.run(&h.fixture.profile_path(ID_B), &[20, 21]);

    h.core().quit_account(ID_A).unwrap();

    assert_eq!(h.os.ended.lock().unwrap().clone(), vec![10, 11, 12]);
}

#[test]
fn quit_works_for_the_default_account_too() {
    let h = Harness::new();
    h.fixture.add_profile(ID_A);
    let default = h
        .core()
        .list_accounts()
        .unwrap()
        .accounts
        .into_iter()
        .find(|a| a.is_default)
        .unwrap();
    h.processes.run(&default.path, &[5, 6]);
    h.processes.run(&h.fixture.profile_path(ID_A), &[10]);

    h.core().quit_account("default").unwrap();

    assert_eq!(h.os.ended.lock().unwrap().clone(), vec![5, 6]);
}

#[test]
fn quit_on_an_account_that_is_not_running_ends_nothing() {
    let h = Harness::new();
    h.fixture.add_profile(ID_A);

    h.core().quit_account(ID_A).unwrap();

    assert!(h.os.ended.lock().unwrap().is_empty());
}

#[test]
fn quit_for_an_unknown_account_ends_nothing() {
    let h = Harness::new();
    h.fixture.add_profile(ID_A);
    h.processes.run(&h.fixture.profile_path(ID_A), &[10]);

    for id in [ID_B, "..", "../Claude-Data"] {
        assert!(matches!(
            h.core().quit_account(id).unwrap_err(),
            AccountError::Unknown
        ));
    }
    assert!(h.os.ended.lock().unwrap().is_empty());
}

#[test]
fn quit_reports_a_failure_to_end_the_processes() {
    let h = Harness::new();
    h.fixture.add_profile(ID_A);
    h.processes.run(&h.fixture.profile_path(ID_A), &[10]);
    *h.os.end_fails.lock().unwrap() = true;

    assert!(matches!(
        h.core().quit_account(ID_A).unwrap_err(),
        AccountError::Io(_)
    ));
}

// ---- delete ------------------------------------------------------------------

fn ids(h: &Harness) -> Vec<String> {
    labels(h).into_iter().map(|(id, _)| id).collect()
}

#[test]
fn delete_removes_the_profile_folder_and_the_account_record() {
    let h = Harness::new();
    h.fixture.add_profile(ID_A);
    h.fixture.add_profile(ID_B);
    h.core().rename_account(ID_A, "Work").unwrap();

    h.core().delete_account(ID_A).unwrap();

    assert!(!h.fixture.profile_path(ID_A).exists());
    assert!(h.fixture.profile_path(ID_B).is_dir());
    assert_eq!(ids(&h), vec![ID_B.to_string()]);
    // The record is gone too: a folder re-appearing is adopted afresh.
    h.fixture.add_profile(ID_A);
    let relabelled = labels(&h);
    assert!(relabelled.iter().all(|(_, label)| label != "Work"));
}

#[test]
fn delete_leaves_other_folders_alone() {
    let h = Harness::new();
    h.fixture.add_profile(ID_A);
    h.fixture.add_folder("ClaudeProfile-Account 2");
    h.fixture.add_folder("Claude-Data");

    h.core().delete_account(ID_A).unwrap();

    let names = folder_names(&h);
    assert!(names.contains(&"ClaudeProfile-Account 2".to_string()));
    assert!(names.contains(&"Claude-Data".to_string()));
    assert!(!names.contains(&format!("clawshell-{ID_A}")));
}

#[test]
fn delete_is_refused_while_the_account_is_running() {
    let h = Harness::new();
    h.fixture.add_profile(ID_A);
    h.fixture.sign_in(ID_A);
    run_claude_on(&h, ID_A);

    let err = h.core().delete_account(ID_A).unwrap_err();

    assert!(matches!(err, DeleteError::Running));
    assert!(h.fixture.profile_path(ID_A).join("config.json").is_file());
    assert_eq!(ids(&h), vec![ID_A.to_string()]);
}

#[test]
fn delete_is_refused_while_a_signed_out_account_is_running() {
    let h = Harness::new();
    h.fixture.add_profile(ID_A);
    run_claude_on(&h, ID_A);

    assert!(matches!(
        h.core().delete_account(ID_A).unwrap_err(),
        DeleteError::Running
    ));
    assert!(h.fixture.profile_path(ID_A).is_dir());
}

#[test]
fn deleting_an_unknown_account_removes_nothing() {
    let h = Harness::new();
    h.fixture.add_profile(ID_A);
    h.fixture.add_folder("Claude-Data");

    for id in [ID_B, "..", "../Claude-Data", "Claude-Data"] {
        assert!(matches!(
            h.core().delete_account(id).unwrap_err(),
            DeleteError::Account(AccountError::Unknown)
        ));
    }
    assert!(h.fixture.profile_path(ID_A).is_dir());
    assert!(h.fixture.local_app_data().join("Claude-Data").is_dir());
}

// ---- missing folder ----------------------------------------------------------

#[test]
fn an_account_whose_folder_vanished_is_listed_as_missing() {
    let h = Harness::new();
    h.fixture.add_profile(ID_A);
    h.fixture.add_profile(ID_B);
    h.core().rename_account(ID_A, "Work").unwrap();
    std::fs::remove_dir_all(h.fixture.profile_path(ID_A)).unwrap();

    let accounts = h.core().list_accounts().unwrap().added();

    assert_eq!(accounts.len(), 2);
    assert_eq!(accounts[0].id, ID_A);
    assert_eq!(accounts[0].label, "Work");
    assert_eq!(accounts[0].status, Status::Missing);
    assert!(!accounts[0].running);
    assert_eq!(accounts[1].status, Status::SignedOut);
}

#[test]
fn a_missing_account_can_be_removed_and_leaves_the_list() {
    let h = Harness::new();
    h.fixture.add_profile(ID_A);
    h.core().list_accounts().unwrap();
    std::fs::remove_dir_all(h.fixture.profile_path(ID_A)).unwrap();

    h.core().delete_account(ID_A).unwrap();

    assert!(ids(&h).is_empty());
}

#[test]
fn a_missing_account_can_still_be_renamed() {
    let h = Harness::new();
    h.fixture.add_profile(ID_A);
    h.core().list_accounts().unwrap();
    std::fs::remove_dir_all(h.fixture.profile_path(ID_A)).unwrap();

    h.core().rename_account(ID_A, "Old").unwrap();

    assert_eq!(labels(&h), vec![(ID_A.to_string(), "Old".to_string())]);
}

#[test]
fn a_missing_account_cannot_be_launched() {
    let h = Harness::new();
    h.fixture.add_profile(ID_A);
    h.core().list_accounts().unwrap();
    std::fs::remove_dir_all(h.fixture.profile_path(ID_A)).unwrap();

    assert!(matches!(
        h.core().launch_account(ID_A).unwrap_err(),
        LaunchError::Account(AccountError::Unknown)
    ));
    assert!(h.os.launched.lock().unwrap().is_empty());
}

#[test]
fn a_tampered_record_with_a_path_like_id_is_never_listed_or_deleted() {
    let h = Harness::new();
    h.fixture.add_folder("Claude-Data");
    let dir = h.fixture.local_app_data().join("clawshell");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("accounts.json"),
        r#"{"accounts":[{"id":"../Claude-Data","label":"Evil"}]}"#,
    )
    .unwrap();

    assert!(ids(&h).is_empty());
    assert!(matches!(
        h.core().delete_account("../Claude-Data").unwrap_err(),
        DeleteError::Account(AccountError::Unknown)
    ));
    assert!(h.fixture.local_app_data().join("Claude-Data").is_dir());
}

// ---- saved order -------------------------------------------------------------

const ID_C: &str = "c0c0c0c0-0000-4000-8000-000000000003";

fn ids_in_list(h: &Harness) -> Vec<String> {
    h.core()
        .list_accounts()
        .unwrap()
        .added()
        .into_iter()
        .map(|a| a.id)
        .collect()
}

#[test]
fn accounts_are_listed_in_the_order_they_were_dropped_and_it_survives_a_restart() {
    let h = Harness::new();
    h.fixture.add_profile(ID_A);
    h.fixture.add_profile(ID_B);
    h.fixture.add_profile(ID_C);
    assert_eq!(ids_in_list(&h), vec![ID_A, ID_B, ID_C]);

    h.core()
        .reorder_accounts(&[ID_C.to_string(), ID_A.to_string(), ID_B.to_string()])
        .unwrap();

    assert_eq!(ids_in_list(&h), vec![ID_C, ID_A, ID_B]);
}

#[test]
fn a_reorder_ignores_unknown_ids_and_keeps_unnamed_accounts_last() {
    let h = Harness::new();
    h.fixture.add_profile(ID_A);
    h.fixture.add_profile(ID_B);
    h.fixture.add_profile(ID_C);

    h.core()
        .reorder_accounts(&[
            ID_C.to_string(),
            "not-an-account".to_string(),
            ID_A.to_string(),
        ])
        .unwrap();

    assert_eq!(ids_in_list(&h), vec![ID_C, ID_A, ID_B]);
}

#[test]
fn a_missing_account_keeps_its_saved_place_until_it_is_removed() {
    let h = Harness::new();
    h.fixture.add_profile(ID_A);
    h.fixture.add_profile(ID_B);
    h.core()
        .reorder_accounts(&[ID_B.to_string(), ID_A.to_string()])
        .unwrap();
    std::fs::remove_dir_all(h.fixture.profile_path(ID_B)).unwrap();

    let accounts = h.core().list_accounts().unwrap().added();

    assert_eq!(
        accounts.iter().map(|a| a.id.as_str()).collect::<Vec<_>>(),
        vec![ID_B, ID_A]
    );
    assert_eq!(accounts[0].status, Status::Missing);
}

#[test]
fn a_new_account_goes_to_the_end_of_the_saved_order() {
    let h = Harness::new();
    h.fixture.add_profile(ID_A);
    h.fixture.add_profile(ID_B);
    h.core()
        .reorder_accounts(&[ID_B.to_string(), ID_A.to_string()])
        .unwrap();

    let added = add_default(&h.core());

    assert_eq!(ids_in_list(&h), vec![ID_B, ID_A, added.id.as_str()]);
}

#[test]
fn a_records_file_without_a_saved_order_keeps_the_id_order_it_was_listed_in() {
    let h = Harness::new();
    h.fixture.add_profile(ID_A);
    h.fixture.add_profile(ID_B);
    let dir = h.fixture.local_app_data().join("clawshell");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("accounts.json"),
        format!(
            r#"{{"accounts":[{{"id":"{ID_B}","label":"Second"}},{{"id":"{ID_A}","label":"First"}}]}}"#
        ),
    )
    .unwrap();

    assert_eq!(ids_in_list(&h), vec![ID_A, ID_B]);
}
