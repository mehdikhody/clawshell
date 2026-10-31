mod support;

use clawshell_lib::core::AccountError;
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
        .accounts
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
    let first = core.add_account().unwrap();
    core.rename_account(&first.id, "Work").unwrap();

    let second = core.add_account().unwrap();

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

    let accounts = h.core().list_accounts().unwrap().accounts;

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
