//! Copying Code sessions between accounts, through the core: given the
//! session records on disk and what the profile endpoint said about the
//! target, what lands where?
//!
//! Tokens, uuids and titles in these fixtures are synthetic.

mod support;

use clawshell_lib::core::usage::{PROFILE_URL, USAGE_URL};
use clawshell_lib::core::{CopyBlocker, CopyOutcome, SessionsError};
use std::fs;
use std::path::PathBuf;
use support::Harness;

const SOURCE: &str = "3f2b8c1e-5a4d-4e7b-9c10-2d6f8a9b0c11";
const TARGET: &str = "9a8b7c6d-5e4f-4a3b-8c2d-1e0f9a8b7c6d";
const TOKEN: &str = "sk-test-token-bbbb";
const NOW_MS: i64 = 1_781_524_800_000;
const HOUR_MS: i64 = 3_600_000;
/// The account and organisation the target is signed into now.
const ACCOUNT: &str = "11111111-2222-4333-8444-555555555555";
const ORG: &str = "66666666-7777-4888-9999-000000000000";
/// Folders the source keeps its records in, and an older organisation of
/// the target.
const SOURCE_ACCOUNT: &str = "ffffffff-0000-4111-8222-333333333333";
const OTHER_ORG: &str = "aaaaaaaa-bbbb-4ccc-8ddd-eeeeeeeeeeee";

fn folder(h: &Harness, id: &str, account: &str, org: &str) -> PathBuf {
    h.fixture
        .profile_path(id)
        .join("claude-code-sessions")
        .join(account)
        .join(org)
}

/// Writes `local_{cli_id}.json` below `{account}/{org}` of the profile and
/// returns its body.
fn write_record(
    h: &Harness,
    id: &str,
    account: &str,
    org: &str,
    cli_id: &str,
    title: &str,
    archived: bool,
) -> String {
    let dir = folder(h, id, account, org);
    fs::create_dir_all(&dir).unwrap();
    let body = format!(
        r#"{{"sessionId":"local_{cli_id}","cliSessionId":"{cli_id}","title":"{title}","cwd":"C:\\work\\app","lastActivityAt":1000,"isArchived":{archived},"spawnSeed":"seed-1"}}"#
    );
    fs::write(dir.join(format!("local_{cli_id}.json")), &body).unwrap();
    body
}

/// A source with no sign-in of its own, and a signed-in target whose current
/// account and organisation folder exists.
fn eligible_target(h: &Harness) {
    h.fixture.add_profile(SOURCE);
    signed_in_target(h);
    fs::create_dir_all(folder(h, TARGET, ACCOUNT, ORG)).unwrap();
}

fn signed_in_target(h: &Harness) {
    h.fixture.add_profile(TARGET);
    h.fixture.sign_in(TARGET);
    h.tokens.set(
        &h.fixture.profile_path(TARGET),
        TOKEN,
        Some(NOW_MS + HOUR_MS),
    );
    h.http.respond(
        USAGE_URL,
        200,
        r#"{"five_hour":{"utilization":10.0,"resets_at":null},"seven_day":{"utilization":10.0,"resets_at":null}}"#,
    );
    h.http.respond(
        PROFILE_URL,
        200,
        &format!(
            r#"{{"account":{{"uuid":"{ACCOUNT}","email":"b@example.com"}},"organization":{{"uuid":"{ORG}"}}}}"#
        ),
    );
}

fn ids(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| s.to_string()).collect()
}

/// The `local_*.json` files below the profile's session folders, as
/// `{account}/{org}/{file}`.
fn files_in(h: &Harness, id: &str) -> Vec<String> {
    let root = h.fixture.profile_path(id).join("claude-code-sessions");
    let mut found = Vec::new();
    for account in fs::read_dir(&root).into_iter().flatten().flatten() {
        for org in fs::read_dir(account.path()).into_iter().flatten().flatten() {
            for file in fs::read_dir(org.path()).into_iter().flatten().flatten() {
                found.push(format!(
                    "{}/{}/{}",
                    account.file_name().to_string_lossy(),
                    org.file_name().to_string_lossy(),
                    file.file_name().to_string_lossy()
                ));
            }
        }
    }
    found.sort();
    found
}

#[test]
fn ticked_records_are_copied_unchanged_into_the_targets_current_folder() {
    let h = Harness::new();
    eligible_target(&h);
    let ticked = write_record(&h, SOURCE, SOURCE_ACCOUNT, ORG, "cli-1", "Fix", true);
    write_record(&h, SOURCE, SOURCE_ACCOUNT, ORG, "cli-2", "Skip", false);
    let core = h.core();
    core.refresh_usage(false);

    let outcome = core
        .copy_sessions(SOURCE, &ids(&["cli-1"]), TARGET)
        .unwrap();

    assert_eq!(
        outcome,
        CopyOutcome {
            copied: 1,
            target_running: false
        }
    );
    assert_eq!(
        files_in(&h, TARGET),
        [format!("{ACCOUNT}/{ORG}/local_cli-1.json")]
    );
    // Archived stays archived, and every other field is kept as it was.
    let copy = fs::read_to_string(folder(&h, TARGET, ACCOUNT, ORG).join("local_cli-1.json"));
    assert_eq!(copy.unwrap(), ticked);
    // The source keeps both of its records.
    assert_eq!(files_in(&h, SOURCE).len(), 2);
}

#[test]
fn a_record_the_target_already_keeps_is_replaced_wherever_it_is() {
    let h = Harness::new();
    eligible_target(&h);
    let newer = write_record(&h, SOURCE, SOURCE_ACCOUNT, ORG, "cli-1", "Newer", false);
    write_record(&h, TARGET, ACCOUNT, OTHER_ORG, "cli-1", "Older", false);
    write_record(&h, TARGET, ACCOUNT, ORG, "cli-9", "Own", false);
    let core = h.core();
    core.refresh_usage(false);

    core.copy_sessions(SOURCE, &ids(&["cli-1"]), TARGET)
        .unwrap();

    assert_eq!(
        files_in(&h, TARGET),
        [
            format!("{ACCOUNT}/{ORG}/local_cli-1.json"),
            format!("{ACCOUNT}/{ORG}/local_cli-9.json"),
        ]
    );
    let copy = fs::read_to_string(folder(&h, TARGET, ACCOUNT, ORG).join("local_cli-1.json"));
    assert_eq!(copy.unwrap(), newer);
}

#[test]
fn copying_over_the_same_file_name_overwrites_it() {
    let h = Harness::new();
    eligible_target(&h);
    let newer = write_record(&h, SOURCE, SOURCE_ACCOUNT, ORG, "cli-1", "Newer", false);
    write_record(&h, TARGET, ACCOUNT, ORG, "cli-1", "Older", false);
    let core = h.core();
    core.refresh_usage(false);

    core.copy_sessions(SOURCE, &ids(&["cli-1"]), TARGET)
        .unwrap();

    let copy = fs::read_to_string(folder(&h, TARGET, ACCOUNT, ORG).join("local_cli-1.json"));
    assert_eq!(copy.unwrap(), newer);
    assert_eq!(files_in(&h, TARGET).len(), 1);
}

#[test]
fn a_session_whose_cli_id_claude_cleared_is_copied_and_replaced_by_its_own_id() {
    let h = Harness::new();
    eligible_target(&h);
    let dir = folder(&h, SOURCE, SOURCE_ACCOUNT, ORG);
    fs::create_dir_all(&dir).unwrap();
    let cleared = r#"{"sessionId":"local_cleared","priorCliSessionIds":["cli-old"],"title":"New","lastActivityAt":2}"#;
    fs::write(dir.join("local_cleared.json"), cleared).unwrap();
    let old = folder(&h, TARGET, ACCOUNT, OTHER_ORG);
    fs::create_dir_all(&old).unwrap();
    fs::write(
        old.join("local_cleared.json"),
        r#"{"sessionId":"local_cleared","title":"Old","lastActivityAt":1}"#,
    )
    .unwrap();
    let core = h.core();
    core.refresh_usage(false);

    let outcome = core
        .copy_sessions(SOURCE, &ids(&["local_cleared"]), TARGET)
        .unwrap();

    assert_eq!(outcome.copied, 1);
    assert_eq!(
        files_in(&h, TARGET),
        [format!("{ACCOUNT}/{ORG}/local_cleared.json")]
    );
    let copy = fs::read_to_string(folder(&h, TARGET, ACCOUNT, ORG).join("local_cleared.json"));
    assert_eq!(copy.unwrap(), cleared);
}

#[test]
fn a_running_target_is_reported_so_the_user_restarts_it() {
    let h = Harness::new();
    eligible_target(&h);
    write_record(&h, SOURCE, SOURCE_ACCOUNT, ORG, "cli-1", "Fix", false);
    h.processes.run(&h.fixture.profile_path(TARGET), &[30]);
    let core = h.core();
    core.refresh_usage(false);

    let outcome = core
        .copy_sessions(SOURCE, &ids(&["cli-1"]), TARGET)
        .unwrap();

    assert!(outcome.target_running);
}

#[test]
fn an_ineligible_target_is_refused_and_nothing_is_written() {
    let h = Harness::new();
    h.fixture.add_profile(SOURCE);
    signed_in_target(&h);
    write_record(&h, SOURCE, SOURCE_ACCOUNT, ORG, "cli-1", "Fix", false);
    let core = h.core();
    core.refresh_usage(false);

    // Signed in, but its Code tab has never made its folder.
    let result = core.copy_sessions(SOURCE, &ids(&["cli-1"]), TARGET);

    assert!(matches!(
        result,
        Err(SessionsError::TargetBlocked(CopyBlocker::OpenCodeTab))
    ));
    assert!(files_in(&h, TARGET).is_empty());
}

#[test]
fn a_target_whose_uuids_are_unknown_asks_for_a_sign_in() {
    let h = Harness::new();
    h.fixture.add_profile(SOURCE);
    h.fixture.add_profile(TARGET);
    write_record(&h, SOURCE, SOURCE_ACCOUNT, ORG, "cli-1", "Fix", false);

    let result = h.core().copy_sessions(SOURCE, &ids(&["cli-1"]), TARGET);

    assert!(matches!(
        result,
        Err(SessionsError::TargetBlocked(CopyBlocker::SignIn))
    ));
}

#[test]
fn an_unknown_record_layout_in_the_target_leaves_nothing_written() {
    let h = Harness::new();
    eligible_target(&h);
    write_record(&h, SOURCE, SOURCE_ACCOUNT, ORG, "cli-1", "Fix", false);
    let odd = folder(&h, TARGET, ACCOUNT, OTHER_ORG);
    fs::create_dir_all(&odd).unwrap();
    fs::write(odd.join("local_odd.json"), r#"{"title":"no ids"}"#).unwrap();
    let core = h.core();
    core.refresh_usage(false);

    let result = core.copy_sessions(SOURCE, &ids(&["cli-1"]), TARGET);

    assert!(matches!(result, Err(SessionsError::LayoutChanged)));
    assert_eq!(
        files_in(&h, TARGET),
        [format!("{ACCOUNT}/{OTHER_ORG}/local_odd.json")]
    );
}

#[test]
fn an_account_cannot_copy_into_itself() {
    let h = Harness::new();
    eligible_target(&h);
    write_record(&h, TARGET, ACCOUNT, ORG, "cli-1", "Fix", false);
    let core = h.core();
    core.refresh_usage(false);

    let result = core.copy_sessions(TARGET, &ids(&["cli-1"]), TARGET);

    assert!(matches!(result, Err(SessionsError::SameAccount)));
}
