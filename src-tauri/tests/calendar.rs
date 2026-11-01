use clawshell_lib::core::Language;
use clawshell_lib::core::calendar::{
    Calendar, CalendarSetting, Digits, digits_of, resolve_calendar, week_start_of,
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
