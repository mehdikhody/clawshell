//! Polling policy (claude-swap's): pure decisions, no clock and no network.

use clawshell_lib::core::polling::*;
use std::time::Duration;

const fn secs(n: u64) -> Duration {
    Duration::from_secs(n)
}

#[test]
fn the_minimum_interval_is_180_seconds() {
    assert_eq!(MIN_INTERVAL, secs(180));
}

#[test]
fn an_exhausted_account_is_polled_every_600_seconds() {
    assert_eq!(adaptive_interval(100.0, true), secs(600));
    assert_eq!(adaptive_interval(100.0, false), secs(600));
}

#[test]
fn an_account_near_a_limit_wants_60_seconds_but_never_below_the_minimum() {
    assert_eq!(adaptive_interval(80.0, true), secs(60));
    assert_eq!(adaptive_interval(99.0, false), secs(60));
    // The minimum interval still holds once the delay is worked out.
    assert_eq!(next_poll_delay(secs(60), 0.0), MIN_INTERVAL);
}

#[test]
fn a_comfortable_account_is_polled_slower_when_idle_than_when_running() {
    assert_eq!(adaptive_interval(10.0, true), secs(300));
    assert_eq!(adaptive_interval(10.0, false), secs(600));
}

#[test]
fn jitter_spreads_the_delay_by_at_most_ten_percent_either_way() {
    assert_eq!(next_poll_delay(secs(600), -1.0), secs(540));
    assert_eq!(next_poll_delay(secs(600), 1.0), secs(660));
    assert_eq!(next_poll_delay(secs(600), 0.0), secs(600));
    // Out-of-range input is clamped, not trusted.
    assert_eq!(next_poll_delay(secs(600), 5.0), secs(660));
}

#[test]
fn jitter_fractions_stay_within_plus_or_minus_one_and_vary_with_the_seed() {
    let all: Vec<f64> = (0..200).map(jitter_unit).collect();
    assert!(all.iter().all(|j| (-1.0..=1.0).contains(j)));
    assert!(all.windows(2).any(|w| w[0] != w[1]));
    assert_eq!(jitter_unit(7), jitter_unit(7));
}

#[test]
fn errors_back_off_exponentially_up_to_1800_seconds() {
    let waits: Vec<_> = (1..=6).map(|n| failure_backoff(n).as_secs()).collect();
    assert_eq!(waits, vec![180, 360, 720, 1440, 1800, 1800]);
}

#[test]
fn retry_after_is_honoured_but_not_below_the_minimum_interval() {
    let wait = rate_limit_wait(Some(900), 1, false);
    assert_eq!(wait, secs(900));
    assert_eq!(rate_limit_wait(Some(30), 1, false), MIN_INTERVAL);
}

#[test]
fn an_absurd_retry_after_is_capped_at_an_hour() {
    assert_eq!(rate_limit_wait(Some(86_400), 1, false), secs(3600));
}

#[test]
fn a_429_without_retry_after_backs_off_exponentially() {
    assert_eq!(rate_limit_wait(None, 1, false), secs(180));
    assert_eq!(rate_limit_wait(None, 3, false), secs(720));
}

#[test]
fn retry_after_zero_means_saturated_and_waits_300_seconds() {
    assert_eq!(rate_limit_wait(Some(0), 1, false), secs(300));
}

#[test]
fn while_the_saturated_floor_is_active_every_wait_is_at_least_360_seconds() {
    assert_eq!(rate_limit_wait(Some(0), 2, true), secs(360));
    assert_eq!(rate_limit_wait(Some(200), 2, true), secs(360));
    assert_eq!(rate_limit_wait(None, 1, true), secs(360));
    assert_eq!(apply_saturated_floor(MIN_INTERVAL, true), secs(360));
    assert_eq!(apply_saturated_floor(secs(600), true), secs(600));
    assert_eq!(apply_saturated_floor(MIN_INTERVAL, false), MIN_INTERVAL);
}

#[test]
fn the_saturated_floor_lasts_an_hour() {
    assert_eq!(SATURATED_FLOOR_FOR, secs(3600));
}

#[test]
fn retry_after_is_read_from_the_headers_whatever_the_case() {
    let h = |name: &str, value: &str| vec![(name.to_string(), value.to_string())];
    assert_eq!(parse_retry_after(&h("Retry-After", "120")), Some(120));
    assert_eq!(parse_retry_after(&h("retry-after", " 0 ")), Some(0));
    assert_eq!(parse_retry_after(&h("RETRY-AFTER", "7")), Some(7));
    assert_eq!(parse_retry_after(&[]), None);
    // An HTTP date is not a number of seconds: treated as absent.
    assert_eq!(
        parse_retry_after(&h("Retry-After", "Wed, 21 Oct 2026 07:28:00 GMT")),
        None
    );
}
