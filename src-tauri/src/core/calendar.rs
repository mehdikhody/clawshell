//! The calendar setting and what it decides: the digits of every number and
//! the first day of the week. Mirrors the window's `locale.ts`, so both sides
//! agree on what the user chose.

use super::Language;
use icu_calendar::Date;
use icu_calendar::cal::{ChineseTraditional, Gregorian, Hijri, Persian};
use icu_calendar::types::RataDie;
use serde::{Deserialize, Serialize};

/// What the user picked, saved with the `Intl` calendar ids. Automatic follows
/// the language.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CalendarSetting {
    #[default]
    Automatic,
    Gregory,
    Persian,
    IslamicUmalqura,
    Chinese,
}

/// A calendar in use: a setting with Automatic resolved away.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Calendar {
    Gregory,
    Persian,
    IslamicUmalqura,
    Chinese,
}

/// The numbering systems the calendars write numbers in, by their `Intl` ids.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Digits {
    /// 0-9.
    Latn,
    /// Arabic-Indic.
    Arab,
    /// Persian.
    Arabext,
}

/// The calendar in use: Automatic gives Jalali for Persian, Hijri for Arabic
/// and Gregorian otherwise; a pinned one stays.
pub fn resolve_calendar(language: Language, setting: CalendarSetting) -> Calendar {
    match setting {
        CalendarSetting::Gregory => Calendar::Gregory,
        CalendarSetting::Persian => Calendar::Persian,
        CalendarSetting::IslamicUmalqura => Calendar::IslamicUmalqura,
        CalendarSetting::Chinese => Calendar::Chinese,
        CalendarSetting::Automatic => match language {
            Language::Fa => Calendar::Persian,
            Language::Ar => Calendar::IslamicUmalqura,
            Language::En | Language::Ru | Language::Zh => Calendar::Gregory,
        },
    }
}

/// Persian digits for Jalali, Arabic-Indic for Hijri, 0-9 for Gregorian and
/// Chinese lunar.
pub fn digits_of(calendar: Calendar) -> Digits {
    match calendar {
        Calendar::Persian => Digits::Arabext,
        Calendar::IslamicUmalqura => Digits::Arab,
        Calendar::Gregory | Calendar::Chinese => Digits::Latn,
    }
}

/// `text` with its 0-9 written in `digits`; everything else stays as it is.
pub fn localize_digits(text: &str, digits: Digits) -> String {
    let zero = match digits {
        Digits::Latn => return text.to_string(),
        Digits::Arab => '\u{0660}',
        Digits::Arabext => '\u{06F0}',
    };
    text.chars()
        .map(|c| match c.to_digit(10) {
            Some(n) => char::from_u32(zero as u32 + n).unwrap_or(c),
            _ => c,
        })
        .collect()
}

/// The day a week starts on, counted as JavaScript's `Date.getDay` counts
/// (Sunday is 0). Jalali and Hijri start on Saturday and Chinese lunar on
/// Monday; Gregorian takes it from the language.
pub fn week_start_of(language: Language, setting: CalendarSetting) -> u8 {
    match resolve_calendar(language, setting) {
        Calendar::Persian | Calendar::IslamicUmalqura => 6,
        Calendar::Chinese => 1,
        Calendar::Gregory => match language {
            Language::En => 0,
            Language::Ru | Language::Zh => 1,
            Language::Fa | Language::Ar => 6,
        },
    }
}

/// Days from 0001-01-01 (Rata Die day 1) to 1970-01-01.
const UNIX_EPOCH_RATA_DIE: i64 = 719_163;

/// The month of `calendar` that holds `day` (days since 1970-01-01): its first
/// day, and the first day of the next month.
pub fn month_span(calendar: Calendar, day: i64) -> (i64, i64) {
    match calendar {
        Calendar::Gregory => span_in(Gregorian, day),
        Calendar::Persian => span_in(Persian::new(), day),
        Calendar::IslamicUmalqura => span_in(Hijri::new_umm_al_qura(), day),
        Calendar::Chinese => span_in(ChineseTraditional::new(), day),
    }
}

fn span_in<C: icu_calendar::Calendar>(calendar: C, day: i64) -> (i64, i64) {
    let date = Date::from_rata_die(RataDie::new(day + UNIX_EPOCH_RATA_DIE), calendar);
    let first = day - i64::from(date.day_of_month().0 - 1);
    (first, first + i64::from(date.days_in_month()))
}
