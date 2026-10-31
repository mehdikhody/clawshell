//! The 30-day heatmap: one cell per local day, in dollars or in percent.
//!
//! Dollars are each day's Code-tab estimate (extra usage has no daily
//! breakdown, so it is left out). Percent is each day's peak weekly
//! utilisation, averaged over the accounts that have a reading that day.
//! A day nobody has data for is `None`, which is not the same as zero.

use super::{DAY_MS, NoEstimate, Prices, Record, civil_date, epoch_ms, price_of, session_owners};
use crate::core::Core;
use crate::core::accounts::Status;
use serde::Serialize;
use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Cells in the heatmap: the last 30 local days, today included.
const WINDOW_DAYS: i64 = 30;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Heatmap {
    /// Oldest first, today last.
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
    pub value: f64,
}

/// Values by `(local day, account id)`.
type ByDay = HashMap<(i64, String), f64>;

impl Core {
    /// The last 30 local days (today last) in both modes, so the UI can
    /// switch without asking again.
    pub fn heatmap(&self) -> Heatmap {
        let accounts: Vec<_> = self
            .known_accounts()
            .unwrap_or_default()
            .into_iter()
            .filter(|a| a.status != Status::Missing)
            .collect();
        let now = self.ports.clock.now();
        let today = self.local_day(now);
        let first = today - (WINDOW_DAYS - 1);
        let in_window = |at: SystemTime| (first..=today).contains(&self.local_day(at));

        let owners = session_owners(&accounts);
        let window_start_ms = first * DAY_MS - self.offset_ms(now);
        let records = self.records_where(&owners, window_start_ms, in_window);

        let (dollars, no_estimate) = if records.is_empty() {
            (None, None)
        } else {
            match self.prices() {
                Some(prices) => (
                    Some(daily_dollars(&records, &prices, |ms| {
                        self.local_day(time_of(ms))
                    })),
                    None,
                ),
                None => (None, Some(NoEstimate::PricesUnavailable)),
            }
        };
        let percent = daily_peaks(&self.weekly_readings(&accounts), first..=today, |ms| {
            self.local_day(time_of(ms))
        });

        let names: Vec<(String, String)> = accounts
            .iter()
            .map(|a| (a.id.clone(), a.label.clone()))
            .collect();
        let days = (first..=today)
            .map(|day| HeatmapDay {
                date: date_of(day),
                usd: dollars
                    .as_ref()
                    .filter(|d| day >= d.first_day)
                    .map(|d| summed(day, &names, &d.by_day)),
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

fn date_of(day: i64) -> String {
    let (year, month, day) = civil_date(day);
    format!("{year:04}-{month:02}-{day:02}")
}

/// Dollars per day and account, plus the first day anything was recorded:
/// from then on a quiet day is a real zero, before it nothing is known.
struct Dollars {
    first_day: i64,
    by_day: ByDay,
}

fn daily_dollars(
    records: &[(String, Record)],
    prices: &Prices,
    day_of: impl Fn(i64) -> i64,
) -> Dollars {
    let mut by_day = ByDay::new();
    let mut first_day = i64::MAX;
    for (account, record) in records {
        let day = day_of(record.at_ms);
        first_day = first_day.min(day);
        if let Some(dollars) = price_of(prices, record) {
            *by_day.entry((day, account.clone())).or_default() += dollars;
        }
    }
    Dollars { first_day, by_day }
}

/// Every account's dollars for `day` (zero when it has none), summed.
fn summed(day: i64, names: &[(String, String)], by_day: &ByDay) -> DayValues {
    let accounts = ranked(names.iter().map(|(id, label)| {
        let value = by_day.get(&(day, id.clone())).copied().unwrap_or(0.0);
        (id, label, value)
    }));
    DayValues {
        total: accounts.iter().map(|a| a.value).sum(),
        accounts,
    }
}

/// The percent of the accounts that have a reading for `day`, averaged;
/// `None` when none do.
fn averaged(day: i64, names: &[(String, String)], by_day: &ByDay) -> Option<DayValues> {
    let accounts = ranked(
        names
            .iter()
            .filter_map(|(id, label)| Some((id, label, *by_day.get(&(day, id.clone()))?))),
    );
    (!accounts.is_empty()).then(|| DayValues {
        total: accounts.iter().map(|a| a.value).sum::<f64>() / accounts.len() as f64,
        accounts,
    })
}

/// Biggest first, ties by label.
fn ranked<'a>(rows: impl Iterator<Item = (&'a String, &'a String, f64)>) -> Vec<DayAccount> {
    let mut accounts: Vec<DayAccount> = rows
        .map(|(id, label, value)| DayAccount {
            id: id.clone(),
            label: label.clone(),
            value,
        })
        .collect();
    accounts.sort_by(|a, b| {
        b.value
            .total_cmp(&a.value)
            .then_with(|| a.label.cmp(&b.label))
    });
    accounts
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
