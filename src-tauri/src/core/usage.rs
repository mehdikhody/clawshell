//! Live usage per account: email from the OAuth profile endpoint, 5-hour and
//! weekly utilisation from the OAuth usage endpoint, and the polling that
//! keeps them current.
//!
//! The access token is read in memory for each request and sent only to the
//! two Anthropic endpoints below. It is never refreshed, stored or logged,
//! and neither are emails: nothing in here writes either to an error message.

use super::Core;
use super::accounts::Status;
use super::copy_target::ProfileIdentity;
use super::polling::{
    MIN_INTERVAL, SATURATED_FLOOR_FOR, adaptive_interval, apply_saturated_floor, failure_backoff,
    jitter_unit, next_poll_delay, parse_retry_after, rate_limit_wait,
};
use super::ports::{HttpRequest, HttpResponse};
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::time::{Duration, UNIX_EPOCH};

pub const PROFILE_URL: &str = "https://api.anthropic.com/api/oauth/profile";
pub const USAGE_URL: &str = "https://api.anthropic.com/api/oauth/usage";
const OAUTH_BETA: &str = "oauth-2025-04-20";
const HISTORY_FILE: &str = "plan-usage-history.json";
/// How long this session's polls are remembered for the heatmap.
const POLL_MEMORY_MS: i64 = 31 * 86_400_000;

/// Where a snapshot's numbers came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum UsageSource {
    /// Fetched from the usage endpoint on the last successful poll.
    Live,
    /// The last live snapshot, kept because the latest poll failed.
    Stale,
    /// Read from the profile's `plan-usage-history.json` (no reset times).
    LocalHistory,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowUsage {
    /// Percent of the window used, 0 to 100.
    pub utilization: f64,
    /// When the window resets, as the endpoint reported it (RFC 3339).
    pub resets_at: Option<String>,
}

/// Extra-usage (overage) spend, in US cents.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtraUsage {
    pub enabled: bool,
    pub used_minor: Option<i64>,
    pub limit_minor: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageSnapshot {
    pub five_hour: Option<WindowUsage>,
    pub weekly: Option<WindowUsage>,
    pub extra_usage: Option<ExtraUsage>,
    /// When the numbers were taken, epoch milliseconds.
    pub fetched_at_ms: i64,
    pub source: UsageSource,
}

impl UsageSnapshot {
    pub fn max_utilization(&self) -> f64 {
        [&self.five_hour, &self.weekly]
            .into_iter()
            .flatten()
            .map(|w| w.utilization)
            .fold(0.0, f64::max)
    }

    /// Either window at or above 100%.
    pub fn is_limited(&self) -> bool {
        self.max_utilization() >= super::polling::EXHAUSTED_PERCENT
    }
}

/// What a refresh did.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RefreshOutcome {
    /// Accounts asked for fresh numbers (successfully or not).
    pub refreshed: usize,
    /// Epoch ms before which a manual refresh is refused; `None` when one is
    /// allowed now.
    pub refresh_allowed_at: Option<i64>,
}

/// Per-account polling state, kept in memory only.
#[derive(Default)]
pub(super) struct AccountUsage {
    pub email: Option<String>,
    /// The account and organisation uuids the profile endpoint reported.
    pub ids: Option<ProfileIdentity>,
    pub snapshot: Option<UsageSnapshot>,
    last_attempt_ms: Option<i64>,
    /// A 429 told us to stay away until then (manual refresh obeys it too).
    blocked_until_ms: Option<i64>,
    next_due_ms: i64,
    failures: u32,
    saturated_since_ms: Option<i64>,
    /// Weekly readings from this session's successful polls, `(epoch ms, %)`,
    /// kept for the heatmap. Only the last `POLL_MEMORY_MS` is held.
    weekly_polls: Vec<(i64, f64)>,
}

impl AccountUsage {
    /// Earliest moment a manual refresh may go ahead.
    fn allowed_at_ms(&self) -> i64 {
        let spaced = self
            .last_attempt_ms
            .map_or(i64::MIN, |at| at.saturating_add(ms(MIN_INTERVAL)));
        spaced.max(self.blocked_until_ms.unwrap_or(i64::MIN))
    }

    fn record_weekly(&mut self, at_ms: i64, percent: f64) {
        self.weekly_polls.push((at_ms, percent));
        self.weekly_polls
            .retain(|(at, _)| at_ms.saturating_sub(*at) <= POLL_MEMORY_MS);
    }

    fn saturated_floor_active(&self, now_ms: i64) -> bool {
        self.saturated_since_ms
            .is_some_and(|since| now_ms - since < ms(SATURATED_FLOOR_FOR))
    }
}

#[derive(Default)]
pub(super) struct UsageState {
    pub accounts: HashMap<String, AccountUsage>,
}

/// One poll's result, before it is applied to the account's state.
enum Fetched {
    Live {
        snapshot: UsageSnapshot,
        /// Only asked for when the account has no email yet.
        profile: Option<ProfileAnswer>,
    },
    /// The token is expired (or unreadable): no request was made.
    NoUsableToken,
    /// The endpoint answered 401.
    Unauthorized,
    RateLimited {
        retry_after: Option<u64>,
    },
    Failed,
}

/// What the profile endpoint said about the signed-in account.
struct ProfileAnswer {
    email: Option<String>,
    ids: Option<ProfileIdentity>,
}

fn ms(d: Duration) -> i64 {
    i64::try_from(d.as_millis()).unwrap_or(i64::MAX)
}

impl Core {
    fn now_ms(&self) -> i64 {
        self.ports
            .clock
            .now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
    }

    /// Fetches usage for the accounts that are due. `force` is the manual
    /// refresh: every signed-in account whose minimum interval has passed is
    /// asked, due or not. Anything else waits for its own schedule.
    pub fn refresh_usage(&self, force: bool) -> RefreshOutcome {
        let _one_at_a_time = self.refresh_gate.lock().unwrap();
        let Ok(accounts) = self.known_accounts() else {
            return RefreshOutcome {
                refreshed: 0,
                refresh_allowed_at: None,
            };
        };
        let mut refreshed = 0;
        for account in &accounts {
            if matches!(account.status, Status::Missing | Status::SignedOut) {
                continue;
            }
            let now = self.now_ms();
            if !self.is_due(&account.id, now, force) {
                continue;
            }
            self.refresh_account(&account.id, &account.path, account.running, now);
            self.notify_after_poll(&account.id, &account.label);
            refreshed += 1;
        }
        let present: Vec<&str> = accounts.iter().map(|a| a.id.as_str()).collect();
        self.usage
            .lock()
            .unwrap()
            .accounts
            .retain(|id, _| present.contains(&id.as_str()));
        RefreshOutcome {
            refreshed,
            refresh_allowed_at: self.refresh_allowed_at(&accounts),
        }
    }

    /// Hands the account's fresh snapshot to the notifications.
    fn notify_after_poll(&self, id: &str, label: &str) {
        let snapshot = self
            .usage
            .lock()
            .unwrap()
            .accounts
            .get(id)
            .and_then(|s| s.snapshot.clone());
        if let Some(snapshot) = snapshot {
            self.notify_usage(id, label, &snapshot);
        }
    }

    fn is_due(&self, id: &str, now: i64, force: bool) -> bool {
        let state = self.usage.lock().unwrap();
        match state.accounts.get(id) {
            None => true,
            Some(s) if force => now >= s.allowed_at_ms(),
            Some(s) => now >= s.next_due_ms,
        }
    }

    /// When a manual refresh is next allowed, or `None` if it is allowed now
    /// (or there is nothing to refresh).
    pub(super) fn refresh_allowed_at(&self, accounts: &[super::Account]) -> Option<i64> {
        let now = self.now_ms();
        let state = self.usage.lock().unwrap();
        let mut earliest: Option<i64> = None;
        for account in accounts {
            if matches!(account.status, Status::Missing | Status::SignedOut) {
                continue;
            }
            let at = state
                .accounts
                .get(&account.id)
                .map_or(i64::MIN, AccountUsage::allowed_at_ms);
            if at <= now {
                return None;
            }
            earliest = Some(earliest.map_or(at, |e| e.min(at)));
        }
        earliest
    }

    fn refresh_account(&self, id: &str, path: &Path, running: bool, now: i64) {
        let history = read_history(path);
        let email_known = self
            .usage
            .lock()
            .unwrap()
            .accounts
            .get(id)
            .is_some_and(|s| s.email.is_some());
        // The profile endpoint is only asked until the email is known.
        let fetched = self.fetch(path, now, !email_known);

        let mut state = self.usage.lock().unwrap();
        let s = state.accounts.entry(id.to_string()).or_default();
        s.last_attempt_ms = Some(now);
        let floor_active = s.saturated_floor_active(now);
        let wait = match fetched {
            Fetched::Live { snapshot, profile } => {
                if let Some(profile) = profile {
                    if profile.email.is_some() {
                        s.email = profile.email;
                    }
                    if profile.ids.is_some() {
                        s.ids = profile.ids;
                    }
                }
                s.failures = 0;
                s.blocked_until_ms = None;
                let seed = jitter_seed(id, now);
                let interval = adaptive_interval(snapshot.max_utilization(), running);
                if let Some(w) = &snapshot.weekly {
                    s.record_weekly(now, w.utilization);
                }
                s.snapshot = Some(snapshot);
                apply_saturated_floor(next_poll_delay(interval, jitter_unit(seed)), floor_active)
            }
            Fetched::NoUsableToken => {
                // Nothing was requested, so this costs no budget: look again
                // soon in case Claude Desktop has renewed the token.
                s.snapshot = fallback(s.snapshot.take(), history);
                apply_saturated_floor(MIN_INTERVAL, floor_active)
            }
            Fetched::Unauthorized | Fetched::Failed => {
                s.failures += 1;
                s.snapshot = fallback(s.snapshot.take(), history);
                apply_saturated_floor(failure_backoff(s.failures), floor_active)
            }
            Fetched::RateLimited { retry_after } => {
                s.failures += 1;
                s.snapshot = fallback(s.snapshot.take(), history);
                let wait = rate_limit_wait(retry_after, s.failures, floor_active);
                if retry_after == Some(0) && !floor_active {
                    s.saturated_since_ms = Some(now);
                }
                s.blocked_until_ms = Some(now.saturating_add(ms(wait)));
                wait
            }
        };
        s.next_due_ms = now.saturating_add(ms(wait));
    }

    fn fetch(&self, path: &Path, now: i64, needs_profile: bool) -> Fetched {
        let Some(token) = self.ports.tokens.read_token(path) else {
            return Fetched::NoUsableToken;
        };
        if token.expires_at_ms.is_some_and(|expires| expires <= now) {
            return Fetched::NoUsableToken;
        }
        let response = self
            .ports
            .http
            .send(authorized(USAGE_URL, &token.access_token, true));
        match response {
            Err(_) => Fetched::Failed,
            Ok(HttpResponse {
                status: 200, body, ..
            }) => match parse_usage(&body, now) {
                Some(snapshot) => Fetched::Live {
                    snapshot,
                    profile: needs_profile
                        .then(|| self.fetch_profile(&token.access_token))
                        .flatten(),
                },
                None => Fetched::Failed,
            },
            Ok(r) if r.status == 401 => Fetched::Unauthorized,
            Ok(r) if r.status == 429 => Fetched::RateLimited {
                retry_after: parse_retry_after(&r.headers),
            },
            Ok(_) => Fetched::Failed,
        }
    }

    fn fetch_profile(&self, access_token: &str) -> Option<ProfileAnswer> {
        let response = self
            .ports
            .http
            .send(authorized(PROFILE_URL, access_token, false))
            .ok()?;
        if response.status != 200 {
            return None;
        }
        let body: Value = serde_json::from_str(&response.body).ok()?;
        let text = |pointer: &str| body.pointer(pointer).and_then(Value::as_str);
        Some(ProfileAnswer {
            email: text("/account/email")
                .filter(|e| !e.is_empty())
                .map(str::to_string),
            ids: text("/account/uuid")
                .zip(text("/organization/uuid"))
                .and_then(|(account, organization)| ProfileIdentity::parse(account, organization)),
        })
    }
}

fn authorized(url: &str, token: &str, beta: bool) -> HttpRequest {
    let mut headers = vec![
        ("Authorization".to_string(), format!("Bearer {token}")),
        ("Accept".to_string(), "application/json".to_string()),
    ];
    if beta {
        headers.push(("anthropic-beta".to_string(), OAUTH_BETA.to_string()));
    }
    HttpRequest {
        url: url.to_string(),
        headers,
    }
}

fn jitter_seed(id: &str, now_ms: i64) -> u64 {
    let mut hasher = DefaultHasher::new();
    id.hash(&mut hasher);
    now_ms.hash(&mut hasher);
    hasher.finish()
}

/// Reads the usage endpoint's body. Every field is optional; a body with
/// neither window is not a usage reading.
pub fn parse_usage(body: &str, now_ms: i64) -> Option<UsageSnapshot> {
    let json: Value = serde_json::from_str(body).ok()?;
    let five_hour = parse_window(json.get("five_hour"));
    let weekly = parse_window(json.get("seven_day"));
    if five_hour.is_none() && weekly.is_none() {
        return None;
    }
    Some(UsageSnapshot {
        five_hour,
        weekly,
        extra_usage: parse_extra_usage(&json),
        fetched_at_ms: now_ms,
        source: UsageSource::Live,
    })
}

fn parse_window(value: Option<&Value>) -> Option<WindowUsage> {
    let value = value?;
    Some(WindowUsage {
        utilization: value.get("utilization")?.as_f64()?,
        resets_at: value
            .get("resets_at")
            .and_then(Value::as_str)
            .map(str::to_string),
    })
}

fn parse_extra_usage(json: &Value) -> Option<ExtraUsage> {
    let extra = json.get("extra_usage")?.as_object()?;
    let minor = |v: Option<&Value>| v.and_then(Value::as_f64).map(|n| n.round() as i64);
    let used_minor = minor(extra.get("used_credits")).or_else(|| spend_used_cents(json));
    Some(ExtraUsage {
        enabled: extra
            .get("is_enabled")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        used_minor,
        limit_minor: minor(extra.get("monthly_limit")),
    })
}

/// `spend.used` as US cents. Its amount is in minor units of `exponent`
/// decimal places (2 when absent); anything but USD is not counted.
fn spend_used_cents(json: &Value) -> Option<i64> {
    let used = json.pointer("/spend/used")?;
    if used.get("currency")?.as_str()? != "USD" {
        return None;
    }
    let amount = used.get("amount_minor")?.as_f64()?;
    let exponent = used.get("exponent").and_then(Value::as_i64).unwrap_or(2);
    let exponent = i32::try_from(exponent).ok()?;
    Some((amount * 100.0 / 10f64.powi(exponent)).round() as i64)
}

/// One reading from the profile's `plan-usage-history.json`.
struct HistorySample {
    at_ms: i64,
    five_hour: f64,
    weekly: f64,
}

/// The newest sample of `{"samples":[{"t": epoch ms, "u": {"fh": %, "sd": %}}]}`.
fn read_history(profile: &Path) -> Option<HistorySample> {
    let text = std::fs::read_to_string(profile.join(HISTORY_FILE)).ok()?;
    let json: Value = serde_json::from_str(&text).ok()?;
    json.get("samples")?
        .as_array()?
        .iter()
        .filter_map(|sample| {
            Some(HistorySample {
                at_ms: sample.get("t")?.as_i64()?,
                five_hour: sample.pointer("/u/fh")?.as_f64()?,
                weekly: sample.pointer("/u/sd")?.as_f64()?,
            })
        })
        .max_by_key(|s| s.at_ms)
}

/// What to show after a failed poll. The last snapshot stays, marked stale,
/// unless Claude's own history has a newer reading (it has no reset times).
fn fallback(
    previous: Option<UsageSnapshot>,
    history: Option<HistorySample>,
) -> Option<UsageSnapshot> {
    let from_history = |h: HistorySample| UsageSnapshot {
        five_hour: Some(WindowUsage {
            utilization: h.five_hour,
            resets_at: None,
        }),
        weekly: Some(WindowUsage {
            utilization: h.weekly,
            resets_at: None,
        }),
        extra_usage: None,
        fetched_at_ms: h.at_ms,
        source: UsageSource::LocalHistory,
    };
    match (previous, history) {
        (Some(prev), Some(h)) if h.at_ms > prev.fetched_at_ms => Some(from_history(h)),
        (Some(prev), _) if prev.source == UsageSource::LocalHistory => Some(prev),
        (Some(prev), _) => Some(UsageSnapshot {
            source: UsageSource::Stale,
            ..prev
        }),
        (None, Some(h)) => Some(from_history(h)),
        (None, None) => None,
    }
}

impl Core {
    /// Every weekly reading we have per account, `(epoch ms, %)`: Claude's own
    /// `plan-usage-history.json` plus this session's polls. Unreadable files
    /// contribute nothing.
    pub(super) fn weekly_readings(
        &self,
        accounts: &[super::Account],
    ) -> HashMap<String, Vec<(i64, f64)>> {
        let state = self.usage.lock().unwrap();
        accounts
            .iter()
            .map(|a| {
                let mut readings = read_weekly_history(&a.path);
                if let Some(s) = state.accounts.get(&a.id) {
                    readings.extend_from_slice(&s.weekly_polls);
                }
                (a.id.clone(), readings)
            })
            .collect()
    }
}

/// Every weekly sample of `plan-usage-history.json`, `(epoch ms, %)`.
fn read_weekly_history(profile: &Path) -> Vec<(i64, f64)> {
    let Ok(text) = std::fs::read_to_string(profile.join(HISTORY_FILE)) else {
        return Vec::new();
    };
    let Ok(json) = serde_json::from_str::<Value>(&text) else {
        return Vec::new();
    };
    json.get("samples")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|sample| {
            Some((
                sample.get("t")?.as_i64()?,
                sample.pointer("/u/sd")?.as_f64()?,
            ))
        })
        .collect()
}
