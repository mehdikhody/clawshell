//! Which accounts can receive session copies, through the core: given the
//! profile endpoint's answer and the folders on disk, what does each account
//! report?
//!
//! Tokens and uuids in these fixtures are synthetic.

mod support;

use clawshell_lib::core::usage::{PROFILE_URL, USAGE_URL};
use clawshell_lib::core::{Account, CopyBlocker, Core};
use std::fs;
use support::Harness;

const ID_A: &str = "3f2b8c1e-5a4d-4e7b-9c10-2d6f8a9b0c11";
const TOKEN_A: &str = "sk-test-token-aaaa";
const NOW_MS: i64 = 1_781_524_800_000;
const HOUR_MS: i64 = 3_600_000;
const ACCOUNT_UUID: &str = "11111111-2222-4333-8444-555555555555";
const ORG_UUID: &str = "66666666-7777-4888-9999-000000000000";

fn account(core: &Core, id: &str) -> Account {
    core.list_accounts()
        .unwrap()
        .accounts
        .into_iter()
        .find(|a| a.id == id)
        .expect("account listed")
}

/// A signed-in account with a valid token whose usage endpoint answers.
fn signed_in(h: &Harness) {
    h.fixture.add_profile(ID_A);
    h.fixture.sign_in(ID_A);
    h.tokens.set(
        &h.fixture.profile_path(ID_A),
        TOKEN_A,
        Some(NOW_MS + HOUR_MS),
    );
    h.http.respond(
        USAGE_URL,
        200,
        r#"{"five_hour":{"utilization":10.0,"resets_at":null},"seven_day":{"utilization":10.0,"resets_at":null}}"#,
    );
}

fn profile_answers(h: &Harness, body: &str) {
    h.http.respond(PROFILE_URL, 200, body);
}

fn profile_with_ids() -> String {
    format!(
        r#"{{"account":{{"uuid":"{ACCOUNT_UUID}","email":"a@example.com"}},"organization":{{"uuid":"{ORG_UUID}"}}}}"#
    )
}

fn make_folder(h: &Harness, account_uuid: &str, org_uuid: &str) {
    fs::create_dir_all(
        h.fixture
            .profile_path(ID_A)
            .join("claude-code-sessions")
            .join(account_uuid)
            .join(org_uuid),
    )
    .unwrap();
}

#[test]
fn a_signed_out_account_asks_to_be_signed_in_first() {
    let h = Harness::new();
    h.fixture.add_profile(ID_A);
    let core = h.core();

    assert_eq!(account(&core, ID_A).copy_blocker, Some(CopyBlocker::SignIn));
}

#[test]
fn an_account_without_its_session_folder_asks_for_the_code_tab_to_be_opened() {
    let h = Harness::new();
    signed_in(&h);
    profile_answers(&h, &profile_with_ids());
    let core = h.core();
    core.refresh_usage(false);

    assert_eq!(
        account(&core, ID_A).copy_blocker,
        Some(CopyBlocker::OpenCodeTab)
    );
}

#[test]
fn an_account_with_known_uuids_and_its_session_folder_can_receive_copies() {
    let h = Harness::new();
    signed_in(&h);
    profile_answers(&h, &profile_with_ids());
    make_folder(&h, ACCOUNT_UUID, ORG_UUID);
    let core = h.core();
    core.refresh_usage(false);

    assert_eq!(account(&core, ID_A).copy_blocker, None);
}

#[test]
fn a_folder_of_another_account_or_organisation_does_not_count() {
    let h = Harness::new();
    signed_in(&h);
    profile_answers(&h, &profile_with_ids());
    make_folder(&h, ACCOUNT_UUID, "aaaaaaaa-bbbb-4ccc-8ddd-eeeeeeeeeeee");
    make_folder(&h, "aaaaaaaa-bbbb-4ccc-8ddd-eeeeeeeeeeee", ORG_UUID);
    let core = h.core();
    core.refresh_usage(false);

    assert_eq!(
        account(&core, ID_A).copy_blocker,
        Some(CopyBlocker::OpenCodeTab)
    );
}

#[test]
fn malformed_uuids_are_never_made_part_of_a_path() {
    let h = Harness::new();
    signed_in(&h);
    profile_answers(
        &h,
        &format!(
            r#"{{"account":{{"uuid":"..","email":"a@example.com"}},"organization":{{"uuid":"{ORG_UUID}"}}}}"#
        ),
    );
    let core = h.core();
    core.refresh_usage(false);

    let a = account(&core, ID_A);
    assert_eq!(a.copy_blocker, Some(CopyBlocker::SignIn));
    assert_eq!(a.email.as_deref(), Some("a@example.com"));
}

#[test]
fn the_default_account_can_receive_copies_too() {
    let h = Harness::new();
    h.fixture.sign_in_default();
    let profile = h.fixture.default_profile_path();
    h.tokens.set(&profile, TOKEN_A, Some(NOW_MS + HOUR_MS));
    h.http.respond(
        USAGE_URL,
        200,
        r#"{"five_hour":{"utilization":10.0,"resets_at":null},"seven_day":{"utilization":10.0,"resets_at":null}}"#,
    );
    profile_answers(&h, &profile_with_ids());
    fs::create_dir_all(
        profile
            .join("claude-code-sessions")
            .join(ACCOUNT_UUID)
            .join(ORG_UUID),
    )
    .unwrap();
    let core = h.core();
    core.refresh_usage(false);

    assert_eq!(account(&core, "default").copy_blocker, None);
}

#[test]
fn a_signed_in_account_whose_uuids_are_unknown_asks_to_be_signed_in_first() {
    let h = Harness::new();
    signed_in(&h);
    let core = h.core();

    // Before the first poll nothing has said who the account is.
    assert_eq!(account(&core, ID_A).copy_blocker, Some(CopyBlocker::SignIn));

    // The profile answer carries no uuids.
    profile_answers(
        &h,
        r#"{"account":{"email":"a@example.com"},"organization":{}}"#,
    );
    core.refresh_usage(false);

    assert_eq!(account(&core, ID_A).copy_blocker, Some(CopyBlocker::SignIn));
}
