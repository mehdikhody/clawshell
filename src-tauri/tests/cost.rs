//! This month's cost, through the core: given these profiles, transcripts,
//! prices and usage responses at this time, what does the month show?
//!
//! Transcripts, ids and prices in these fixtures are synthetic.

mod support;

use clawshell_lib::core::cost::{MonthCost, NoEstimate, PRICES_URL};
use clawshell_lib::core::usage::USAGE_URL;
use serde_json::json;
use std::time::Duration;
use support::Harness;

const ID_A: &str = "3f2b8c1e-5a4d-4e7b-9c10-2d6f8a9b0c11";
const ID_B: &str = "a0b1c2d3-e4f5-4a6b-8c7d-9e0f1a2b3c4d";
const SESSION_A: &str = "11111111-aaaa-4aaa-8aaa-111111111111";
const SESSION_A_OLD: &str = "11111111-aaaa-4aaa-8aaa-222222222222";
const SESSION_B: &str = "22222222-bbbb-4bbb-8bbb-111111111111";
const SESSION_OTHER: &str = "99999999-9999-4999-8999-999999999999";
const MODEL: &str = "claude-test-1";
/// 2026-06-15 on the harness clock (12:00 UTC).
const TS: &str = "2026-06-15T10:00:00.000Z";

/// $1 / $5 per million input / output, cache writes $1.25 (5 minutes) and $2
/// (1 hour), cache reads $0.10.
const PRICES: &str = r#"{
    "sample_spec": {"input_cost_per_token": 1.0},
    "claude-test-1": {
        "input_cost_per_token": 1e-6,
        "output_cost_per_token": 5e-6,
        "cache_creation_input_token_cost": 1.25e-6,
        "cache_creation_input_token_cost_above_1hr": 2e-6,
        "cache_read_input_token_cost": 1e-7
    },
    "claude-test-2": {
        "input_cost_per_token": 3e-6,
        "output_cost_per_token": 15e-6,
        "cache_creation_input_token_cost": 3.75e-6,
        "cache_read_input_token_cost": 3e-7
    }
}"#;

struct Msg {
    id: &'static str,
    request: Option<&'static str>,
    model: &'static str,
    ts: &'static str,
    input: u64,
    output: u64,
    cache_5m: u64,
    cache_1h: u64,
    cache_read: u64,
}

fn msg(id: &'static str, request: &'static str) -> Msg {
    Msg {
        id,
        request: Some(request),
        model: MODEL,
        ts: TS,
        input: 0,
        output: 0,
        cache_5m: 0,
        cache_1h: 0,
        cache_read: 0,
    }
}

impl Msg {
    fn line(&self) -> String {
        let mut record = json!({
            "type": "assistant",
            "timestamp": self.ts,
            "message": {
                "id": self.id,
                "model": self.model,
                "content": [{"type": "text", "text": "private reply text"}],
                "usage": {
                    "input_tokens": self.input,
                    "output_tokens": self.output,
                    "cache_creation_input_tokens": self.cache_5m + self.cache_1h,
                    "cache_creation": {
                        "ephemeral_5m_input_tokens": self.cache_5m,
                        "ephemeral_1h_input_tokens": self.cache_1h,
                    },
                    "cache_read_input_tokens": self.cache_read,
                },
            },
        });
        if let Some(request) = self.request {
            record["requestId"] = json!(request);
        }
        record.to_string()
    }
}

/// A user turn: carries text but no usage, and must never matter.
fn user_line() -> String {
    json!({"type": "user", "timestamp": TS, "message": {"content": "private prompt text"}})
        .to_string()
}

fn write_transcript(h: &Harness, relative: &str, messages: &[Msg]) {
    let mut lines = vec![user_line()];
    lines.extend(messages.iter().map(Msg::line));
    let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
    h.fixture.add_transcript(relative, &refs);
}

/// A profile with a session file mapping `session` to it.
fn account_with_session(h: &Harness, id: &str, session: &str, prior: &[&str]) {
    h.fixture.add_profile(id);
    h.fixture.add_session(id, session, prior);
}

fn prices_ok(h: &Harness) {
    h.http.respond(PRICES_URL, 200, PRICES);
}

fn account_cost<'a>(cost: &'a MonthCost, id: &str) -> &'a clawshell_lib::core::cost::AccountCost {
    cost.accounts
        .iter()
        .find(|a| a.id == id)
        .expect("account in breakdown")
}

fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 1e-9,
        "expected {expected}, got {actual}"
    );
}

#[test]
fn prices_a_mapped_transcript_for_every_token_class() {
    let h = Harness::new();
    account_with_session(&h, ID_A, SESSION_A, &[]);
    let mut m = msg("msg_1", "req_1");
    m.input = 1_000;
    m.output = 200;
    m.cache_5m = 400;
    m.cache_1h = 100;
    m.cache_read = 2_000;
    write_transcript(&h, &format!("proj-x/{SESSION_A}.jsonl"), &[m]);
    prices_ok(&h);

    let cost = h.core().month_cost();

    // 1000*1e-6 + 200*5e-6 + 400*1.25e-6 + 100*2e-6 + 2000*1e-7
    assert_close(cost.estimate_usd.unwrap(), 0.0029);
    assert_close(account_cost(&cost, ID_A).estimate_usd.unwrap(), 0.0029);
}

#[test]
fn prices_each_message_by_its_own_model() {
    let h = Harness::new();
    account_with_session(&h, ID_A, SESSION_A, &[]);
    let mut cheap = msg("msg_1", "req_1");
    cheap.input = 1_000_000;
    let mut dear = msg("msg_2", "req_2");
    dear.model = "claude-test-2";
    dear.input = 1_000_000;
    write_transcript(&h, &format!("proj-x/{SESSION_A}.jsonl"), &[cheap, dear]);
    prices_ok(&h);

    let cost = h.core().month_cost();

    assert_close(cost.estimate_usd.unwrap(), 4.0);
}

#[test]
fn bills_cache_writes_without_a_split_as_five_minute_writes() {
    let h = Harness::new();
    account_with_session(&h, ID_A, SESSION_A, &[]);
    let line = json!({
        "type": "assistant", "timestamp": TS, "requestId": "req_1",
        "message": {"id": "msg_1", "model": MODEL, "usage": {
            "input_tokens": 0, "output_tokens": 0,
            "cache_creation_input_tokens": 1_000_000, "cache_read_input_tokens": 0}}
    })
    .to_string();
    h.fixture
        .add_transcript(&format!("proj-x/{SESSION_A}.jsonl"), &[&line]);
    prices_ok(&h);

    assert_close(h.core().month_cost().estimate_usd.unwrap(), 1.25);
}

#[test]
fn ignores_transcripts_that_map_to_no_account() {
    let h = Harness::new();
    account_with_session(&h, ID_A, SESSION_A, &[]);
    let mut mine = msg("msg_1", "req_1");
    mine.input = 1_000_000;
    write_transcript(&h, &format!("proj-x/{SESSION_A}.jsonl"), &[mine]);
    let mut stranger = msg("msg_2", "req_2");
    stranger.input = 5_000_000;
    write_transcript(&h, &format!("proj-y/{SESSION_OTHER}.jsonl"), &[stranger]);
    prices_ok(&h);

    let cost = h.core().month_cost();

    assert_close(cost.estimate_usd.unwrap(), 1.0);
}

#[test]
fn maps_a_transcript_by_a_prior_session_id() {
    let h = Harness::new();
    account_with_session(&h, ID_A, SESSION_A, &[SESSION_A_OLD]);
    let mut old = msg("msg_1", "req_1");
    old.input = 2_000_000;
    write_transcript(&h, &format!("proj-x/{SESSION_A_OLD}.jsonl"), &[old]);
    prices_ok(&h);

    let cost = h.core().month_cost();

    assert_close(account_cost(&cost, ID_A).estimate_usd.unwrap(), 2.0);
}

#[test]
fn attributes_each_transcript_to_its_own_account() {
    let h = Harness::new();
    account_with_session(&h, ID_A, SESSION_A, &[]);
    account_with_session(&h, ID_B, SESSION_B, &[]);
    let mut a = msg("msg_1", "req_1");
    a.input = 1_000_000;
    let mut b = msg("msg_2", "req_2");
    b.input = 3_000_000;
    write_transcript(&h, &format!("proj-x/{SESSION_A}.jsonl"), &[a]);
    write_transcript(&h, &format!("proj-x/{SESSION_B}.jsonl"), &[b]);
    prices_ok(&h);

    let cost = h.core().month_cost();

    assert_close(account_cost(&cost, ID_A).estimate_usd.unwrap(), 1.0);
    assert_close(account_cost(&cost, ID_B).estimate_usd.unwrap(), 3.0);
}

#[test]
fn counts_the_work_of_subagent_transcripts_for_the_parent_session() {
    let h = Harness::new();
    account_with_session(&h, ID_A, SESSION_A, &[]);
    let mut sub = msg("msg_1", "req_1");
    sub.input = 1_000_000;
    write_transcript(
        &h,
        &format!("proj-x/{SESSION_A}/subagents/agent-1.jsonl"),
        &[sub],
    );
    prices_ok(&h);

    assert_close(h.core().month_cost().estimate_usd.unwrap(), 1.0);
}

#[test]
fn counts_a_streamed_message_once() {
    let h = Harness::new();
    account_with_session(&h, ID_A, SESSION_A, &[]);
    // The same message streamed three times: the final record has the full output.
    let mut first = msg("msg_1", "req_1");
    first.input = 1_000_000;
    first.output = 10;
    let mut second = msg("msg_1", "req_1");
    second.input = 1_000_000;
    second.output = 400_000;
    let mut third = msg("msg_1", "req_1");
    third.input = 1_000_000;
    third.output = 1_000_000;
    write_transcript(
        &h,
        &format!("proj-x/{SESSION_A}.jsonl"),
        &[first, second, third],
    );
    prices_ok(&h);

    // One message: 1M input ($1) + its final 1M output ($5).
    assert_close(h.core().month_cost().estimate_usd.unwrap(), 6.0);
}

#[test]
fn counts_a_message_once_across_transcripts() {
    let h = Harness::new();
    account_with_session(&h, ID_A, SESSION_A, &[SESSION_A_OLD]);
    let shared = || {
        let mut m = msg("msg_1", "req_1");
        m.input = 1_000_000;
        m
    };
    write_transcript(&h, &format!("proj-x/{SESSION_A_OLD}.jsonl"), &[shared()]);
    // Resuming copies the earlier history into the new transcript.
    write_transcript(&h, &format!("proj-x/{SESSION_A}.jsonl"), &[shared()]);
    prices_ok(&h);

    assert_close(h.core().month_cost().estimate_usd.unwrap(), 1.0);
}

#[test]
fn falls_back_to_the_message_id_when_there_is_no_request_id() {
    let h = Harness::new();
    account_with_session(&h, ID_A, SESSION_A, &[]);
    let mut a = msg("msg_1", "");
    a.request = None;
    a.input = 1_000_000;
    let mut b = msg("msg_1", "");
    b.request = None;
    b.input = 1_000_000;
    write_transcript(&h, &format!("proj-x/{SESSION_A}.jsonl"), &[a, b]);
    prices_ok(&h);

    assert_close(h.core().month_cost().estimate_usd.unwrap(), 1.0);
}

#[test]
fn counts_the_same_message_id_under_different_requests_separately() {
    let h = Harness::new();
    account_with_session(&h, ID_A, SESSION_A, &[]);
    let mut a = msg("msg_1", "req_1");
    a.input = 1_000_000;
    let mut b = msg("msg_1", "req_2");
    b.input = 1_000_000;
    write_transcript(&h, &format!("proj-x/{SESSION_A}.jsonl"), &[a, b]);
    prices_ok(&h);

    assert_close(h.core().month_cost().estimate_usd.unwrap(), 2.0);
}

// Month bucketing: the local calendar month.

fn at(ts: &'static str) -> Msg {
    let mut m = msg("msg_t", "req_t");
    m.ts = ts;
    m.input = 1_000_000;
    m
}

/// Prices one message at `ts` on a clock set `offset_hours` from UTC.
fn estimate_for(ts: &'static str, offset_hours: i32) -> f64 {
    let h = Harness::new();
    h.clock.set_utc_offset_hours(offset_hours);
    account_with_session(&h, ID_A, SESSION_A, &[]);
    write_transcript(&h, &format!("proj-x/{SESSION_A}.jsonl"), &[at(ts)]);
    prices_ok(&h);
    h.core().month_cost().estimate_usd.unwrap()
}

#[test]
fn counts_only_messages_from_the_current_month() {
    assert_close(estimate_for("2026-06-01T00:30:00.000Z", 0), 1.0);
    assert_close(estimate_for("2026-06-30T23:59:59.000Z", 0), 1.0);
    assert_close(estimate_for("2026-05-31T23:59:59.000Z", 0), 0.0);
    assert_close(estimate_for("2026-07-01T00:00:00.000Z", 0), 0.0);
}

#[test]
fn buckets_by_the_local_day_west_of_utc() {
    // 02:00 UTC on June 1 is still May 31 at UTC-5.
    assert_close(estimate_for("2026-06-01T02:00:00.000Z", -5), 0.0);
    // 03:00 UTC on July 1 is still June 30 at UTC-5.
    assert_close(estimate_for("2026-07-01T03:00:00.000Z", -5), 1.0);
}

#[test]
fn buckets_by_the_local_day_east_of_utc() {
    // 22:00 UTC on May 31 is already June 1 at UTC+3.
    assert_close(estimate_for("2026-05-31T22:00:00.000Z", 3), 1.0);
    // 22:00 UTC on June 30 is already July 1 at UTC+3.
    assert_close(estimate_for("2026-06-30T22:00:00.000Z", 3), 0.0);
}

#[test]
fn understands_timestamps_with_an_explicit_offset() {
    // 2026-06-01T01:00+03:00 is 22:00 UTC on May 31, i.e. June 1 at UTC+3.
    assert_close(estimate_for("2026-06-01T01:00:00.000+03:00", 3), 1.0);
    assert_close(estimate_for("2026-06-01T01:00:00.000+03:00", 0), 0.0);
}

// Pricing details

#[test]
fn finds_a_price_under_the_provider_prefix_or_without_the_date() {
    let h = Harness::new();
    account_with_session(&h, ID_A, SESSION_A, &[]);
    let mut dated = msg("msg_1", "req_1");
    dated.model = "claude-test-1-20250101";
    dated.input = 1_000_000;
    let mut prefixed = msg("msg_2", "req_2");
    prefixed.model = "claude-prefixed";
    prefixed.input = 1_000_000;
    write_transcript(&h, &format!("proj-x/{SESSION_A}.jsonl"), &[dated, prefixed]);
    h.http.respond(
        PRICES_URL,
        200,
        r#"{"claude-test-1": {"input_cost_per_token": 1e-6, "output_cost_per_token": 5e-6},
            "anthropic/claude-prefixed": {"input_cost_per_token": 2e-6, "output_cost_per_token": 5e-6}}"#,
    );

    assert_close(h.core().month_cost().estimate_usd.unwrap(), 3.0);
}

#[test]
fn derives_cache_prices_the_list_omits_from_the_input_price() {
    let h = Harness::new();
    account_with_session(&h, ID_A, SESSION_A, &[]);
    let mut m = msg("msg_1", "req_1");
    m.cache_1h = 1_000_000;
    m.cache_read = 1_000_000;
    write_transcript(&h, &format!("proj-x/{SESSION_A}.jsonl"), &[m]);
    h.http.respond(
        PRICES_URL,
        200,
        r#"{"claude-test-1": {"input_cost_per_token": 1e-6, "output_cost_per_token": 5e-6}}"#,
    );

    // 1h writes at 2x input ($2) plus reads at 0.1x input ($0.10).
    assert_close(h.core().month_cost().estimate_usd.unwrap(), 2.1);
}

#[test]
fn leaves_out_and_counts_messages_whose_model_has_no_price() {
    let h = Harness::new();
    account_with_session(&h, ID_A, SESSION_A, &[]);
    let mut known = msg("msg_1", "req_1");
    known.input = 1_000_000;
    let mut unknown = msg("msg_2", "req_2");
    unknown.model = "claude-mystery";
    unknown.input = 9_000_000;
    let mut synthetic = msg("msg_3", "req_3");
    synthetic.model = "<synthetic>";
    write_transcript(
        &h,
        &format!("proj-x/{SESSION_A}.jsonl"),
        &[known, unknown, synthetic],
    );
    prices_ok(&h);

    let cost = h.core().month_cost();

    assert_close(cost.estimate_usd.unwrap(), 1.0);
    assert_eq!(cost.unpriced_messages, 1);
}

// Offline

#[test]
fn shows_no_estimate_when_the_prices_cannot_be_fetched() {
    let h = Harness::new();
    account_with_session(&h, ID_A, SESSION_A, &[]);
    let mut m = msg("msg_1", "req_1");
    m.input = 1_000_000;
    write_transcript(&h, &format!("proj-x/{SESSION_A}.jsonl"), &[m]);
    h.http.fail(PRICES_URL);

    let cost = h.core().month_cost();

    assert_eq!(cost.estimate_usd, None);
    assert_eq!(cost.total_usd, None);
    assert_eq!(cost.no_estimate, Some(NoEstimate::PricesUnavailable));
    let row = account_cost(&cost, ID_A);
    assert_eq!(row.estimate_usd, None);
    assert_eq!(row.total_usd, None);
    assert_eq!(row.share, None);
}

#[test]
fn shows_no_estimate_for_a_failing_or_unreadable_price_list() {
    for (status, body) in [(500, "oops"), (200, "not json"), (200, "{}"), (404, PRICES)] {
        let h = Harness::new();
        account_with_session(&h, ID_A, SESSION_A, &[]);
        let mut m = msg("msg_1", "req_1");
        m.input = 1_000_000;
        write_transcript(&h, &format!("proj-x/{SESSION_A}.jsonl"), &[m]);
        h.http.respond(PRICES_URL, status, body);

        let cost = h.core().month_cost();

        assert_eq!(cost.estimate_usd, None, "status {status}, body {body}");
        assert_eq!(cost.no_estimate, Some(NoEstimate::PricesUnavailable));
    }
}

#[test]
fn recovers_once_the_prices_can_be_fetched_again() {
    let h = Harness::new();
    account_with_session(&h, ID_A, SESSION_A, &[]);
    let mut m = msg("msg_1", "req_1");
    m.input = 1_000_000;
    write_transcript(&h, &format!("proj-x/{SESSION_A}.jsonl"), &[m]);
    h.http.fail(PRICES_URL);
    let core = h.core();
    assert_eq!(core.month_cost().estimate_usd, None);

    // Too soon to ask again: the failure stands without another request.
    assert_eq!(core.month_cost().estimate_usd, None);
    assert_eq!(h.http.requests().len(), 1);

    h.clock.advance(Duration::from_secs(61));
    prices_ok(&h);
    assert_close(core.month_cost().estimate_usd.unwrap(), 1.0);
}

#[test]
fn fetches_the_prices_once_an_hour() {
    let h = Harness::new();
    account_with_session(&h, ID_A, SESSION_A, &[]);
    let mut m = msg("msg_1", "req_1");
    m.input = 1_000_000;
    write_transcript(&h, &format!("proj-x/{SESSION_A}.jsonl"), &[m]);
    prices_ok(&h);
    let core = h.core();

    core.month_cost();
    h.clock.advance(Duration::from_secs(30 * 60));
    core.month_cost();
    assert_eq!(h.http.requests().len(), 1);

    h.clock.advance(Duration::from_secs(31 * 60));
    prices_ok(&h);
    core.month_cost();
    assert_eq!(h.http.requests().len(), 2);
}

#[test]
fn needs_no_prices_when_there_is_nothing_to_price() {
    let h = Harness::new();
    account_with_session(&h, ID_A, SESSION_A, &[]);

    let cost = h.core().month_cost();

    assert_eq!(cost.estimate_usd, Some(0.0));
    assert_eq!(cost.no_estimate, None);
    assert!(h.http.requests().is_empty());
}

#[test]
fn sends_nothing_but_a_plain_price_request() {
    let h = Harness::new();
    account_with_session(&h, ID_A, SESSION_A, &[]);
    let mut m = msg("msg_1", "req_1");
    m.input = 1_000_000;
    write_transcript(&h, &format!("proj-x/{SESSION_A}.jsonl"), &[m]);
    prices_ok(&h);

    h.core().month_cost();

    let requests = h.http.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].url, PRICES_URL);
    assert!(requests[0].headers.is_empty());
}

// The month total

/// A signed-in account whose usage endpoint reports `extra_usage_json`.
fn with_extra_usage(h: &Harness, id: &str, extra_usage_json: &str) {
    h.fixture.sign_in(id);
    h.tokens.set(
        &h.fixture.profile_path(id),
        "sk-test-token",
        Some(h.now_ms() + 3_600_000),
    );
    h.http.respond(
        USAGE_URL,
        200,
        &format!(
            r#"{{"five_hour":{{"utilization":10.0,"resets_at":"2026-06-15T15:00:00+00:00"}},
                "seven_day":{{"utilization":20.0,"resets_at":"2026-06-19T09:30:00+00:00"}},
                {extra_usage_json}}}"#
        ),
    );
    h.http.respond(
        "https://api.anthropic.com/api/oauth/profile",
        200,
        r#"{"account":{"email":"someone@example.test"}}"#,
    );
}

#[test]
fn adds_extra_usage_spend_in_dollars_to_the_estimate() {
    let h = Harness::new();
    account_with_session(&h, ID_A, SESSION_A, &[]);
    with_extra_usage(
        &h,
        ID_A,
        r#""extra_usage":{"is_enabled":true,"used_credits":1250,"monthly_limit":5000}"#,
    );
    let mut m = msg("msg_1", "req_1");
    m.input = 2_000_000;
    write_transcript(&h, &format!("proj-x/{SESSION_A}.jsonl"), &[m]);
    prices_ok(&h);
    let core = h.core();
    core.refresh_usage(true);

    let cost = core.month_cost();

    assert_close(cost.estimate_usd.unwrap(), 2.0);
    assert_close(cost.extra_usd, 12.5);
    assert_close(cost.total_usd.unwrap(), 14.5);
}

#[test]
fn treats_absent_extra_usage_fields_as_zero() {
    let h = Harness::new();
    account_with_session(&h, ID_A, SESSION_A, &[]);
    // The shape of an account with extra usage disabled: no spend fields.
    with_extra_usage(&h, ID_A, r#""extra_usage":{"is_enabled":false}"#);
    let mut m = msg("msg_1", "req_1");
    m.input = 1_000_000;
    write_transcript(&h, &format!("proj-x/{SESSION_A}.jsonl"), &[m]);
    prices_ok(&h);
    let core = h.core();
    core.refresh_usage(true);

    let cost = core.month_cost();

    assert_close(cost.extra_usd, 0.0);
    assert_close(cost.total_usd.unwrap(), 1.0);
}

#[test]
fn counts_no_extra_usage_spend_when_extra_usage_is_disabled() {
    let h = Harness::new();
    account_with_session(&h, ID_A, SESSION_A, &[]);
    // A leftover spend figure on an account whose extra usage is switched off.
    with_extra_usage(
        &h,
        ID_A,
        r#""extra_usage":{"is_enabled":false,"used_credits":1250}"#,
    );
    let mut m = msg("msg_1", "req_1");
    m.input = 1_000_000;
    write_transcript(&h, &format!("proj-x/{SESSION_A}.jsonl"), &[m]);
    prices_ok(&h);
    let core = h.core();
    core.refresh_usage(true);

    let cost = core.month_cost();

    assert_close(cost.extra_usd, 0.0);
    assert_close(account_cost(&cost, ID_A).extra_usd, 0.0);
    assert_close(cost.total_usd.unwrap(), 1.0);
}

#[test]
fn still_reports_extra_usage_when_there_is_no_estimate() {
    let h = Harness::new();
    account_with_session(&h, ID_A, SESSION_A, &[]);
    with_extra_usage(
        &h,
        ID_A,
        r#""extra_usage":{"is_enabled":true,"used_credits":300}"#,
    );
    let mut m = msg("msg_1", "req_1");
    m.input = 1_000_000;
    write_transcript(&h, &format!("proj-x/{SESSION_A}.jsonl"), &[m]);
    h.http.fail(PRICES_URL);
    let core = h.core();
    core.refresh_usage(true);

    let cost = core.month_cost();

    assert_eq!(cost.total_usd, None);
    assert_close(cost.extra_usd, 3.0);
    assert_close(account_cost(&cost, ID_A).extra_usd, 3.0);
}

#[test]
fn breaks_the_month_down_per_account_with_shares_that_add_up() {
    let h = Harness::new();
    account_with_session(&h, ID_A, SESSION_A, &[]);
    account_with_session(&h, ID_B, SESSION_B, &[]);
    with_extra_usage(
        &h,
        ID_A,
        r#""extra_usage":{"is_enabled":true,"used_credits":1250}"#,
    );
    let mut a = msg("msg_1", "req_1");
    a.input = 1_000_000;
    let mut b = msg("msg_2", "req_2");
    b.input = 3_000_000;
    write_transcript(&h, &format!("proj-x/{SESSION_A}.jsonl"), &[a]);
    write_transcript(&h, &format!("proj-x/{SESSION_B}.jsonl"), &[b]);
    prices_ok(&h);
    let core = h.core();
    core.refresh_usage(true);
    core.reorder_accounts(&[ID_B.to_string(), ID_A.to_string()])
        .unwrap();

    let cost = core.month_cost();

    let (a, b) = (account_cost(&cost, ID_A), account_cost(&cost, ID_B));
    assert_close(a.total_usd.unwrap(), 13.5);
    assert_close(b.total_usd.unwrap(), 3.0);
    assert_close(cost.total_usd.unwrap(), 16.5);
    let summed: f64 = cost.accounts.iter().map(|r| r.total_usd.unwrap()).sum();
    assert_close(summed, cost.total_usd.unwrap());
    assert_close(a.share.unwrap(), 13.5 / 16.5);
    assert_close(b.share.unwrap(), 3.0 / 16.5);
    assert_close(cost.accounts.iter().map(|r| r.share.unwrap()).sum(), 1.0);
    // Listed in the saved order, not by spend: B is smaller but was dropped first.
    assert_eq!(cost.accounts[0].id, ID_B);
}

#[test]
fn lists_an_account_with_no_cost_with_a_zero_share() {
    let h = Harness::new();
    account_with_session(&h, ID_A, SESSION_A, &[]);
    account_with_session(&h, ID_B, SESSION_B, &[]);
    let mut a = msg("msg_1", "req_1");
    a.input = 1_000_000;
    write_transcript(&h, &format!("proj-x/{SESSION_A}.jsonl"), &[a]);
    prices_ok(&h);

    let cost = h.core().month_cost();

    let idle = account_cost(&cost, ID_B);
    assert_eq!(idle.total_usd, Some(0.0));
    assert_eq!(idle.share, Some(0.0));
}

#[test]
fn has_no_shares_when_nothing_was_spent() {
    let h = Harness::new();
    account_with_session(&h, ID_A, SESSION_A, &[]);

    let cost = h.core().month_cost();

    assert_eq!(cost.total_usd, Some(0.0));
    assert_eq!(account_cost(&cost, ID_A).share, None);
}

#[test]
fn survives_unreadable_transcripts_and_session_files() {
    let h = Harness::new();
    account_with_session(&h, ID_A, SESSION_A, &[]);
    let dir = h
        .fixture
        .profile_path(ID_A)
        .join("claude-code-sessions/acct/org");
    std::fs::write(dir.join("broken.json"), "{not json").unwrap();
    let mut m = msg("msg_1", "req_1");
    m.input = 1_000_000;
    let good = m.line();
    h.fixture.add_transcript(
        &format!("proj-x/{SESSION_A}.jsonl"),
        &[
            "\"usage\" garbage",
            "",
            &good,
            "{\"message\":{\"usage\":{}}}",
        ],
    );
    prices_ok(&h);

    assert_close(h.core().month_cost().estimate_usd.unwrap(), 1.0);
}
