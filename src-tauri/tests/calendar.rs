use clawshell_lib::core::Language;
use clawshell_lib::core::calendar::{
    Calendar, CalendarSetting, Digits, digits_of, month_span, resolve_calendar, week_start_of,
};

const LANGUAGES: [Language; 5] = [
    Language::En,
    Language::Fa,
    Language::Ar,
    Language::Ru,
    Language::Zh,
];

const PINNED: [(CalendarSetting, Calendar); 4] = [
    (CalendarSetting::Gregory, Calendar::Gregory),
    (CalendarSetting::Persian, Calendar::Persian),
    (CalendarSetting::IslamicUmalqura, Calendar::IslamicUmalqura),
    (CalendarSetting::Chinese, Calendar::Chinese),
];

#[test]
fn automatic_follows_the_language() {
    for (language, calendar) in [
        (Language::Fa, Calendar::Persian),
        (Language::Ar, Calendar::IslamicUmalqura),
        (Language::En, Calendar::Gregory),
        (Language::Ru, Calendar::Gregory),
        (Language::Zh, Calendar::Gregory),
    ] {
        assert_eq!(
            resolve_calendar(language, CalendarSetting::Automatic),
            calendar,
            "{language:?}"
        );
    }
}

#[test]
fn a_pinned_calendar_stays_whatever_the_language() {
    for language in LANGUAGES {
        for (setting, calendar) in PINNED {
            assert_eq!(resolve_calendar(language, setting), calendar);
        }
    }
}

#[test]
fn digits_follow_the_calendar() {
    assert_eq!(digits_of(Calendar::Persian), Digits::Arabext);
    assert_eq!(digits_of(Calendar::IslamicUmalqura), Digits::Arab);
    assert_eq!(digits_of(Calendar::Gregory), Digits::Latn);
    assert_eq!(digits_of(Calendar::Chinese), Digits::Latn);
}

#[test]
fn jalali_and_hijri_weeks_start_on_saturday_and_lunar_on_monday_in_any_language() {
    for language in LANGUAGES {
        assert_eq!(week_start_of(language, CalendarSetting::Persian), 6);
        assert_eq!(week_start_of(language, CalendarSetting::IslamicUmalqura), 6);
        assert_eq!(week_start_of(language, CalendarSetting::Chinese), 1);
    }
}

#[test]
fn gregorian_weeks_start_on_the_day_of_the_language() {
    for (language, day) in [
        (Language::En, 0),
        (Language::Ru, 1),
        (Language::Zh, 1),
        (Language::Fa, 6),
        (Language::Ar, 6),
    ] {
        assert_eq!(
            week_start_of(language, CalendarSetting::Gregory),
            day,
            "{language:?}"
        );
    }
}

#[test]
fn automatic_weeks_start_where_the_resolved_calendar_says() {
    for (language, day) in [
        (Language::En, 0),
        (Language::Ru, 1),
        (Language::Zh, 1),
        (Language::Fa, 6),
        (Language::Ar, 6),
    ] {
        assert_eq!(
            week_start_of(language, CalendarSetting::Automatic),
            day,
            "{language:?}"
        );
    }
}

/// Days since 1970-01-01 of a Gregorian date.
fn day(year: i32, month: u32, date: u32) -> i64 {
    let epoch = chrono::NaiveDate::from_ymd_opt(1970, 1, 1).unwrap();
    let at = chrono::NaiveDate::from_ymd_opt(year, month, date).unwrap();
    at.signed_duration_since(epoch).num_days()
}

// The expected boundaries below are the ones the platform's `Intl` gives for
// these calendars.

#[test]
fn a_gregorian_month_runs_from_the_first_to_the_first() {
    assert_eq!(
        month_span(Calendar::Gregory, day(2026, 6, 15)),
        (day(2026, 6, 1), day(2026, 7, 1))
    );
    assert_eq!(
        month_span(Calendar::Gregory, day(2026, 2, 28)),
        (day(2026, 2, 1), day(2026, 3, 1))
    );
}

#[test]
fn a_jalali_month_runs_from_its_first_day_to_the_next_months() {
    // 25 Khordad 1405: Khordad has 31 days.
    assert_eq!(
        month_span(Calendar::Persian, day(2026, 6, 15)),
        (day(2026, 5, 22), day(2026, 6, 22))
    );
    // 1 Tir 1405 opens a new month.
    assert_eq!(
        month_span(Calendar::Persian, day(2026, 6, 22)).0,
        day(2026, 6, 22)
    );
    assert_eq!(
        month_span(Calendar::Persian, day(2026, 6, 21)).1,
        day(2026, 6, 22)
    );
}

#[test]
fn a_hijri_month_edge_falls_between_the_29th_or_30th_and_the_first() {
    // Dhu al-Hijjah 1447 ends on 15 June 2026; Muharram 1448 starts on the 16th.
    assert_eq!(
        month_span(Calendar::IslamicUmalqura, day(2026, 6, 15)),
        (day(2026, 5, 18), day(2026, 6, 16))
    );
    assert_eq!(
        month_span(Calendar::IslamicUmalqura, day(2026, 6, 16)),
        (day(2026, 6, 16), day(2026, 7, 15))
    );
}

#[test]
fn a_chinese_leap_month_is_a_month_of_its_own() {
    // 2025 had a leap sixth month, from 25 July to 22 August.
    assert_eq!(
        month_span(Calendar::Chinese, day(2025, 8, 1)),
        (day(2025, 7, 25), day(2025, 8, 23))
    );
    assert_eq!(
        month_span(Calendar::Chinese, day(2025, 7, 24)),
        (day(2025, 6, 25), day(2025, 7, 25))
    );
    assert_eq!(
        month_span(Calendar::Chinese, day(2025, 8, 23)),
        (day(2025, 8, 23), day(2025, 9, 22))
    );
}
