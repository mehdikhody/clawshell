//! Live usage per account, through the core: given these files, tokens and
//! HTTP responses at this time, what does each account show?
//!
//! Tokens and emails in these fixtures are synthetic.

mod support;

use clawshell_lib::core::usage::{PROFILE_URL, USAGE_URL, UsageSource};
use clawshell_lib::core::{Account, Core, Status};
use std::time::Duration;
use support::Harness;

const ID_A: &str = "3f2b8c1e-5a4d-4e7b-9c10-2d6f8a9b0c11";
const ID_B: &str = "a0b1c2d3-e4f5-4a6b-8c7d-9e0f1a2b3c4d";
const TOKEN_A: &str = "sk-test-token-aaaa";
const TOKEN_B: &str = "sk-test-token-bbbb";
const NOW_MS: i64 = 1_781_524_800_000;
const HOUR_MS: i64 = 3_600_000;

fn usage_body(five_hour: f64, five_reset: &str, seven_day: f64, seven_reset: &str) -> String {
    format!(
        r#"{{"five_hour":{{"utilization":{five_hour},"resets_at":"{five_reset}"}},
            "seven_day":{{"utilization":{seven_day},"resets_at":"{seven_reset}"}},
            "seven_day_opus":null}}"#
    )
}

fn profile_body(email: &str) -> String {
    format!(r#"{{"account":{{"email":"{email}","display_name":"Someone"}},"organization":{{}}}}"#)
}

const FIVE_RESET: &str = "2026-06-15T15:00:00.000000+00:00";
const WEEK_RESET: &str = "2026-06-19T09:30:00.000000+00:00";

/// A signed-in account with a valid token, and a ready email and usage response.
fn signed_in(h: &Harness, id: &str, token: &str) {
    h.fixture.add_profile(id);
    h.fixture.sign_in(id);
    h.tokens
        .set(&h.fixture.profile_path(id), token, Some(NOW_MS + HOUR_MS));
}

fn script_ok(h: &Harness, email: &str, five: f64, seven: f64) {
    h.http.respond(
        USAGE_URL,
        200,
        &usage_body(five, FIVE_RESET, seven, WEEK_RESET),
    );
    h.http.respond(PROFILE_URL, 200, &profile_body(email));
}

fn account(core: &Core, id: &str) -> Account {
    core.list_accounts()
        .unwrap()
        .accounts
        .into_iter()
        .find(|a| a.id == id)
        .expect("account listed")
}

#[test]
fn a_row_shows_the_email_and_both_windows_with_their_reset_times() {
    let h = Harness::new();
    signed_in(&h, ID_A, TOKEN_A);
    script_ok(&h, "work@example.com", 42.0, 17.5);
    let core = h.core();

    core.refresh_usage(false);

    let a = account(&core, ID_A);
    assert_eq!(a.email.as_deref(), Some("work@example.com"));
    let usage = a.usage.expect("a usage snapshot");
    assert_eq!(usage.source, UsageSource::Live);
    assert_eq!(usage.fetched_at_ms, NOW_MS);
    let five = usage.five_hour.expect("5h window");
    assert_eq!(five.utilization, 42.0);
    assert_eq!(five.resets_at.as_deref(), Some(FIVE_RESET));
    let week = usage.weekly.expect("weekly window");
    assert_eq!(week.utilization, 17.5);
    assert_eq!(week.resets_at.as_deref(), Some(WEEK_RESET));
}

#[test]
fn nothing_is_shown_before_the_first_refresh() {
    let h = Harness::new();
    signed_in(&h, ID_A, TOKEN_A);

    let a = account(&h.core(), ID_A);

    assert_eq!(a.email, None);
    assert!(a.usage.is_none());
    assert!(
        h.http.requests().is_empty(),
        "listing never touches the network"
    );
}

#[test]
fn each_account_is_fetched_with_its_own_token_and_only_from_the_two_endpoints() {
    let h = Harness::new();
    signed_in(&h, ID_A, TOKEN_A);
    signed_in(&h, ID_B, TOKEN_B);
    script_ok(&h, "a@example.com", 10.0, 10.0);
    script_ok(&h, "b@example.com", 20.0, 20.0);
    let core = h.core();

    core.refresh_usage(false);

    let requests = h.http.requests();
    assert_eq!(
        requests.len(),
        4,
        "one usage and one profile call per account"
    );
    for request in &requests {
        assert!(
            request.url == USAGE_URL || request.url == PROFILE_URL,
            "unexpected endpoint"
        );
        let header = |name: &str| {
            request
                .headers
                .iter()
                .find(|(n, _)| n.eq_ignore_ascii_case(name))
                .map(|(_, v)| v.as_str())
        };
        let bearer = header("authorization").expect("bearer header");
        assert!(bearer == format!("Bearer {TOKEN_A}") || bearer == format!("Bearer {TOKEN_B}"));
        assert_eq!(
            header("anthropic-beta").is_some(),
            request.url == USAGE_URL,
            "only the usage endpoint takes the beta header"
        );
        if request.url == USAGE_URL {
            assert_eq!(header("anthropic-beta"), Some("oauth-2025-04-20"));
        }
    }
    // Account order is by id: A's token went with A's calls.
    assert_eq!(account(&core, ID_B).email.as_deref(), Some("b@example.com"));
    assert_eq!(account(&core, ID_A).email.as_deref(), Some("a@example.com"));
}

#[test]
fn a_request_never_shows_its_token_when_printed() {
    let h = Harness::new();
    signed_in(&h, ID_A, TOKEN_A);
    script_ok(&h, "a@example.com", 1.0, 1.0);
    h.core().refresh_usage(false);

    let shown = format!("{:?}", h.http.requests());

    assert!(!shown.contains(TOKEN_A));
    assert!(!shown.contains("Bearer"));
}

#[test]
fn the_email_is_fetched_once_and_remembered() {
    let h = Harness::new();
    signed_in(&h, ID_A, TOKEN_A);
    script_ok(&h, "a@example.com", 10.0, 10.0);
    h.http.respond(
        USAGE_URL,
        200,
        &usage_body(11.0, FIVE_RESET, 10.0, WEEK_RESET),
    );
    let core = h.core();
    core.refresh_usage(false);
    h.clock.advance(Duration::from_secs(1200));

    core.refresh_usage(false);

    let profile_calls = h
        .http
        .requests()
        .iter()
        .filter(|r| r.url == PROFILE_URL)
        .count();
    assert_eq!(profile_calls, 1);
    assert_eq!(account(&core, ID_A).email.as_deref(), Some("a@example.com"));
}

#[test]
fn a_failed_profile_call_leaves_the_email_empty_but_keeps_the_usage() {
    let h = Harness::new();
    signed_in(&h, ID_A, TOKEN_A);
    h.http.respond(
        USAGE_URL,
        200,
        &usage_body(30.0, FIVE_RESET, 5.0, WEEK_RESET),
    );
    h.http.respond(PROFILE_URL, 500, "{}");
    let core = h.core();

    core.refresh_usage(false);

    let a = account(&core, ID_A);
    assert_eq!(a.email, None);
    assert_eq!(a.usage.unwrap().source, UsageSource::Live);
}

#[test]
fn extra_usage_fields_are_read_when_present_and_optional_when_not() {
    let h = Harness::new();
    signed_in(&h, ID_A, TOKEN_A);
    signed_in(&h, ID_B, TOKEN_B);
    h.http.respond(
        USAGE_URL,
        200,
        r#"{"five_hour":{"utilization":1,"resets_at":null},
            "seven_day":{"utilization":2,"resets_at":null},
            "extra_usage":{"is_enabled":true,"used_credits":1250,"monthly_limit":5000}}"#,
    );
    h.http
        .respond(PROFILE_URL, 200, &profile_body("a@example.com"));
    h.http.respond(
        USAGE_URL,
        200,
        r#"{"five_hour":{"utilization":1,"resets_at":null},
            "seven_day":{"utilization":2,"resets_at":null},
            "extra_usage":{"is_enabled":false,"credits_ever_enabled":false}}"#,
    );
    h.http
        .respond(PROFILE_URL, 200, &profile_body("b@example.com"));
    let core = h.core();

    core.refresh_usage(false);

    let extra = account(&core, ID_A).usage.unwrap().extra_usage.unwrap();
    assert!(extra.enabled);
    assert_eq!(extra.used_minor, Some(1250));
    assert_eq!(extra.limit_minor, Some(5000));
    let off = account(&core, ID_B).usage.unwrap();
    assert_eq!(off.five_hour.unwrap().resets_at, None);
    let off = off.extra_usage.unwrap();
    assert!(!off.enabled);
    assert_eq!(off.used_minor, None);
}

#[test]
fn extra_usage_spend_falls_back_to_the_spend_money_object() {
    let h = Harness::new();
    signed_in(&h, ID_A, TOKEN_A);
    h.http.respond(
        USAGE_URL,
        200,
        r#"{"five_hour":{"utilization":1,"resets_at":null},
            "seven_day":{"utilization":2,"resets_at":null},
            "extra_usage":{"is_enabled":true},
            "spend":{"used":{"amount_minor":830,"currency":"USD","exponent":2}}}"#,
    );
    h.http
        .respond(PROFILE_URL, 200, &profile_body("a@example.com"));
    let core = h.core();

    core.refresh_usage(false);

    let extra = account(&core, ID_A).usage.unwrap().extra_usage.unwrap();
    assert_eq!(extra.used_minor, Some(830));
}

#[test]
fn signed_out_accounts_are_never_fetched() {
    let h = Harness::new();
    h.fixture.add_profile(ID_A); // no sign-in
    let core = h.core();

    let outcome = core.refresh_usage(false);

    assert_eq!(outcome.refreshed, 0);
    assert!(h.http.requests().is_empty());
}

#[test]
fn an_account_is_limited_when_either_window_reaches_100_percent() {
    let h = Harness::new();
    signed_in(&h, ID_A, TOKEN_A);
    signed_in(&h, ID_B, TOKEN_B);
    script_ok(&h, "a@example.com", 100.0, 20.0); // 5h exhausted
    script_ok(&h, "b@example.com", 30.0, 100.0); // weekly exhausted
    let core = h.core();

    core.refresh_usage(false);

    assert_eq!(account(&core, ID_A).status, Status::Limited);
    assert_eq!(account(&core, ID_B).status, Status::Limited);
}

#[test]
fn just_under_100_percent_is_not_limited() {
    let h = Harness::new();
    signed_in(&h, ID_A, TOKEN_A);
    script_ok(&h, "a@example.com", 99.0, 99.9);
    let core = h.core();

    core.refresh_usage(false);

    assert_eq!(account(&core, ID_A).status, Status::Idle);
}

#[test]
fn a_running_limited_account_is_limited_but_still_marked_running() {
    let h = Harness::new();
    signed_in(&h, ID_A, TOKEN_A);
    script_ok(&h, "a@example.com", 100.0, 20.0);
    h.processes
        .0
        .lock()
        .unwrap()
        .insert(h.fixture.profile_path(ID_A));
    let core = h.core();

    core.refresh_usage(false);

    let a = account(&core, ID_A);
    assert_eq!(a.status, Status::Limited);
    assert!(a.running);
}

#[test]
fn signed_out_outranks_limited() {
    let h = Harness::new();
    signed_in(&h, ID_A, TOKEN_A);
    script_ok(&h, "a@example.com", 100.0, 100.0);
    let core = h.core();
    core.refresh_usage(false);
    // The sign-in goes away (Claude signed out); the old snapshot lingers.
    h.fixture
        .write_config(ID_A, r#"{"windowSizeWasSignedIn": false}"#);

    assert_eq!(account(&core, ID_A).status, Status::SignedOut);
}

// ---------------------------------------------------------------------------
// Failure: stale snapshot, then local history. A token is never refreshed.
// ---------------------------------------------------------------------------

const HISTORY: &str = r#"{"version":2,"samples":[
    {"t":1781500000000,"org":"o","u":{"fh":10,"sd":20}},
    {"t":1781524000000,"org":"o","u":{"fh":55,"sd":60}},
    {"t":1781510000000,"org":"o","u":{"fh":99,"sd":99}}]}"#;

fn only_the_two_endpoints_were_called(h: &Harness) {
    for request in h.http.requests() {
        assert!(
            request.url == USAGE_URL || request.url == PROFILE_URL,
            "a request went somewhere other than the usage and profile endpoints"
        );
    }
}

#[test]
fn a_401_keeps_the_last_snapshot_marked_stale() {
    let h = Harness::new();
    signed_in(&h, ID_A, TOKEN_A);
    script_ok(&h, "a@example.com", 42.0, 17.5);
    let core = h.core();
    core.refresh_usage(false);
    h.clock.advance(Duration::from_secs(700));
    h.http
        .respond(USAGE_URL, 401, r#"{"error":"unauthorized"}"#);

    core.refresh_usage(false);

    let a = account(&core, ID_A);
    let usage = a.usage.expect("the last snapshot is kept");
    assert_eq!(usage.source, UsageSource::Stale);
    assert_eq!(
        usage.fetched_at_ms, NOW_MS,
        "updated-at is still the live fetch"
    );
    let five = usage.five_hour.unwrap();
    assert_eq!(five.utilization, 42.0);
    assert_eq!(five.resets_at.as_deref(), Some(FIVE_RESET));
    assert_eq!(a.email.as_deref(), Some("a@example.com"));
    only_the_two_endpoints_were_called(&h);
    assert_eq!(
        h.http.requests().len(),
        3,
        "no retry, no refresh call after the 401"
    );
}

#[test]
fn a_401_with_no_snapshot_falls_back_to_the_newest_local_history_sample() {
    let h = Harness::new();
    signed_in(&h, ID_A, TOKEN_A);
    h.fixture.write_usage_history(ID_A, HISTORY);
    h.http.respond(USAGE_URL, 401, "{}");
    let core = h.core();

    core.refresh_usage(false);

    let usage = account(&core, ID_A).usage.expect("history values");
    assert_eq!(usage.source, UsageSource::LocalHistory);
    assert_eq!(usage.fetched_at_ms, 1_781_524_000_000);
    let five = usage.five_hour.unwrap();
    assert_eq!(five.utilization, 55.0);
    assert_eq!(five.resets_at, None, "history carries no reset times");
    assert_eq!(usage.weekly.unwrap().utilization, 60.0);
}

#[test]
fn a_newer_history_sample_replaces_an_older_stale_snapshot() {
    let h = Harness::new();
    signed_in(&h, ID_A, TOKEN_A);
    script_ok(&h, "a@example.com", 42.0, 17.5);
    let core = h.core();
    core.refresh_usage(false);
    // Claude's own usage tray recorded a reading after our last live fetch.
    h.fixture.write_usage_history(
        ID_A,
        r#"{"version":2,"samples":[{"t":1781525000000,"org":"o","u":{"fh":70,"sd":30}}]}"#,
    );
    h.clock.advance(Duration::from_secs(700));
    h.http.respond(USAGE_URL, 401, "{}");

    core.refresh_usage(false);

    let usage = account(&core, ID_A).usage.unwrap();
    assert_eq!(usage.source, UsageSource::LocalHistory);
    assert_eq!(usage.five_hour.unwrap().utilization, 70.0);
}

#[test]
fn an_older_history_sample_does_not_replace_a_newer_stale_snapshot() {
    let h = Harness::new();
    signed_in(&h, ID_A, TOKEN_A);
    h.fixture.write_usage_history(ID_A, HISTORY); // newest sample is before NOW
    script_ok(&h, "a@example.com", 42.0, 17.5);
    let core = h.core();
    core.refresh_usage(false);
    h.clock.advance(Duration::from_secs(700));
    h.http.respond(USAGE_URL, 401, "{}");

    core.refresh_usage(false);

    let usage = account(&core, ID_A).usage.unwrap();
    assert_eq!(usage.source, UsageSource::Stale);
    assert_eq!(usage.five_hour.unwrap().utilization, 42.0);
}

#[test]
fn an_expired_token_makes_no_request_and_shows_stale_values() {
    let h = Harness::new();
    signed_in(&h, ID_A, TOKEN_A);
    script_ok(&h, "a@example.com", 42.0, 17.5);
    let core = h.core();
    core.refresh_usage(false);
    let calls_before = h.http.requests().len();
    h.clock.advance(Duration::from_secs(2 * 3600)); // past the token's expiry

    core.refresh_usage(false);

    assert_eq!(
        h.http.requests().len(),
        calls_before,
        "an expired token is never sent"
    );
    assert_eq!(
        account(&core, ID_A).usage.unwrap().source,
        UsageSource::Stale
    );
}

#[test]
fn an_expired_token_with_no_snapshot_shows_local_history() {
    let h = Harness::new();
    signed_in(&h, ID_A, TOKEN_A);
    h.tokens
        .set(&h.fixture.profile_path(ID_A), TOKEN_A, Some(NOW_MS - 1));
    h.fixture.write_usage_history(ID_A, HISTORY);
    let core = h.core();

    core.refresh_usage(false);

    assert!(h.http.requests().is_empty());
    let usage = account(&core, ID_A).usage.unwrap();
    assert_eq!(usage.source, UsageSource::LocalHistory);
    assert_eq!(usage.weekly.unwrap().utilization, 60.0);
}

#[test]
fn a_signed_in_profile_whose_token_cannot_be_read_shows_history_without_requests() {
    let h = Harness::new();
    h.fixture.add_profile(ID_A);
    h.fixture.sign_in(ID_A); // no token registered: decryption failed
    h.fixture.write_usage_history(ID_A, HISTORY);
    let core = h.core();

    core.refresh_usage(false);

    assert!(h.http.requests().is_empty());
    assert_eq!(
        account(&core, ID_A).usage.unwrap().source,
        UsageSource::LocalHistory
    );
}

#[test]
fn live_numbers_return_once_claude_renews_the_token() {
    let h = Harness::new();
    signed_in(&h, ID_A, TOKEN_A);
    h.http.respond(USAGE_URL, 401, "{}");
    h.fixture.write_usage_history(ID_A, HISTORY);
    let core = h.core();
    core.refresh_usage(false);
    assert_eq!(
        account(&core, ID_A).usage.unwrap().source,
        UsageSource::LocalHistory
    );
    // Desktop refreshes its own token; we only notice, we never do it.
    h.tokens.set(
        &h.fixture.profile_path(ID_A),
        "sk-test-renewed",
        Some(NOW_MS + 5 * HOUR_MS),
    );
    h.clock.advance(Duration::from_secs(2000));
    script_ok(&h, "a@example.com", 5.0, 6.0);

    core.refresh_usage(false);

    let usage = account(&core, ID_A).usage.unwrap();
    assert_eq!(usage.source, UsageSource::Live);
    assert_eq!(usage.five_hour.unwrap().utilization, 5.0);
}

#[test]
fn a_network_error_keeps_the_last_snapshot_as_stale() {
    let h = Harness::new();
    signed_in(&h, ID_A, TOKEN_A);
    script_ok(&h, "a@example.com", 42.0, 17.5);
    let core = h.core();
    core.refresh_usage(false);
    h.clock.advance(Duration::from_secs(700));
    h.http.fail(USAGE_URL);

    core.refresh_usage(false);

    assert_eq!(
        account(&core, ID_A).usage.unwrap().source,
        UsageSource::Stale
    );
}

#[test]
fn a_body_that_is_not_usage_counts_as_a_failure() {
    let h = Harness::new();
    signed_in(&h, ID_A, TOKEN_A);
    h.http.respond(USAGE_URL, 200, r#"{"unexpected":true}"#);
    let core = h.core();

    core.refresh_usage(false);

    assert!(account(&core, ID_A).usage.is_none());
}

#[test]
fn a_stale_snapshot_still_at_100_percent_keeps_the_account_limited() {
    let h = Harness::new();
    signed_in(&h, ID_A, TOKEN_A);
    script_ok(&h, "a@example.com", 100.0, 40.0);
    let core = h.core();
    core.refresh_usage(false);
    h.clock.advance(Duration::from_secs(700));
    h.http.respond(USAGE_URL, 401, "{}");

    core.refresh_usage(false);

    assert_eq!(account(&core, ID_A).status, Status::Limited);
}

#[test]
fn local_history_at_100_percent_makes_the_account_limited() {
    let h = Harness::new();
    signed_in(&h, ID_A, TOKEN_A);
    h.fixture.write_usage_history(
        ID_A,
        r#"{"version":2,"samples":[{"t":1781524000000,"org":"o","u":{"fh":10,"sd":100}}]}"#,
    );
    h.http.respond(USAGE_URL, 401, "{}");
    let core = h.core();

    core.refresh_usage(false);

    assert_eq!(account(&core, ID_A).status, Status::Limited);
}

#[test]
fn unreadable_history_is_ignored() {
    let h = Harness::new();
    signed_in(&h, ID_A, TOKEN_A);
    h.fixture.write_usage_history(ID_A, "not json at all");
    h.http.respond(USAGE_URL, 401, "{}");
    let core = h.core();

    core.refresh_usage(false);

    assert!(account(&core, ID_A).usage.is_none());
}

// ---------------------------------------------------------------------------
// Polling schedule, manual refresh and rate limits
// ---------------------------------------------------------------------------

fn more_usage(h: &Harness, five: f64, seven: f64) {
    h.http.respond(
        USAGE_URL,
        200,
        &usage_body(five, FIVE_RESET, seven, WEEK_RESET),
    );
}

fn usage_calls(h: &Harness) -> usize {
    h.http
        .requests()
        .iter()
        .filter(|r| r.url == USAGE_URL)
        .count()
}

fn wait(h: &Harness, secs: u64) {
    h.clock.advance(Duration::from_secs(secs));
}

#[test]
fn an_idle_account_is_polled_again_only_after_about_600_seconds() {
    let h = Harness::new();
    signed_in(&h, ID_A, TOKEN_A);
    script_ok(&h, "a@example.com", 10.0, 10.0);
    let core = h.core();
    core.refresh_usage(false);

    wait(&h, 500);
    assert_eq!(core.refresh_usage(false).refreshed, 0, "not due at 500 s");
    more_usage(&h, 11.0, 10.0);
    wait(&h, 200);
    assert_eq!(core.refresh_usage(false).refreshed, 1, "due by 700 s");
    assert_eq!(usage_calls(&h), 2);
}

#[test]
fn an_account_with_claude_running_is_polled_about_every_300_seconds() {
    let h = Harness::new();
    signed_in(&h, ID_A, TOKEN_A);
    h.processes
        .0
        .lock()
        .unwrap()
        .insert(h.fixture.profile_path(ID_A));
    script_ok(&h, "a@example.com", 10.0, 10.0);
    let core = h.core();
    core.refresh_usage(false);

    wait(&h, 250);
    assert_eq!(core.refresh_usage(false).refreshed, 0);
    more_usage(&h, 11.0, 10.0);
    wait(&h, 100);
    assert_eq!(core.refresh_usage(false).refreshed, 1);
}

#[test]
fn an_account_near_a_limit_is_polled_at_the_minimum_interval_not_faster() {
    let h = Harness::new();
    signed_in(&h, ID_A, TOKEN_A);
    script_ok(&h, "a@example.com", 85.0, 10.0);
    let core = h.core();
    core.refresh_usage(false);

    wait(&h, 170);
    assert_eq!(
        core.refresh_usage(false).refreshed,
        0,
        "60 s is raised to 180 s"
    );
    more_usage(&h, 86.0, 10.0);
    wait(&h, 40); // 210 s: past 180 s plus the largest jitter
    assert_eq!(core.refresh_usage(false).refreshed, 1);
}

#[test]
fn an_exhausted_account_is_polled_about_every_600_seconds() {
    let h = Harness::new();
    signed_in(&h, ID_A, TOKEN_A);
    script_ok(&h, "a@example.com", 100.0, 10.0);
    let core = h.core();
    core.refresh_usage(false);

    wait(&h, 500);
    assert_eq!(core.refresh_usage(false).refreshed, 0);
}

#[test]
fn the_next_refresh_time_is_reported_after_each_refresh() {
    let h = Harness::new();
    signed_in(&h, ID_A, TOKEN_A);
    script_ok(&h, "a@example.com", 10.0, 10.0);
    let core = h.core();
    assert_eq!(core.list_accounts().unwrap().refresh_allowed_at, None);

    let outcome = core.refresh_usage(false);

    let allowed = NOW_MS + 180_000;
    assert_eq!(outcome.refresh_allowed_at, Some(allowed));
    assert_eq!(
        core.list_accounts().unwrap().refresh_allowed_at,
        Some(allowed)
    );
    wait(&h, 181);
    assert_eq!(core.list_accounts().unwrap().refresh_allowed_at, None);
}

#[test]
fn a_manual_refresh_inside_the_minimum_interval_does_nothing_and_says_when_it_may_run() {
    let h = Harness::new();
    signed_in(&h, ID_A, TOKEN_A);
    script_ok(&h, "a@example.com", 10.0, 10.0);
    let core = h.core();
    core.refresh_usage(false);
    wait(&h, 60);

    let outcome = core.refresh_usage(true);

    assert_eq!(outcome.refreshed, 0);
    assert_eq!(outcome.refresh_allowed_at, Some(NOW_MS + 180_000));
    assert_eq!(usage_calls(&h), 1);
}

#[test]
fn a_manual_refresh_after_the_minimum_interval_fetches_even_when_not_due() {
    let h = Harness::new();
    signed_in(&h, ID_A, TOKEN_A);
    script_ok(&h, "a@example.com", 10.0, 10.0);
    let core = h.core();
    core.refresh_usage(false);
    wait(&h, 200); // the scheduled poll is not due until ~600 s
    more_usage(&h, 12.0, 10.0);

    let outcome = core.refresh_usage(true);

    assert_eq!(outcome.refreshed, 1);
    assert_eq!(
        account(&core, ID_A)
            .usage
            .unwrap()
            .five_hour
            .unwrap()
            .utilization,
        12.0
    );
    assert_eq!(outcome.refresh_allowed_at, Some(NOW_MS + 200_000 + 180_000));
}

#[test]
fn a_manual_refresh_only_asks_accounts_that_are_past_their_own_minimum() {
    let h = Harness::new();
    signed_in(&h, ID_A, TOKEN_A);
    script_ok(&h, "a@example.com", 10.0, 10.0);
    let core = h.core();
    core.refresh_usage(false);
    wait(&h, 100);
    signed_in(&h, ID_B, TOKEN_B); // a new account, never polled
    script_ok(&h, "b@example.com", 20.0, 20.0);

    let outcome = core.refresh_usage(true);

    assert_eq!(outcome.refreshed, 1, "only the new account");
    assert_eq!(account(&core, ID_B).email.as_deref(), Some("b@example.com"));
    assert_eq!(usage_calls(&h), 2);
}

#[test]
fn a_429_with_retry_after_backs_off_for_that_long_and_blocks_manual_refresh() {
    let h = Harness::new();
    signed_in(&h, ID_A, TOKEN_A);
    script_ok(&h, "a@example.com", 10.0, 10.0);
    let core = h.core();
    core.refresh_usage(false);
    wait(&h, 700);
    h.http
        .respond_with_headers(USAGE_URL, 429, &[("Retry-After", "900")], "{}");

    let outcome = core.refresh_usage(false);

    let now = NOW_MS + 700_000;
    assert_eq!(
        account(&core, ID_A).usage.unwrap().source,
        UsageSource::Stale
    );
    assert_eq!(outcome.refresh_allowed_at, Some(now + 900_000));
    wait(&h, 600);
    assert_eq!(core.refresh_usage(false).refreshed, 0, "still backing off");
    assert_eq!(core.refresh_usage(true).refreshed, 0, "manual obeys it too");
    more_usage(&h, 11.0, 10.0);
    wait(&h, 301);
    assert_eq!(core.refresh_usage(false).refreshed, 1);
}

#[test]
fn retry_after_zero_waits_300_seconds_then_360_while_the_saturated_floor_lasts() {
    let h = Harness::new();
    signed_in(&h, ID_A, TOKEN_A);
    h.http
        .respond_with_headers(USAGE_URL, 429, &[("retry-after", "0")], "{}");
    let core = h.core();

    let first = core.refresh_usage(false);
    assert_eq!(first.refresh_allowed_at, Some(NOW_MS + 300_000));

    wait(&h, 301);
    h.http
        .respond_with_headers(USAGE_URL, 429, &[("Retry-After", "0")], "{}");
    let second = core.refresh_usage(false);
    assert_eq!(second.refreshed, 1);
    assert_eq!(second.refresh_allowed_at, Some(NOW_MS + 301_000 + 360_000));
}

#[test]
fn the_saturated_floor_also_slows_polling_after_a_success() {
    let h = Harness::new();
    signed_in(&h, ID_A, TOKEN_A);
    h.http
        .respond_with_headers(USAGE_URL, 429, &[("Retry-After", "0")], "{}");
    let core = h.core();
    core.refresh_usage(false);
    wait(&h, 301);
    script_ok(&h, "a@example.com", 85.0, 10.0); // near a limit: normally ~180 s
    core.refresh_usage(false);

    wait(&h, 300);
    assert_eq!(
        core.refresh_usage(false).refreshed,
        0,
        "floor keeps it at 360 s"
    );
    more_usage(&h, 86.0, 10.0);
    wait(&h, 100);
    assert_eq!(core.refresh_usage(false).refreshed, 1);
}

#[test]
fn a_429_without_retry_after_backs_off_exponentially() {
    let h = Harness::new();
    signed_in(&h, ID_A, TOKEN_A);
    let core = h.core();
    let mut waits = Vec::new();
    for _ in 0..4 {
        h.http.respond(USAGE_URL, 429, "{}");
        let at = core
            .refresh_usage(false)
            .refresh_allowed_at
            .expect("backing off");
        let started = h.now_ms();
        waits.push((at - started) / 1000);
        h.clock
            .advance(Duration::from_millis((at - started) as u64 + 1000));
    }
    assert_eq!(waits, vec![180, 360, 720, 1440]);
}

#[test]
fn server_errors_back_off_exponentially_up_to_1800_seconds_and_success_resets_it() {
    let h = Harness::new();
    signed_in(&h, ID_A, TOKEN_A);
    // Long-lived token: this test runs for hours of fake time.
    h.tokens.set(
        &h.fixture.profile_path(ID_A),
        TOKEN_A,
        Some(NOW_MS + 100 * HOUR_MS),
    );
    let core = h.core();
    for _ in 0..6 {
        h.http.respond(USAGE_URL, 503, "{}");
        core.refresh_usage(true);
        wait(&h, 1801);
    }
    // After many failures the next poll is 1800 s out, not further.
    h.http.respond(USAGE_URL, 503, "{}");
    core.refresh_usage(false);
    wait(&h, 1700);
    assert_eq!(core.refresh_usage(false).refreshed, 0);
    more_usage(&h, 10.0, 10.0);
    wait(&h, 200);
    assert_eq!(core.refresh_usage(false).refreshed, 1);

    // The success reset the count: one failure now backs off only 180 s.
    wait(&h, 1000);
    h.http.respond(USAGE_URL, 503, "{}");
    assert_eq!(core.refresh_usage(false).refreshed, 1);
    more_usage(&h, 10.0, 10.0);
    wait(&h, 200);
    assert_eq!(core.refresh_usage(false).refreshed, 1);
}

#[test]
fn deleted_accounts_are_forgotten() {
    let h = Harness::new();
    signed_in(&h, ID_A, TOKEN_A);
    script_ok(&h, "a@example.com", 10.0, 10.0);
    let core = h.core();
    core.refresh_usage(false);

    core.delete_account(ID_A).unwrap();

    assert_eq!(core.refresh_usage(true).refresh_allowed_at, None);
    assert!(core.list_accounts().unwrap().accounts.is_empty());
}

// ---- Pooled remaining ----

const ID_C: &str = "c0c0c0c0-0000-4000-8000-000000000003";

/// Signs in each account (in id order: A, B, C) and scripts its usage, then
/// refreshes once. `usage` is (five-hour %, weekly %) per account.
fn pooled_core(h: &Harness, usage: &[(&str, f64, f64)]) -> Core {
    for (i, (id, five, seven)) in usage.iter().enumerate() {
        signed_in(h, id, &format!("sk-test-token-{i}"));
        script_ok(h, &format!("user{i}@example.com"), *five, *seven);
    }
    let core = h.core();
    core.refresh_usage(false);
    core
}

#[test]
fn pooled_remaining_is_the_average_of_100_minus_utilisation() {
    let h = Harness::new();
    let core = pooled_core(
        &h,
        &[(ID_A, 60.0, 30.0), (ID_B, 0.0, 0.0), (ID_C, 0.0, 60.0)],
    );

    let pooled = core.list_accounts().unwrap().pooled;

    assert_eq!(pooled.five_hour.remaining_percent, Some(80.0));
    assert_eq!(pooled.weekly.remaining_percent, Some(70.0));
}

#[test]
fn available_counts_the_accounts_below_100_percent_of_those_pooled() {
    let h = Harness::new();
    let core = pooled_core(
        &h,
        &[
            (ID_A, 100.0, 40.0),
            (ID_B, 99.0, 100.0),
            (ID_C, 20.0, 100.0),
        ],
    );

    let pooled = core.list_accounts().unwrap().pooled;

    assert_eq!((pooled.five_hour.available, pooled.five_hour.total), (2, 3));
    assert_eq!((pooled.weekly.available, pooled.weekly.total), (1, 3));
}

#[test]
fn a_signed_out_account_is_left_out_of_the_average_and_the_count() {
    let h = Harness::new();
    signed_in(&h, ID_A, TOKEN_A);
    script_ok(&h, "a@example.com", 60.0, 60.0);
    h.fixture.add_profile(ID_B); // never signed in
    let core = h.core();
    core.refresh_usage(false);

    let pooled = core.list_accounts().unwrap().pooled;

    assert_eq!(pooled.five_hour.remaining_percent, Some(40.0));
    assert_eq!((pooled.five_hour.available, pooled.five_hour.total), (1, 1));
    assert_eq!((pooled.weekly.available, pooled.weekly.total), (1, 1));
}

#[test]
fn with_no_signed_in_accounts_nothing_is_pooled() {
    let h = Harness::new();
    h.fixture.add_profile(ID_A); // signed out
    let core = h.core();
    core.refresh_usage(false);

    let pooled = core.list_accounts().unwrap().pooled;

    assert_eq!(pooled.five_hour.remaining_percent, None);
    assert_eq!((pooled.five_hour.available, pooled.five_hour.total), (0, 0));
    assert_eq!(pooled.five_hour.soonest_reset, None);
    assert_eq!(pooled.weekly.remaining_percent, None);
}

#[test]
fn a_signed_in_account_with_no_reading_yet_is_not_pooled() {
    let h = Harness::new();
    signed_in(&h, ID_A, TOKEN_A); // listed, never refreshed

    let pooled = h.core().list_accounts().unwrap().pooled;

    assert_eq!(pooled.five_hour.remaining_percent, None);
    assert_eq!(pooled.five_hour.total, 0);
}

#[test]
fn the_soonest_reset_is_the_earliest_across_accounts_whatever_their_offset() {
    let h = Harness::new();
    for (id, token, five_reset, week_reset) in [
        (
            ID_A,
            "ta",
            "2026-06-15T18:00:00.000000+00:00",
            "2026-06-19T09:30:00+00:00",
        ),
        // 16:30 UTC written with a +03:30 offset, so textually "later" than A's.
        (
            ID_B,
            "tb",
            "2026-06-15T20:00:00+03:30",
            "2026-06-18T09:30:00Z",
        ),
        (
            ID_C,
            "tc",
            "2026-06-15T17:00:00.5+00:00",
            "2026-06-20T09:30:00+00:00",
        ),
    ] {
        signed_in(&h, id, token);
        h.http.respond(
            USAGE_URL,
            200,
            &usage_body(10.0, five_reset, 10.0, week_reset),
        );
        h.http
            .respond(PROFILE_URL, 200, &profile_body("x@example.com"));
    }
    let core = h.core();
    core.refresh_usage(false);

    let pooled = core.list_accounts().unwrap().pooled;

    assert_eq!(
        pooled.five_hour.soonest_reset.as_deref(),
        Some("2026-06-15T20:00:00+03:30")
    );
    assert_eq!(
        pooled.weekly.soonest_reset.as_deref(),
        Some("2026-06-18T09:30:00Z")
    );
}

#[test]
fn local_history_has_no_reset_so_it_never_supplies_the_soonest() {
    let h = Harness::new();
    signed_in(&h, ID_A, TOKEN_A);
    h.fixture.write_usage_history(
        ID_A,
        &format!(r#"{{"samples":[{{"t":{NOW_MS},"u":{{"fh":50,"sd":25}}}}]}}"#),
    );
    h.http.respond(USAGE_URL, 500, "{}");
    let core = h.core();
    core.refresh_usage(false);

    let pooled = core.list_accounts().unwrap().pooled;

    assert_eq!(pooled.five_hour.remaining_percent, Some(50.0));
    assert_eq!(pooled.five_hour.soonest_reset, None);
}

#[test]
fn live_inputs_are_not_marked_stale() {
    let h = Harness::new();
    let core = pooled_core(&h, &[(ID_A, 10.0, 10.0), (ID_B, 20.0, 20.0)]);

    let pooled = core.list_accounts().unwrap().pooled;

    assert_eq!(pooled.five_hour.stale_since_ms, None);
    assert_eq!(pooled.weekly.stale_since_ms, None);
}

#[test]
fn a_stale_input_marks_the_pool_with_the_age_of_its_oldest_stale_number() {
    let h = Harness::new();
    signed_in(&h, ID_A, TOKEN_A);
    signed_in(&h, ID_B, TOKEN_B);
    script_ok(&h, "a@example.com", 10.0, 10.0);
    script_ok(&h, "b@example.com", 20.0, 20.0);
    let core = h.core();
    core.refresh_usage(false);
    h.clock.advance(Duration::from_secs(20 * 60));
    // A's poll now fails (stale); B answers fresh.
    h.http.respond(USAGE_URL, 500, "{}");
    h.http.respond(
        USAGE_URL,
        200,
        &usage_body(25.0, FIVE_RESET, 25.0, WEEK_RESET),
    );
    core.refresh_usage(true);

    let pooled = core.list_accounts().unwrap().pooled;

    assert_eq!(pooled.five_hour.remaining_percent, Some(82.5));
    assert_eq!(pooled.five_hour.stale_since_ms, Some(NOW_MS));
    assert_eq!(pooled.weekly.stale_since_ms, Some(NOW_MS));
}

/// The extra-usage spend read from a body whose only spend is `spend.used`.
fn spend_used_minor(used_json: &str) -> Option<i64> {
    let h = Harness::new();
    signed_in(&h, ID_A, TOKEN_A);
    h.http.respond(
        USAGE_URL,
        200,
        &format!(
            r#"{{"five_hour":{{"utilization":1,"resets_at":null}},
                "seven_day":{{"utilization":2,"resets_at":null}},
                "extra_usage":{{"is_enabled":true}},
                "spend":{{"used":{used_json}}}}}"#
        ),
    );
    h.http
        .respond(PROFILE_URL, 200, &profile_body("a@example.com"));
    let core = h.core();

    core.refresh_usage(false);

    account(&core, ID_A)
        .usage
        .unwrap()
        .extra_usage
        .unwrap()
        .used_minor
}

#[test]
fn spend_money_object_in_another_currency_is_not_counted() {
    assert_eq!(
        spend_used_minor(r#"{"amount_minor":830,"currency":"EUR","exponent":2}"#),
        None
    );
    assert_eq!(
        spend_used_minor(r#"{"amount_minor":830,"exponent":2}"#),
        None
    );
}

#[test]
fn spend_money_object_converts_its_exponent_to_cents() {
    // 830 whole dollars.
    assert_eq!(
        spend_used_minor(r#"{"amount_minor":830,"currency":"USD","exponent":0}"#),
        Some(83_000)
    );
    // 830 thousandths of a dollar.
    assert_eq!(
        spend_used_minor(r#"{"amount_minor":830,"currency":"USD","exponent":3}"#),
        Some(83)
    );
}
