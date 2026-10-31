//! Polling policy, adapted from claude-swap. Pure functions: no clock, no
//! network, no state, so every decision can be tested on its own.

use std::time::Duration;

/// No account is asked for usage more often than this, scheduled or manual.
pub const MIN_INTERVAL: Duration = Duration::from_secs(180);

/// Utilisation at or above which an account counts as near a limit.
pub const NEAR_LIMIT_PERCENT: f64 = 80.0;
/// Utilisation at which an account is out of quota.
pub const EXHAUSTED_PERCENT: f64 = 100.0;

const NEAR_LIMIT_INTERVAL: Duration = Duration::from_secs(60);
const RUNNING_INTERVAL: Duration = Duration::from_secs(300);
const IDLE_INTERVAL: Duration = Duration::from_secs(600);
const EXHAUSTED_INTERVAL: Duration = Duration::from_secs(600);

/// Largest share the delay may drift either way.
const JITTER: f64 = 0.10;

const MAX_ERROR_BACKOFF: Duration = Duration::from_secs(1800);
/// A server asking for longer than this is not obeyed beyond it.
const MAX_RETRY_AFTER: Duration = Duration::from_secs(3600);
/// `Retry-After: 0`: the budget is saturated, so wait this long.
const SATURATED_WAIT: Duration = Duration::from_secs(300);
/// While the saturated floor is active, no wait is shorter than this.
const SATURATED_FLOOR: Duration = Duration::from_secs(360);
/// How long the floor stays active after the first saturated 429.
pub const SATURATED_FLOOR_FOR: Duration = Duration::from_secs(3600);

/// How long to wait before polling an account again, before jitter.
/// Exhausted: 600 s. Near a limit: 60 s (raised to [`MIN_INTERVAL`] by
/// [`next_poll_delay`]). Otherwise 300 s while Claude runs on it, 600 s idle.
pub fn adaptive_interval(max_utilization: f64, running: bool) -> Duration {
    if max_utilization >= EXHAUSTED_PERCENT {
        EXHAUSTED_INTERVAL
    } else if max_utilization >= NEAR_LIMIT_PERCENT {
        NEAR_LIMIT_INTERVAL
    } else if running {
        RUNNING_INTERVAL
    } else {
        IDLE_INTERVAL
    }
}

/// Applies jitter (`unit` in -1..=1, scaled to +-10%) and then the minimum
/// interval, so the floor holds whatever the jitter says.
pub fn next_poll_delay(interval: Duration, unit: f64) -> Duration {
    let unit = unit.clamp(-1.0, 1.0);
    interval.mul_f64(1.0 + JITTER * unit).max(MIN_INTERVAL)
}

/// Deterministic pseudo-random value in -1..=1 from a seed (splitmix64).
pub fn jitter_unit(seed: u64) -> f64 {
    let mut z = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 31;
    (z >> 11) as f64 / (1u64 << 52) as f64 - 1.0
}

/// Exponential backoff after `failures` consecutive errors (1 = first):
/// 180, 360, 720 ... capped at 1800 s.
pub fn failure_backoff(failures: u32) -> Duration {
    let doublings = failures.saturating_sub(1).min(16);
    MIN_INTERVAL
        .saturating_mul(1u32 << doublings)
        .min(MAX_ERROR_BACKOFF)
}

/// How long to stay away after a 429. `retry_after` is the header in seconds,
/// when sent. 0 means the budget is saturated. `floor_active` says the
/// saturated floor from an earlier 429 is still running.
pub fn rate_limit_wait(retry_after: Option<u64>, failures: u32, floor_active: bool) -> Duration {
    let wait = match retry_after {
        Some(0) => SATURATED_WAIT,
        Some(secs) => Duration::from_secs(secs)
            .min(MAX_RETRY_AFTER)
            .max(MIN_INTERVAL),
        None => failure_backoff(failures),
    };
    apply_saturated_floor(wait, floor_active)
}

/// Raises `wait` to the 360 s floor while the saturated floor is active.
pub fn apply_saturated_floor(wait: Duration, floor_active: bool) -> Duration {
    if floor_active {
        wait.max(SATURATED_FLOOR)
    } else {
        wait
    }
}

/// `Retry-After` in whole seconds. An HTTP date counts as absent.
pub fn parse_retry_after(headers: &[(String, String)]) -> Option<u64> {
    headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("retry-after"))
        .and_then(|(_, value)| value.trim().parse().ok())
}
