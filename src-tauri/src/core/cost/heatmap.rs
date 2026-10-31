//! The 9-week heatmap: one cell per local day, in dollars, with the day's
//! tokens alongside. Dollars are each day's Code-tab estimate (extra usage has
//! no daily breakdown, so it is left out). Percent is each day's peak weekly
//! utilisation, averaged over the accounts that have a reading that day.
//! A day nobody has data for is `None`, which is not the same as zero.

use super::{DAY_MS, NoEstimate, Record, civil_date, epoch_ms, price_of, session_owners};
use crate::core::Core;
use crate::core::accounts::Status;
use serde::Serialize;
use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Week columns in the heatmap, this week included. Weeks start on Sunday.
const WINDOW_WEEKS: i64 = 9;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Heatmap {
    /// Oldest first, starting on a Sunday, today last.
    pub days: Vec<HeatmapDay>,
    /// Why every dollar cell is empty, when prices are the reason.
    pub no_estimate: Option<NoEstimate>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HeatmapDay {
    /// The local calendar day, `YYYY-MM-DD`.
    pub date: String,
    /// Code-tab dollars; `None` when nothing is known about the day.
    pub usd: Option<DayValues>,
    /// Every token the day's messages used, and each account's part; `None`
    /// when nothing is known about the day. Needs no prices.
    pub tokens: Option<DayValues>,
    /// Peak weekly percent; `None` when no account has a reading that day.
    pub percent: Option<DayValues>,
}

/// A day's total and what each account contributed, biggest first.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DayValues {
    pub total: f64,
    pub accounts: Vec<DayAccount>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DayAccount {
    pub id: String,
    pub label: String,
    /// The account's badge colour, `#rrggbb`.
    pub colour: String,
    pub value: f64,
}

/// Values by `(local day, account id)`.
type ByDay = HashMap<(i64, String), f64>;

/// What a day's breakdown shows of each account.
struct Name {
    id: String,
    label: String,
    colour: String,
}

impl Core {
    /// The local days from the Sunday 8 weeks before this week's to today,
    /// with dollars, tokens and percent.
    pub fn heatmap(&self) -> Heatmap {
        let accounts: Vec<_> = self
            .known_accounts()
            .unwrap_or_default()
            .into_iter()
            .filter(|a| a.status != Status::Missing)
            .collect();
        let now = self.ports.clock.now();
        let today = self.local_day(now);
        let first = sunday_of(today) - (WINDOW_WEEKS - 1) * 7;
        let in_window = |at: SystemTime| (first..=today).contains(&self.local_day(at));

        let owners = session_owners(&accounts);
        let window_start_ms = first * DAY_MS - self.offset_ms(now);
        let records = self.records_where(&owners, window_start_ms, in_window);

        let (dollars, no_estimate) = if records.is_empty() {
            (None, None)
        } else {
            match self.prices() {
                Some(prices) => (
                    Some(daily(
                        &records,
                        |ms| self.local_day(time_of(ms)),
                        |record| price_of(&prices, record),
                    )),
                    None,
                ),
                None => (None, Some(NoEstimate::PricesUnavailable)),
            }
        };
        let tokens = (!records.is_empty()).then(|| {
            daily(
                &records,
                |ms| self.local_day(time_of(ms)),
                |record| Some(record.usage.total() as f64),
            )
        });
        let percent = daily_peaks(&self.weekly_readings(&accounts), first..=today, |ms| {
            self.local_day(time_of(ms))
        });

        let names: Vec<Name> = accounts
            .iter()
            .map(|a| Name {
                id: a.id.clone(),
                label: a.label.clone(),
                colour: a.colour.clone(),
            })
            .collect();
        let days = (first..=today)
            .map(|day| HeatmapDay {
                date: date_of(day),
                usd: dollars
                    .as_ref()
                    .filter(|d| day >= d.first_day)
                    .map(|d| summed(day, &names, &d.by_day)),
                tokens: tokens
                    .as_ref()
                    .filter(|t| day >= t.first_day)
                    .map(|t| summed(day, &names, &t.by_day)),
                percent: averaged(day, &names, &percent),
            })
            .collect();
        Heatmap { days, no_estimate }
    }

    /// The local calendar day of `at`, as days since 1970-01-01.
    fn local_day(&self, at: SystemTime) -> i64 {
        (epoch_ms(at) + self.offset_ms(at)).div_euclid(DAY_MS)
    }

    /// How far the local time zone is ahead of UTC at `at`, in milliseconds.
    fn offset_ms(&self, at: SystemTime) -> i64 {
        i64::from(self.ports.clock.utc_offset_secs(at)) * 1_000
    }
}

fn time_of(ms: i64) -> SystemTime {
    UNIX_EPOCH + Duration::from_millis(u64::try_from(ms).unwrap_or(0))
}

/// The Sunday on or before `day`; day 0, 1970-01-01, was a Thursday.
fn sunday_of(day: i64) -> i64 {
    day - (day + 4).rem_euclid(7)
}

fn date_of(day: i64) -> String {
    let (year, month, day) = civil_date(day);
    format!("{year:04}-{month:02}-{day:02}")
}

/// Dollars or tokens per day and account, plus the first day anything was
/// recorded: from then on a quiet day is a real zero, before it nothing is known.
struct Daily {
    first_day: i64,
    by_day: ByDay,
}

/// Sums `value_of` each record by local day and account; a record it has no
/// value for (an unpriced model) still counts towards `first_day`.
fn daily(
    records: &[(String, Record)],
    day_of: impl Fn(i64) -> i64,
    value_of: impl Fn(&Record) -> Option<f64>,
) -> Daily {
    let mut by_day = ByDay::new();
    let mut first_day = i64::MAX;
    for (account, record) in records {
        let day = day_of(record.at_ms);
        first_day = first_day.min(day);
        if let Some(value) = value_of(record) {
            *by_day.entry((day, account.clone())).or_default() += value;
        }
    }
    Daily { first_day, by_day }
}

/// Every account's dollars or tokens for `day` (zero when it has none), summed.
fn summed(day: i64, names: &[Name], by_day: &ByDay) -> DayValues {
    let accounts = ranked(names.iter().map(|name| {
        let value = by_day.get(&(day, name.id.clone())).copied().unwrap_or(0.0);
        (name, value)
    }));
    DayValues {
        total: accounts.iter().map(|a| a.value).sum(),
        accounts,
    }
}

/// The percent of the accounts that have a reading for `day`, averaged;
/// `None` when none do.
fn averaged(day: i64, names: &[Name], by_day: &ByDay) -> Option<DayValues> {
    let accounts = ranked(
        names
            .iter()
            .filter_map(|name| Some((name, *by_day.get(&(day, name.id.clone()))?))),
    );
    (!accounts.is_empty()).then(|| DayValues {
        total: accounts.iter().map(|a| a.value).sum::<f64>() / accounts.len() as f64,
        accounts,
    })
}

/// In the order the rows come in: the saved account order.
fn ranked<'a>(rows: impl Iterator<Item = (&'a Name, f64)>) -> Vec<DayAccount> {
    rows.map(|(name, value)| DayAccount {
        id: name.id.clone(),
        label: name.label.clone(),
        colour: name.colour.clone(),
        value,
    })
    .collect()
}

/// Each account's highest reading per local day, for the days in `window`.
fn daily_peaks(
    readings: &HashMap<String, Vec<(i64, f64)>>,
    window: std::ops::RangeInclusive<i64>,
    day_of: impl Fn(i64) -> i64,
) -> ByDay {
    let mut peaks = ByDay::new();
    for (account, samples) in readings {
        for &(at_ms, percent) in samples {
            let day = day_of(at_ms);
            if window.contains(&day) {
                let peak = peaks.entry((day, account.clone())).or_insert(percent);
                *peak = peak.max(percent);
            }
        }
    }
    peaks
}
