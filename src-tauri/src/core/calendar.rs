//! The calendar setting and what it decides: the digits of every number and
//! the first day of the week. Mirrors the window's `locale.ts`, so both sides
//! agree on what the user chose.

use super::Language;
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
