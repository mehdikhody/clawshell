//! Limit notifications, through the core: given this sequence of usage
//! responses over time, which Windows notifications go out?
//!
//! Tokens and emails in these fixtures are synthetic.

mod support;

use clawshell_lib::core::usage::{PROFILE_URL, USAGE_URL};
use clawshell_lib::core::{CalendarSetting, Core, Language, Settings, SettingsError};
use std::time::Duration;
use support::Harness;

const ID: &str = "3f2b8c1e-5a4d-4e7b-9c10-2d6f8a9b0c11";
const TOKEN: &str = "sk-test-token-aaaa";
const EMAIL: &str = "private.person@example.com";
const NOW_MS: i64 = 1_781_524_800_000;
const DAY_MS: i64 = 86_400_000;

const FIVE_RESET_1: &str = "2026-06-15T15:00:00.000000+00:00";
const FIVE_RESET_2: &str = "2026-06-15T20:00:00.000000+00:00";
const WEEK_RESET_1: &str = "2026-06-19T09:30:00.000000+00:00";
const WEEK_RESET_2: &str = "2026-06-26T09:30:00.000000+00:00";

fn signed_in(h: &Harness) {
    h.fixture.add_profile(ID);
    h.fixture.sign_in(ID);
    h.tokens.set(
        &h.fixture.profile_path(ID),
        TOKEN,
        Some(NOW_MS + 30 * DAY_MS),
    );
}

/// Queues one poll's answer: usage with these windows, and the profile.
fn script(h: &Harness, five: (f64, &str), week: (f64, &str)) {
    h.http.respond(
        USAGE_URL,
        200,
        &format!(
            r#"{{"five_hour":{{"utilization":{},"resets_at":"{}"}},
                "seven_day":{{"utilization":{},"resets_at":"{}"}}}}"#,
            five.0, five.1, week.0, week.1
        ),
    );
    h.http.respond(
        PROFILE_URL,
        200,
        &format!(r#"{{"account":{{"email":"{EMAIL}"}}}}"#),
    );
}

/// One scheduled poll, a while after the last one.
fn poll(h: &Harness, core: &Core) {
    h.clock.advance(Duration::from_secs(12 * 60));
    core.refresh_usage(false);
}

fn sent(h: &Harness) -> Vec<(String, String)> {
    h.os.notifications.lock().unwrap().clone()
}

#[test]
fn crossing_the_default_warning_threshold_notifies_once_with_the_account_label() {
    let h = Harness::new();
    signed_in(&h);
    script(&h, (79.0, FIVE_RESET_1), (10.0, WEEK_RESET_1));
    script(&h, (80.0, FIVE_RESET_1), (10.0, WEEK_RESET_1));
    let core = h.core();

    core.refresh_usage(false);
    assert!(sent(&h).is_empty(), "79% is below the warning threshold");

    poll(&h, &core);

    let notes = sent(&h);
    assert_eq!(notes.len(), 1);
    let text = format!("{} {}", notes[0].0, notes[0].1);
    assert!(text.contains("Account 2"), "names the account: {text}");
    assert!(text.contains("5-hour"), "names the window: {text}");
    assert!(text.contains("80%"), "says how far along it is: {text}");
}

#[test]
fn no_repeat_within_the_same_window_across_polls() {
    let h = Harness::new();
    signed_in(&h);
    for percent in [85.0, 90.0, 95.0] {
        script(&h, (percent, FIVE_RESET_1), (10.0, WEEK_RESET_1));
    }
    let core = h.core();

    core.refresh_usage(false);
    poll(&h, &core);
    poll(&h, &core);

    assert_eq!(sent(&h).len(), 1);
}

#[test]
fn no_repeat_after_a_restart() {
    let h = Harness::new();
    signed_in(&h);
    script(&h, (85.0, FIVE_RESET_1), (10.0, WEEK_RESET_1));
    script(&h, (86.0, FIVE_RESET_1), (10.0, WEEK_RESET_1));
    h.core().refresh_usage(false);
    assert_eq!(sent(&h).len(), 1);

    let restarted = h.core();
    poll(&h, &restarted);

    assert_eq!(sent(&h).len(), 1, "the new core remembers what fired");
}

#[test]
fn reaching_the_limit_notifies_once() {
    let h = Harness::new();
    signed_in(&h);
    script(&h, (85.0, FIVE_RESET_1), (10.0, WEEK_RESET_1));
    script(&h, (100.0, FIVE_RESET_1), (10.0, WEEK_RESET_1));
    script(&h, (100.0, FIVE_RESET_1), (10.0, WEEK_RESET_1));
    let core = h.core();

    core.refresh_usage(false);
    poll(&h, &core);
    poll(&h, &core);

    let notes = sent(&h);
    assert_eq!(notes.len(), 2, "one warning, then one limit: {notes:?}");
    assert!(notes[1].0.contains("limit"), "{notes:?}");
    assert!(notes[1].1.contains("100%"), "{notes:?}");
}

#[test]
fn jumping_straight_past_the_warning_sends_only_the_limit_alert() {
    let h = Harness::new();
    signed_in(&h);
    script(&h, (60.0, FIVE_RESET_1), (10.0, WEEK_RESET_1));
    script(&h, (100.0, FIVE_RESET_1), (10.0, WEEK_RESET_1));
    script(&h, (100.0, FIVE_RESET_1), (10.0, WEEK_RESET_1));
    let core = h.core();

    core.refresh_usage(false);
    poll(&h, &core);
    poll(&h, &core);

    let notes = sent(&h);
    assert_eq!(notes.len(), 1, "{notes:?}");
    assert!(notes[0].0.contains("hit its limit"), "{notes:?}");
}

#[test]
fn the_weekly_window_has_its_own_alerts() {
    let h = Harness::new();
    signed_in(&h);
    script(&h, (10.0, FIVE_RESET_1), (82.0, WEEK_RESET_1));
    let core = h.core();

    core.refresh_usage(false);

    let notes = sent(&h);
    assert_eq!(notes.len(), 1);
    assert!(notes[0].1.contains("weekly"), "{notes:?}");
}

#[test]
fn both_windows_over_the_threshold_alert_separately() {
    let h = Harness::new();
    signed_in(&h);
    script(&h, (90.0, FIVE_RESET_1), (90.0, WEEK_RESET_1));
    let core = h.core();

    core.refresh_usage(false);

    assert_eq!(sent(&h).len(), 2);
}

#[test]
fn a_new_window_can_warn_again() {
    let h = Harness::new();
    signed_in(&h);
    script(&h, (85.0, FIVE_RESET_1), (10.0, WEEK_RESET_1));
    script(&h, (30.0, FIVE_RESET_2), (10.0, WEEK_RESET_1));
    script(&h, (84.0, FIVE_RESET_2), (10.0, WEEK_RESET_1));
    let core = h.core();

    core.refresh_usage(false);
    poll(&h, &core);
    assert_eq!(sent(&h).len(), 2, "the reset is announced: {:?}", sent(&h));
    poll(&h, &core);

    assert_eq!(sent(&h).len(), 3, "the new window warns again");
}

#[test]
fn a_wobble_in_resets_at_is_still_the_same_window() {
    let h = Harness::new();
    signed_in(&h);
    script(
        &h,
        (85.0, "2026-06-15T15:00:00.100000+00:00"),
        (10.0, WEEK_RESET_1),
    );
    script(
        &h,
        (86.0, "2026-06-15T15:00:00.900000+00:00"),
        (10.0, WEEK_RESET_1),
    );
    let core = h.core();

    core.refresh_usage(false);
    poll(&h, &core);

    assert_eq!(sent(&h).len(), 1);
}

#[test]
fn a_reset_names_the_account_window_and_its_usage() {
    let h = Harness::new();
    signed_in(&h);
    script(&h, (100.0, FIVE_RESET_1), (10.0, WEEK_RESET_1));
    script(&h, (2.0, FIVE_RESET_2), (10.0, WEEK_RESET_1));
    script(&h, (5.0, FIVE_RESET_2), (10.0, WEEK_RESET_1));
    let core = h.core();

    core.refresh_usage(false);
    poll(&h, &core);
    poll(&h, &core);

    let notes = sent(&h);
    assert_eq!(notes.len(), 2, "limit, then one reset: {notes:?}");
    assert!(
        notes[1].0.contains("Account 2's 5-hour window has reset"),
        "{notes:?}"
    );
    assert!(notes[1].1.contains("2% used"), "{notes:?}");
}

#[test]
fn a_reset_is_not_repeated_after_a_restart() {
    let h = Harness::new();
    signed_in(&h);
    script(&h, (100.0, FIVE_RESET_1), (10.0, WEEK_RESET_1));
    script(&h, (2.0, FIVE_RESET_2), (10.0, WEEK_RESET_1));
    script(&h, (3.0, FIVE_RESET_2), (10.0, WEEK_RESET_1));
    h.core().refresh_usage(false);
    poll(&h, &h.core());
    assert_eq!(sent(&h).len(), 2);

    poll(&h, &h.core());

    assert_eq!(sent(&h).len(), 2);
}

#[test]
fn a_window_that_was_never_limited_still_announces_its_reset() {
    let h = Harness::new();
    signed_in(&h);
    script(&h, (70.0, FIVE_RESET_1), (10.0, WEEK_RESET_1));
    script(&h, (1.0, FIVE_RESET_2), (10.0, WEEK_RESET_1));
    let core = h.core();

    core.refresh_usage(false);
    poll(&h, &core);

    let notes = sent(&h);
    assert_eq!(notes.len(), 1, "{notes:?}");
    assert!(notes[0].0.contains("5-hour window has reset"), "{notes:?}");
}

#[test]
fn the_weekly_window_announces_its_reset_too() {
    let h = Harness::new();
    signed_in(&h);
    script(&h, (10.0, FIVE_RESET_1), (40.0, WEEK_RESET_1));
    script(&h, (10.0, FIVE_RESET_1), (1.0, WEEK_RESET_2));
    let core = h.core();

    core.refresh_usage(false);
    assert!(sent(&h).is_empty(), "the first reading says nothing");
    poll(&h, &core);

    let notes = sent(&h);
    assert_eq!(notes.len(), 1, "{notes:?}");
    assert!(notes[0].0.contains("weekly window has reset"), "{notes:?}");
    assert!(notes[0].1.contains("1% used"), "{notes:?}");
}

#[test]
fn a_failed_poll_sends_nothing_and_loses_nothing() {
    let h = Harness::new();
    signed_in(&h);
    script(&h, (85.0, FIVE_RESET_1), (10.0, WEEK_RESET_1));
    h.http.respond(USAGE_URL, 500, "");
    script(&h, (86.0, FIVE_RESET_1), (10.0, WEEK_RESET_1));
    let core = h.core();

    core.refresh_usage(false);
    h.clock.advance(Duration::from_secs(2 * 3600));
    core.refresh_usage(false);
    h.clock.advance(Duration::from_secs(2 * 3600));
    core.refresh_usage(false);

    assert_eq!(sent(&h).len(), 1);
}

#[test]
fn notifications_name_the_account_by_label_and_never_leak_the_email_or_token() {
    let h = Harness::new();
    signed_in(&h);
    script(&h, (100.0, FIVE_RESET_1), (90.0, WEEK_RESET_1));
    script(&h, (1.0, FIVE_RESET_2), (10.0, WEEK_RESET_2));
    let core = h.core();
    core.refresh_usage(false);
    poll(&h, &core);

    let notes = sent(&h);
    assert!(notes.len() >= 3);
    for (title, body) in notes {
        let all = format!("{title} {body}");
        assert!(all.contains("Account 2"), "{all}");
        assert!(
            !all.contains(EMAIL) && !all.contains("example.com"),
            "{all}"
        );
        assert!(!all.contains(TOKEN) && !all.contains("sk-"), "{all}");
    }
}

#[test]
fn the_text_follows_the_saved_language() {
    let h = Harness::new();
    signed_in(&h);
    script(&h, (85.0, FIVE_RESET_1), (10.0, WEEK_RESET_1));
    let core = h.core();
    let mut settings = core.settings();
    settings.language = Language::Fa;
    core.update_settings(settings).unwrap();

    core.refresh_usage(false);

    let notes = sent(&h);
    assert_eq!(notes.len(), 1);
    assert!(notes[0].1.contains("۵ ساعته"), "{notes:?}");
}

/// The one notification a reading of 85% earns, under this language and calendar.
fn warning_under(language: Language, calendar: CalendarSetting) -> (String, String) {
    let h = Harness::new();
    signed_in(&h);
    script(&h, (85.0, FIVE_RESET_1), (10.0, WEEK_RESET_1));
    let core = h.core();
    change(&core, |s| {
        s.language = language;
        s.calendar = calendar;
    })
    .unwrap();
    core.refresh_usage(false);
    let mut notes = sent(&h);
    assert_eq!(notes.len(), 1, "{notes:?}");
    notes.remove(0)
}

#[test]
fn numbers_are_written_in_the_digits_of_the_language_not_the_calendar() {
    // Jalali in English: 0-9.
    let (_, body) = warning_under(Language::En, CalendarSetting::Persian);
    assert_eq!(body, "Its 5-hour usage is at 85%.");
    // Hijri in Russian: 0-9.
    let (_, body) = warning_under(Language::Ru, CalendarSetting::IslamicUmalqura);
    assert_eq!(body, "Расход за 5-часовое окно: 85%.");
    // Gregorian in Persian and Arabic: their own digits.
    let (_, body) = warning_under(Language::Fa, CalendarSetting::Gregory);
    assert_eq!(body, "مصرف ۵ ساعته آن به ۸۵٪ رسیده است.");
    let (_, body) = warning_under(Language::Ar, CalendarSetting::Gregory);
    assert_eq!(body, "بلغ استخدام نافذة ٥ ساعات ٨٥٪.");
    // Chinese lunar: 0-9.
    let (_, body) = warning_under(Language::Zh, CalendarSetting::Chinese);
    assert_eq!(body, "5 小时用量已达 85%。");
}

#[test]
fn every_event_and_window_reads_in_arabic_russian_and_chinese() {
    // A 5-hour limit and a weekly warning, then both windows resetting.
    let expected: [(Language, [(&str, &str); 4]); 3] = [
        (
            Language::Ar,
            [
                (
                    "الحساب Account 2 بلغ حدّه",
                    "بلغ استخدام نافذة ٥ ساعات ١٠٠٪. انتقل إلى حساب آخر.",
                ),
                (
                    "الحساب Account 2 يقترب من حدّه",
                    "بلغ استخدام نافذة أسبوعية ٩٠٪.",
                ),
                (
                    "أُعيد ضبط نافذة ٥ ساعات للحساب Account 2",
                    "عاد الاستخدام إلى ١٪.",
                ),
                (
                    "أُعيد ضبط نافذة أسبوعية للحساب Account 2",
                    "عاد الاستخدام إلى ١٠٪.",
                ),
            ],
        ),
        (
            Language::Ru,
            [
                (
                    "Лимит Account 2 исчерпан",
                    "Расход за 5-часовое окно: 100%. Переключитесь на другой аккаунт.",
                ),
                ("Account 2: лимит близко", "Расход за недельное окно: 90%."),
                ("Сброшено 5-часовое окно: Account 2", "Теперь расход: 1%."),
                ("Сброшено недельное окно: Account 2", "Теперь расход: 10%."),
            ],
        ),
        (
            Language::Zh,
            [
                (
                    "Account 2 已达用量上限",
                    "5 小时用量已达 100%。请切换到其他账户。",
                ),
                ("Account 2 即将达到用量上限", "每周用量已达 90%。"),
                ("Account 2：5 小时窗口已重置", "用量已回到 1%。"),
                ("Account 2：每周窗口已重置", "用量已回到 10%。"),
            ],
        ),
    ];
    for (language, texts) in expected {
        let h = Harness::new();
        signed_in(&h);
        script(&h, (100.0, FIVE_RESET_1), (90.0, WEEK_RESET_1));
        script(&h, (1.0, FIVE_RESET_2), (10.0, WEEK_RESET_2));
        let core = h.core();
        change(&core, |s| s.language = language).unwrap();
        core.refresh_usage(false);
        poll(&h, &core);

        let texts: Vec<(String, String)> = texts
            .iter()
            .map(|(title, body)| (title.to_string(), body.to_string()))
            .collect();
        assert_eq!(sent(&h), texts, "{language:?}");
    }
}

// Settings: switches and thresholds

fn change(core: &Core, edit: impl FnOnce(&mut Settings)) -> Result<Settings, SettingsError> {
    let mut settings = core.settings();
    edit(&mut settings);
    core.update_settings(settings)
}

#[test]
fn the_defaults_are_everything_on_with_80_and_100() {
    let h = Harness::new();

    let s = h.core().settings();

    assert!(s.notify_warning && s.notify_limited && s.notify_reset);
    assert_eq!((s.warning_threshold, s.limit_threshold), (80, 100));
}

#[test]
fn switches_and_thresholds_survive_a_restart() {
    let h = Harness::new();
    change(&h.core(), |s| {
        s.notify_warning = false;
        s.notify_reset = false;
        s.warning_threshold = 60;
        s.limit_threshold = 95;
    })
    .unwrap();

    let s = h.core().settings();

    assert!(!s.notify_warning && s.notify_limited && !s.notify_reset);
    assert_eq!((s.warning_threshold, s.limit_threshold), (60, 95));
}

#[test]
fn the_warning_must_sit_below_the_limit() {
    let h = Harness::new();
    let core = h.core();

    for (warning, limit) in [(90, 90), (95, 90)] {
        let result = change(&core, |s| {
            s.warning_threshold = warning;
            s.limit_threshold = limit;
        });
        assert!(matches!(result, Err(SettingsError::InvalidThresholds)));
    }
    assert_eq!(core.settings(), Settings::default(), "nothing was saved");
}

#[test]
fn thresholds_must_be_between_1_and_100() {
    let h = Harness::new();
    let core = h.core();

    for (warning, limit) in [(0, 100), (80, 101), (0, 0), (200, 255)] {
        let result = change(&core, |s| {
            s.warning_threshold = warning;
            s.limit_threshold = limit;
        });
        assert!(
            matches!(result, Err(SettingsError::InvalidThresholds)),
            "{warning}/{limit}"
        );
    }
    assert!(change(&core, |s| (s.warning_threshold, s.limit_threshold) = (1, 2)).is_ok());
    assert!(
        change(&core, |s| (s.warning_threshold, s.limit_threshold) =
            (99, 100))
        .is_ok()
    );
}

#[test]
fn a_refused_threshold_keeps_the_other_changes_unsaved_too() {
    let h = Harness::new();
    let core = h.core();

    let result = change(&core, |s| {
        s.close_to_tray = false;
        s.warning_threshold = 100;
    });

    assert!(result.is_err());
    assert!(h.core().settings().close_to_tray);
}

#[test]
fn a_saved_file_with_unusable_thresholds_falls_back_to_80_and_100() {
    let h = Harness::new();
    let dir = h.fixture.local_app_data().join("clawshell");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("settings.json"),
        r#"{ "warningThreshold": 100, "limitThreshold": 50, "closeToTray": false }"#,
    )
    .unwrap();

    let s = h.core().settings();

    assert_eq!((s.warning_threshold, s.limit_threshold), (80, 100));
    assert!(!s.close_to_tray, "the rest of the file still counts");
}

#[test]
fn a_custom_warning_threshold_is_the_one_that_fires() {
    let h = Harness::new();
    signed_in(&h);
    script(&h, (55.0, FIVE_RESET_1), (10.0, WEEK_RESET_1));
    script(&h, (60.0, FIVE_RESET_1), (10.0, WEEK_RESET_1));
    let core = h.core();
    change(&core, |s| s.warning_threshold = 60).unwrap();

    core.refresh_usage(false);
    assert!(sent(&h).is_empty());
    poll(&h, &core);

    assert_eq!(sent(&h).len(), 1);
}

#[test]
fn a_custom_limit_threshold_is_the_one_that_counts_as_limited() {
    let h = Harness::new();
    signed_in(&h);
    script(&h, (92.0, FIVE_RESET_1), (10.0, WEEK_RESET_1));
    let core = h.core();
    change(&core, |s| s.limit_threshold = 90).unwrap();

    core.refresh_usage(false);

    let notes = sent(&h);
    assert_eq!(notes.len(), 1);
    assert!(notes[0].0.contains("hit its limit"), "{notes:?}");
}

#[test]
fn each_notification_type_has_its_own_switch() {
    let h = Harness::new();
    signed_in(&h);
    script(&h, (85.0, FIVE_RESET_1), (10.0, WEEK_RESET_1));
    script(&h, (100.0, FIVE_RESET_1), (10.0, WEEK_RESET_1));
    script(&h, (3.0, FIVE_RESET_2), (10.0, WEEK_RESET_1));
    let core = h.core();
    change(&core, |s| {
        s.notify_warning = false;
        s.notify_limited = false;
    })
    .unwrap();

    core.refresh_usage(false);
    poll(&h, &core);
    assert!(sent(&h).is_empty(), "warning and limit are off");
    poll(&h, &core);

    let notes = sent(&h);
    assert_eq!(notes.len(), 1, "the reset switch is still on: {notes:?}");
    assert!(notes[0].0.contains("5-hour window has reset"), "{notes:?}");
}

#[test]
fn the_reset_switch_off_silences_only_the_reset() {
    let h = Harness::new();
    signed_in(&h);
    script(&h, (100.0, FIVE_RESET_1), (10.0, WEEK_RESET_1));
    script(&h, (3.0, FIVE_RESET_2), (10.0, WEEK_RESET_1));
    let core = h.core();
    change(&core, |s| s.notify_reset = false).unwrap();

    core.refresh_usage(false);
    poll(&h, &core);

    assert_eq!(sent(&h).len(), 1, "only the limit alert");
}

#[test]
fn with_the_limit_alert_off_a_jump_to_the_limit_still_warns() {
    let h = Harness::new();
    signed_in(&h);
    script(&h, (100.0, FIVE_RESET_1), (10.0, WEEK_RESET_1));
    let core = h.core();
    change(&core, |s| s.notify_limited = false).unwrap();

    core.refresh_usage(false);

    let notes = sent(&h);
    assert_eq!(notes.len(), 1);
    assert!(notes[0].0.contains("nearing"), "{notes:?}");
}

#[test]
fn a_switch_turned_off_mid_window_does_not_use_up_the_alert() {
    let h = Harness::new();
    signed_in(&h);
    script(&h, (85.0, FIVE_RESET_1), (10.0, WEEK_RESET_1));
    script(&h, (86.0, FIVE_RESET_1), (10.0, WEEK_RESET_1));
    let core = h.core();
    change(&core, |s| s.notify_warning = false).unwrap();
    core.refresh_usage(false);
    assert!(sent(&h).is_empty());

    change(&core, |s| s.notify_warning = true).unwrap();
    poll(&h, &core);

    assert_eq!(sent(&h).len(), 1);
}

#[test]
fn a_new_threshold_applies_to_the_current_reading_without_waiting_for_a_poll() {
    let h = Harness::new();
    signed_in(&h);
    script(&h, (65.0, FIVE_RESET_1), (10.0, WEEK_RESET_1));
    let core = h.core();
    core.refresh_usage(false);
    assert!(sent(&h).is_empty());
    let requests = h.http.requests().len();

    change(&core, |s| s.warning_threshold = 60).unwrap();

    assert_eq!(sent(&h).len(), 1);
    assert_eq!(h.http.requests().len(), requests, "no extra poll was made");
}
