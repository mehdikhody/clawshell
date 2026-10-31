//! Pooled remaining: one number per window across the signed-in accounts.

use super::accounts::{Account, Status};
use super::usage::{UsageSnapshot, UsageSource, WindowUsage};
use serde::Serialize;

/// One window (5-hour or weekly) pooled over the accounts that report it.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PooledWindow {
    /// The average of (100 - utilisation); `None` when no account reports the window.
    pub remaining_percent: Option<f64>,
    /// Accounts below 100% used: N in "N of M available".
    pub available: usize,
    /// Accounts counted: M in "N of M available".
    pub total: usize,
    /// The earliest reset among the accounts, as the endpoint wrote it (RFC 3339).
    pub soonest_reset: Option<String>,
    /// When the oldest not-live input was taken (epoch ms); `None` when every
    /// input is live.
    pub stale_since_ms: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PooledRemaining {
    pub five_hour: PooledWindow,
    pub weekly: PooledWindow,
}

/// Pools `accounts`. Signed-out and missing accounts are left out of the
/// average and the count, and so is an account with no reading for the window.
pub fn pool_remaining(accounts: &[Account]) -> PooledRemaining {
    let snapshots: Vec<&UsageSnapshot> = accounts
        .iter()
        .filter(|a| !matches!(a.status, Status::SignedOut | Status::Missing))
        .filter_map(|a| a.usage.as_ref())
        .collect();
    let pool = |window: fn(&UsageSnapshot) -> Option<&WindowUsage>| {
        pool_window(snapshots.iter().filter_map(|s| window(s).map(|w| (w, *s))))
    };
    PooledRemaining {
        five_hour: pool(|s| s.five_hour.as_ref()),
        weekly: pool(|s| s.weekly.as_ref()),
    }
}

fn pool_window<'a>(
    inputs: impl Iterator<Item = (&'a WindowUsage, &'a UsageSnapshot)>,
) -> PooledWindow {
    let inputs: Vec<_> = inputs.collect();
    let used: Vec<f64> = inputs
        .iter()
        .map(|(w, _)| w.utilization.clamp(0.0, 100.0))
        .collect();
    let total = used.len();
    PooledWindow {
        remaining_percent: (total > 0)
            .then(|| used.iter().map(|u| 100.0 - u).sum::<f64>() / total as f64),
        available: used.iter().filter(|u| **u < 100.0).count(),
        total,
        soonest_reset: inputs
            .iter()
            .filter_map(|(w, _)| w.resets_at.as_deref())
            .filter_map(|at| Some((rfc3339_ms(at)?, at)))
            .min_by_key(|(ms, _)| *ms)
            .map(|(_, at)| at.to_string()),
        stale_since_ms: inputs
            .iter()
            .filter(|(_, s)| s.source != UsageSource::Live)
            .map(|(_, s)| s.fetched_at_ms)
            .min(),
    }
}

/// `2026-06-15T15:00:00.000000+00:00` (or `Z`, or no fraction) as epoch ms.
pub(super) fn rfc3339_ms(text: &str) -> Option<i64> {
    let (date, rest) = text.split_once(['T', 't', ' '])?;
    let mut ymd = date.split('-');
    let year: i64 = ymd.next()?.parse().ok()?;
    let month: i64 = ymd.next()?.parse().ok()?;
    let day: i64 = ymd.next()?.parse().ok()?;
    if ymd.next().is_some() || !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    let zone_at = rest.find(['Z', 'z', '+', '-'])?;
    let (clock, zone) = rest.split_at(zone_at);
    let offset_minutes = match zone {
        "Z" | "z" => 0,
        _ => {
            let sign = if zone.starts_with('-') { -1 } else { 1 };
            let (h, m) = zone[1..].split_once(':')?;
            sign * (h.parse::<i64>().ok()? * 60 + m.parse::<i64>().ok()?)
        }
    };
    let (hms, fraction) = clock.split_once('.').unwrap_or((clock, ""));
    let mut parts = hms.split(':');
    let hour: i64 = parts.next()?.parse().ok()?;
    let minute: i64 = parts.next()?.parse().ok()?;
    let second: i64 = parts.next()?.parse().ok()?;
    if parts.next().is_some() || hour > 23 || minute > 59 || second > 60 {
        return None;
    }
    let millis: i64 = format!("{fraction:0<3}")[..3].parse().ok()?;
    let seconds = days_from_civil(year, month, day) * 86_400 + hour * 3_600 + minute * 60 + second
        - offset_minutes * 60;
    Some(seconds * 1_000 + millis)
}

/// Days since 1970-01-01 for a proleptic Gregorian date.
pub(super) fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}
