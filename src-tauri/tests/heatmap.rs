//! The 30-day heatmap, through the core: given these transcripts, usage
//! history and polls at this time, what does each day show?
//!
//! Transcripts, ids and prices in these fixtures are synthetic. The harness
//! clock reads 2026-06-15 12:00 UTC, so the window is 2026-05-17 to 2026-06-15.

mod support;

use clawshell_lib::core::Heatmap;
use clawshell_lib::core::cost::{NoEstimate, PRICES_URL};
use clawshell_lib::core::usage::{PROFILE_URL, USAGE_URL};
use serde_json::json;
use std::time::Duration;
use support::Harness;

const ID_A: &str = "3f2b8c1e-5a4d-4e7b-9c10-2d6f8a9b0c11";
const ID_B: &str = "a0b1c2d3-e4f5-4a6b-8c7d-9e0f1a2b3c4d";
const SESSION_A: &str = "11111111-aaaa-4aaa-8aaa-111111111111";
const SESSION_B: &str = "22222222-bbbb-4bbb-8bbb-111111111111";
const MODEL: &str = "claude-test-1";
const NOW_MS: i64 = 1_781_524_800_000;

/// $1 per million input tokens.
const PRICES: &str = r#"{
    "claude-test-1": {"input_cost_per_token": 1e-6, "output_cost_per_token": 5e-6}
}"#;

/// A transcript line costing `million_tokens` dollars at `ts`.
fn line(id: &str, ts: &str, million_tokens: u64) -> String {
    json!({
        "type": "assistant",
        "timestamp": ts,
        "requestId": format!("req_{id}"),
        "message": {
            "id": id,
            "model": MODEL,
            "content": [{"type": "text", "text": "private reply text"}],
            "usage": {"input_tokens": million_tokens * 1_000_000, "output_tokens": 0},
        },
    })
    .to_string()
}

/// An account whose session wrote one transcript with a message per `(ts, dollars)`.
fn spender(h: &Harness, id: &str, session: &str, messages: &[(&str, u64)]) {
    h.fixture.add_profile(id);
    h.fixture.add_session(id, session, &[]);
    let lines: Vec<String> = messages
        .iter()
        .enumerate()
        .map(|(i, (ts, dollars))| line(&format!("{session}-{i}"), ts, *dollars))
        .collect();
    let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
    h.fixture
        .add_transcript(&format!("proj-x/{session}.jsonl"), &refs);
}

fn prices_ok(h: &Harness) {
    h.http.respond(PRICES_URL, 200, PRICES);
}

fn day<'a>(map: &'a Heatmap, date: &str) -> &'a clawshell_lib::core::cost::HeatmapDay {
    map.days
        .iter()
        .find(|d| d.date == date)
        .unwrap_or_else(|| panic!("{date} is in the window"))
}

fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 1e-9,
        "expected {expected}, got {actual}"
    );
}

#[test]
fn covers_thirty_local_days_ending_today_oldest_first() {
    let h = Harness::new();
    let map = h.core().heatmap();

    assert_eq!(map.days.len(), 30);
    assert_eq!(map.days.first().unwrap().date, "2026-05-17");
    assert_eq!(map.days.last().unwrap().date, "2026-06-15");
}

#[test]
fn sums_each_days_code_estimate_and_tells_idle_days_from_unknown_ones() {
    let h = Harness::new();
    spender(
        &h,
        ID_A,
        SESSION_A,
        &[
            ("2026-06-10T09:00:00.000Z", 1),
            ("2026-06-10T10:00:00.000Z", 2),
            ("2026-06-13T10:00:00.000Z", 4),
        ],
    );
    prices_ok(&h);

    let map = h.core().heatmap();

    // Before the first recorded message nothing is known.
    assert_eq!(day(&map, "2026-06-09").usd, None);
    assert_close(day(&map, "2026-06-10").usd.as_ref().unwrap().total, 3.0);
    // After it, a day without messages is a real zero.
    let idle = day(&map, "2026-06-11")
        .usd
        .as_ref()
        .expect("zero, not unknown");
    assert_close(idle.total, 0.0);
    assert_close(day(&map, "2026-06-13").usd.as_ref().unwrap().total, 4.0);
    assert_close(day(&map, "2026-06-15").usd.as_ref().unwrap().total, 0.0);
}

#[test]
fn breaks_a_day_down_by_account_biggest_first() {
    let h = Harness::new();
    spender(&h, ID_A, SESSION_A, &[("2026-06-12T09:00:00.000Z", 1)]);
    spender(&h, ID_B, SESSION_B, &[("2026-06-12T09:00:00.000Z", 5)]);
    prices_ok(&h);

    let map = h.core().heatmap();

    let usd = day(&map, "2026-06-12").usd.as_ref().unwrap();
    assert_close(usd.total, 6.0);
    let rows: Vec<(&str, f64)> = usd
        .accounts
        .iter()
        .map(|a| (a.id.as_str(), a.value))
        .collect();
    assert_eq!(rows, vec![(ID_B, 5.0), (ID_A, 1.0)]);
}

#[test]
fn buckets_messages_by_the_local_day() {
    let h = Harness::new();
    h.clock.set_utc_offset_hours(3);
    // 22:30 UTC on June 12 is already June 13 at UTC+3.
    spender(&h, ID_A, SESSION_A, &[("2026-06-12T22:30:00.000Z", 2)]);
    prices_ok(&h);

    let map = h.core().heatmap();

    assert_close(day(&map, "2026-06-13").usd.as_ref().unwrap().total, 2.0);
    assert_eq!(day(&map, "2026-06-12").usd, None);
}

#[test]
fn ends_on_the_local_today_not_the_utc_one() {
    let h = Harness::new();
    // 12:00 UTC is still the 14th at UTC-13.
    h.clock.set_utc_offset_hours(-13);

    let map = h.core().heatmap();

    assert_eq!(map.days.last().unwrap().date, "2026-06-14");
    assert_eq!(map.days.first().unwrap().date, "2026-05-16");
}

#[test]
fn leaves_out_messages_older_than_the_window() {
    let h = Harness::new();
    spender(
        &h,
        ID_A,
        SESSION_A,
        &[
            ("2026-05-16T23:00:00.000Z", 7),
            ("2026-05-17T00:30:00.000Z", 1),
        ],
    );
    prices_ok(&h);

    let map = h.core().heatmap();

    let oldest = map.days.first().unwrap();
    assert_eq!(oldest.date, "2026-05-17");
    assert_close(oldest.usd.as_ref().unwrap().total, 1.0);
}

#[test]
fn counts_no_extra_usage_in_the_dollars() {
    let h = Harness::new();
    spender(&h, ID_A, SESSION_A, &[("2026-06-12T09:00:00.000Z", 1)]);
    h.fixture.sign_in(ID_A);
    h.tokens.set(
        &h.fixture.profile_path(ID_A),
        "sk-test-token",
        Some(NOW_MS + 3_600_000),
    );
    h.http.respond(
        USAGE_URL,
        200,
        r#"{"seven_day":{"utilization":10,"resets_at":null},
            "extra_usage":{"is_enabled":true,"used_credits":2500,"monthly_limit":5000}}"#,
    );
    h.http
        .respond(PROFILE_URL, 200, r#"{"account":{"email":"a@example.com"}}"#);
    prices_ok(&h);
    let core = h.core();
    core.refresh_usage(false);

    let map = core.heatmap();

    assert_close(day(&map, "2026-06-12").usd.as_ref().unwrap().total, 1.0);
    assert_close(day(&map, "2026-06-15").usd.as_ref().unwrap().total, 0.0);
}

#[test]
fn shows_no_dollars_when_the_prices_cannot_be_fetched() {
    let h = Harness::new();
    spender(&h, ID_A, SESSION_A, &[("2026-06-12T09:00:00.000Z", 1)]);
    h.http.fail(PRICES_URL);

    let map = h.core().heatmap();

    assert_eq!(map.no_estimate, Some(NoEstimate::PricesUnavailable));
    assert!(map.days.iter().all(|d| d.usd.is_none()));
}

#[test]
fn needs_no_prices_when_there_are_no_transcripts() {
    let h = Harness::new();
    h.fixture.add_profile(ID_A);

    let map = h.core().heatmap();

    assert_eq!(map.no_estimate, None);
    assert!(map.days.iter().all(|d| d.usd.is_none()));
    assert!(h.http.requests().is_empty());
}

// Percent mode: the peak weekly utilisation of each day.

/// A `plan-usage-history.json` with `(epoch ms, weekly %)` samples.
fn history(samples: &[(i64, f64)]) -> String {
    let rows: Vec<_> = samples
        .iter()
        .map(|(t, sd)| json!({"t": t, "org": "o", "u": {"fh": 1.0, "sd": sd}}))
        .collect();
    json!({"version": 2, "samples": rows}).to_string()
}

/// 2026-06-DD at HH:00 UTC as epoch ms.
fn june(day: i64, hour: i64) -> i64 {
    NOW_MS + (day - 15) * 86_400_000 + (hour - 12) * 3_600_000
}

#[test]
fn takes_a_days_peak_weekly_percent_from_the_local_history() {
    let h = Harness::new();
    h.fixture.add_profile(ID_A);
    h.fixture.write_usage_history(
        ID_A,
        &history(&[
            (june(10, 8), 20.0),
            (june(10, 16), 35.0),
            (june(10, 20), 30.0),
        ]),
    );

    let map = h.core().heatmap();

    let percent = day(&map, "2026-06-10").percent.as_ref().unwrap();
    assert_close(percent.total, 35.0);
    assert_eq!(percent.accounts.len(), 1);
}

#[test]
fn tells_a_day_without_readings_from_a_day_at_zero() {
    let h = Harness::new();
    h.fixture.add_profile(ID_A);
    h.fixture
        .write_usage_history(ID_A, &history(&[(june(10, 8), 0.0)]));

    let map = h.core().heatmap();

    assert_eq!(day(&map, "2026-06-09").percent, None);
    assert_close(day(&map, "2026-06-10").percent.as_ref().unwrap().total, 0.0);
    assert_eq!(day(&map, "2026-06-11").percent, None);
}

#[test]
fn averages_the_peaks_across_the_accounts_that_have_data_that_day() {
    let h = Harness::new();
    h.fixture.add_profile(ID_A);
    h.fixture.add_profile(ID_B);
    h.fixture
        .write_usage_history(ID_A, &history(&[(june(10, 8), 40.0), (june(11, 8), 50.0)]));
    h.fixture
        .write_usage_history(ID_B, &history(&[(june(10, 9), 20.0)]));

    let map = h.core().heatmap();

    // Both have data on the 10th; only A on the 11th (B is not counted as 0).
    let tenth = day(&map, "2026-06-10").percent.as_ref().unwrap();
    assert_close(tenth.total, 30.0);
    let rows: Vec<(&str, f64)> = tenth
        .accounts
        .iter()
        .map(|a| (a.id.as_str(), a.value))
        .collect();
    assert_eq!(rows, vec![(ID_A, 40.0), (ID_B, 20.0)]);
    let eleventh = day(&map, "2026-06-11").percent.as_ref().unwrap();
    assert_close(eleventh.total, 50.0);
    assert_eq!(eleventh.accounts.len(), 1);
}

#[test]
fn buckets_history_samples_by_the_local_day() {
    let h = Harness::new();
    h.clock.set_utc_offset_hours(-5);
    h.fixture.add_profile(ID_A);
    // 02:00 UTC on June 11 is still June 10 at UTC-5.
    h.fixture
        .write_usage_history(ID_A, &history(&[(june(11, 2), 44.0)]));

    let map = h.core().heatmap();

    assert_close(
        day(&map, "2026-06-10").percent.as_ref().unwrap().total,
        44.0,
    );
    assert_eq!(day(&map, "2026-06-11").percent, None);
}

#[test]
fn adds_this_sessions_polls_to_the_history() {
    let h = Harness::new();
    h.fixture.add_profile(ID_A);
    h.fixture.sign_in(ID_A);
    h.tokens.set(
        &h.fixture.profile_path(ID_A),
        "sk-test-token",
        Some(NOW_MS + 86_400_000),
    );
    h.fixture
        .write_usage_history(ID_A, &history(&[(june(15, 8), 10.0)]));
    let core = h.core();
    for weekly in [30.0, 25.0] {
        h.http.respond(
            USAGE_URL,
            200,
            &format!(r#"{{"seven_day":{{"utilization":{weekly},"resets_at":null}}}}"#),
        );
        h.http
            .respond(PROFILE_URL, 200, r#"{"account":{"email":"a@example.com"}}"#);
        core.refresh_usage(true);
        h.clock.advance(Duration::from_secs(300));
    }

    let map = core.heatmap();

    // The peak of the history reading and both polls.
    assert_close(
        day(&map, "2026-06-15").percent.as_ref().unwrap().total,
        30.0,
    );
}

#[test]
fn ignores_history_older_than_the_window_and_unreadable_files() {
    let h = Harness::new();
    h.fixture.add_profile(ID_A);
    h.fixture.add_profile(ID_B);
    h.fixture
        .write_usage_history(ID_A, &history(&[(june(15, 8) - 40 * 86_400_000, 90.0)]));
    h.fixture.write_usage_history(ID_B, "not json");

    let map = h.core().heatmap();

    assert!(map.days.iter().all(|d| d.percent.is_none()));
}
