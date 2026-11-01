//! An account's Code sessions, through the core: given the session records on
//! disk, what does the listing report?
//!
//! Ids and titles in these fixtures are synthetic.

mod support;

use clawshell_lib::core::{AccountError, CodeSession, SessionsError};
use std::fs;
use support::Harness;

const ID_A: &str = "3f2b8c1e-5a4d-4e7b-9c10-2d6f8a9b0c11";
const ACCOUNT: &str = "11111111-2222-4333-8444-555555555555";
const ORG: &str = "66666666-7777-4888-9999-000000000000";
const OTHER_ACCOUNT: &str = "ffffffff-0000-4111-8222-333333333333";
const OTHER_ORG: &str = "aaaaaaaa-bbbb-4ccc-8ddd-eeeeeeeeeeee";

/// Writes `local_{cli_id}.json` below `{account}/{org}` of the profile.
fn write_record(h: &Harness, account: &str, org: &str, cli_id: &str, body: &str) {
    let dir = h
        .fixture
        .profile_path(ID_A)
        .join("claude-code-sessions")
        .join(account)
        .join(org);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join(format!("local_{cli_id}.json")), body).unwrap();
}

fn record(cli_id: &str, title: &str, at_ms: i64) -> String {
    format!(
        r#"{{"sessionId":"local_{cli_id}","cliSessionId":"{cli_id}","title":"{title}","cwd":"C:\\work\\app","lastActivityAt":{at_ms},"isArchived":false}}"#
    )
}

fn ids(sessions: &[CodeSession]) -> Vec<&str> {
    sessions.iter().map(|s| s.cli_session_id.as_str()).collect()
}

#[test]
fn a_session_is_listed_with_its_title_folder_and_last_activity() {
    let h = Harness::new();
    h.fixture.add_profile(ID_A);
    write_record(
        &h,
        ACCOUNT,
        ORG,
        "cli-1",
        &record("cli-1", "Fix the build", 1_000),
    );

    let sessions = h.core().list_sessions(ID_A).unwrap();

    assert_eq!(
        sessions,
        vec![CodeSession {
            cli_session_id: "cli-1".into(),
            title: "Fix the build".into(),
            cwd: "C:\\work\\app".into(),
            last_activity_at: 1_000,
            is_archived: false,
        }]
    );
}

#[test]
fn sessions_of_every_account_and_organisation_folder_are_listed() {
    let h = Harness::new();
    h.fixture.add_profile(ID_A);
    write_record(&h, ACCOUNT, ORG, "cli-1", &record("cli-1", "One", 3));
    write_record(&h, ACCOUNT, OTHER_ORG, "cli-2", &record("cli-2", "Two", 2));
    write_record(
        &h,
        OTHER_ACCOUNT,
        ORG,
        "cli-3",
        &record("cli-3", "Three", 1),
    );

    let sessions = h.core().list_sessions(ID_A).unwrap();

    assert_eq!(ids(&sessions), ["cli-1", "cli-2", "cli-3"]);
}

#[test]
fn removed_sessions_in_deleted_folders_are_skipped() {
    let h = Harness::new();
    h.fixture.add_profile(ID_A);
    write_record(&h, ACCOUNT, ORG, "cli-1", &record("cli-1", "Kept", 2));
    write_record(
        &h,
        ACCOUNT,
        "deleted_2026-05-01",
        "cli-2",
        &record("cli-2", "Gone", 1),
    );
    write_record(&h, "deleted_old", ORG, "cli-3", &record("cli-3", "Gone", 1));

    let sessions = h.core().list_sessions(ID_A).unwrap();

    assert_eq!(ids(&sessions), ["cli-1"]);
}

#[test]
fn an_archived_session_is_marked_archived() {
    let h = Harness::new();
    h.fixture.add_profile(ID_A);
    let body = record("cli-1", "Old work", 1).replace("false", "true");
    write_record(&h, ACCOUNT, ORG, "cli-1", &body);

    let sessions = h.core().list_sessions(ID_A).unwrap();

    assert!(sessions[0].is_archived);
}

#[test]
fn the_newest_activity_comes_first() {
    let h = Harness::new();
    h.fixture.add_profile(ID_A);
    write_record(&h, ACCOUNT, ORG, "cli-old", &record("cli-old", "Old", 100));
    write_record(
        &h,
        ACCOUNT,
        OTHER_ORG,
        "cli-new",
        &record("cli-new", "New", 300),
    );
    write_record(
        &h,
        OTHER_ACCOUNT,
        ORG,
        "cli-mid",
        &record("cli-mid", "Mid", 200),
    );

    let sessions = h.core().list_sessions(ID_A).unwrap();

    assert_eq!(ids(&sessions), ["cli-new", "cli-mid", "cli-old"]);
}

#[test]
fn a_session_whose_cli_id_claude_cleared_is_listed_under_its_own_id() {
    let h = Harness::new();
    h.fixture.add_profile(ID_A);
    write_record(&h, ACCOUNT, ORG, "cli-1", &record("cli-1", "Fine", 1));
    write_record(
        &h,
        ACCOUNT,
        ORG,
        "cleared",
        r#"{"sessionId":"local_cleared","priorCliSessionIds":["cli-old"],"title":"Cleared","lastActivityAt":2}"#,
    );

    let sessions = h.core().list_sessions(ID_A).unwrap();

    assert_eq!(ids(&sessions), ["local_cleared", "cli-1"]);
    assert_eq!(sessions[0].title, "Cleared");
}

#[test]
fn a_record_that_does_not_read_as_a_session_fails_the_whole_listing() {
    for bad in [
        "not json at all",
        r#"{"title":"No id","lastActivityAt":1}"#,
        r#"{"cliSessionId":"cli-2","title":"No activity"}"#,
    ] {
        let h = Harness::new();
        h.fixture.add_profile(ID_A);
        write_record(&h, ACCOUNT, ORG, "cli-1", &record("cli-1", "Fine", 1));
        write_record(&h, ACCOUNT, ORG, "cli-2", bad);

        let result = h.core().list_sessions(ID_A);

        assert!(
            matches!(result, Err(SessionsError::LayoutChanged)),
            "{bad}: {result:?}"
        );
    }
}

#[test]
fn an_account_that_never_opened_the_code_tab_has_no_sessions() {
    let h = Harness::new();
    h.fixture.add_profile(ID_A);

    assert_eq!(h.core().list_sessions(ID_A).unwrap(), []);
}

#[test]
fn an_unknown_account_is_refused() {
    let h = Harness::new();

    let result = h.core().list_sessions(ID_A);

    assert!(matches!(
        result,
        Err(SessionsError::Account(AccountError::Unknown))
    ));
}

#[test]
fn a_session_kept_under_two_organisations_is_listed_once_at_its_latest_activity() {
    let h = Harness::new();
    h.fixture.add_profile(ID_A);
    write_record(&h, ACCOUNT, ORG, "cli-1", &record("cli-1", "Older", 100));
    write_record(
        &h,
        ACCOUNT,
        OTHER_ORG,
        "cli-1",
        &record("cli-1", "Newer", 200),
    );

    let sessions = h.core().list_sessions(ID_A).unwrap();

    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].title, "Newer");
}
