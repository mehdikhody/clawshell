mod support;

use clawshell_lib::core::{CalendarSetting, Language, Settings, SettingsError};
use support::{Harness, add_default};

const ID: &str = "3f2b8c1e-5a4d-4e7b-9c10-2d6f8a9b0c11";

#[test]
fn a_fresh_install_has_the_default_settings() {
    let h = Harness::new();

    let settings = h.core().settings();

    assert_eq!(
        settings,
        Settings {
            launch_at_boot: false,
            start_hidden: false,
            close_to_tray: true,
            language: Language::En,
            calendar: CalendarSetting::Automatic,
            claude_path: None,
            notify_warning: true,
            notify_limited: true,
            notify_reset: true,
            warning_threshold: 80,
            limit_threshold: 100,
            sign_in_routing: false,
        }
    );
}

#[test]
fn settings_survive_a_restart() {
    let h = Harness::new();
    let mut changed = h.core().settings();
    changed.start_hidden = true;
    changed.close_to_tray = false;

    h.core().update_settings(changed.clone()).unwrap();

    assert_eq!(h.core().settings(), changed);
}

#[test]
fn the_language_survives_a_restart() {
    let h = Harness::new();
    let core = h.core();
    let mut settings = core.settings();
    settings.language = Language::Fa;
    core.update_settings(settings).unwrap();

    assert_eq!(h.core().settings().language, Language::Fa);
}

#[test]
fn the_language_is_saved_as_its_lowercase_code() {
    for (language, code) in [
        (Language::En, "en"),
        (Language::Fa, "fa"),
        (Language::Ar, "ar"),
        (Language::Ru, "ru"),
        (Language::Zh, "zh"),
    ] {
        let h = Harness::new();
        let core = h.core();
        let mut settings = core.settings();
        settings.language = language;
        core.update_settings(settings).unwrap();

        let text =
            std::fs::read_to_string(h.fixture.local_app_data().join("clawshell/settings.json"))
                .unwrap();
        assert!(
            text.contains(&format!("\"language\": \"{code}\"")),
            "{text}"
        );
        assert_eq!(h.core().settings().language, language);
    }
}

#[test]
fn a_first_run_takes_the_language_of_windows() {
    // LANGIDs: the primary language in the low ten bits, the region above.
    for (langid, language) in [
        (0x0409, Language::En),
        (0x0429, Language::Fa),
        (0x0401, Language::Ar),
        (0x0C01, Language::Ar),
        (0x0419, Language::Ru),
        (0x0804, Language::Zh),
        (0x0404, Language::Zh),
        (0x0C04, Language::Zh),
        (0x040C, Language::En),
        (0, Language::En),
    ] {
        let h = Harness::new();
        *h.os.ui_language.lock().unwrap() = langid;

        assert_eq!(h.core().settings().language, language, "{langid:#06x}");
    }
}

#[test]
fn a_saved_language_is_never_replaced_by_the_language_of_windows() {
    let h = Harness::new();
    let core = h.core();
    let mut settings = core.settings();
    settings.language = Language::En;
    core.update_settings(settings).unwrap();

    *h.os.ui_language.lock().unwrap() = 0x0429;

    assert_eq!(h.core().settings().language, Language::En);
}

#[test]
fn a_settings_file_without_a_language_is_not_given_the_language_of_windows() {
    let h = Harness::new();
    *h.os.ui_language.lock().unwrap() = 0x0429;
    let dir = h.fixture.local_app_data().join("clawshell");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("settings.json"), r#"{ "closeToTray": false }"#).unwrap();

    assert_eq!(h.core().settings().language, Language::En);
}

#[test]
fn reading_the_language_of_windows_creates_no_settings_file() {
    let h = Harness::new();
    *h.os.ui_language.lock().unwrap() = 0x0429;

    h.core().settings();

    assert!(
        !h.fixture
            .local_app_data()
            .join("clawshell/settings.json")
            .exists()
    );
}

#[test]
fn a_settings_file_from_before_languages_opens_in_english() {
    let h = Harness::new();
    let dir = h.fixture.local_app_data().join("clawshell");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("settings.json"), r#"{ "closeToTray": false }"#).unwrap();

    let settings = h.core().settings();

    assert_eq!(settings.language, Language::En);
    assert!(!settings.close_to_tray);
}

#[test]
fn the_calendar_is_saved_by_its_intl_id() {
    for (calendar, id) in [
        (CalendarSetting::Automatic, "automatic"),
        (CalendarSetting::Gregory, "gregory"),
        (CalendarSetting::Persian, "persian"),
        (CalendarSetting::IslamicUmalqura, "islamic-umalqura"),
        (CalendarSetting::Chinese, "chinese"),
    ] {
        let h = Harness::new();
        let core = h.core();
        let mut settings = core.settings();
        settings.calendar = calendar;
        core.update_settings(settings).unwrap();

        let text =
            std::fs::read_to_string(h.fixture.local_app_data().join("clawshell/settings.json"))
                .unwrap();
        assert!(text.contains(&format!("\"calendar\": \"{id}\"")), "{text}");
        assert_eq!(h.core().settings().calendar, calendar);
    }
}

#[test]
fn a_settings_file_from_before_calendars_opens_on_automatic() {
    let h = Harness::new();
    let dir = h.fixture.local_app_data().join("clawshell");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("settings.json"), r#"{ "language": "fa" }"#).unwrap();

    assert_eq!(h.core().settings().calendar, CalendarSetting::Automatic);
}

#[test]
fn a_pinned_calendar_stays_when_the_language_changes() {
    let h = Harness::new();
    let core = h.core();
    let mut settings = core.settings();
    settings.calendar = CalendarSetting::Chinese;
    core.update_settings(settings).unwrap();

    let mut settings = core.settings();
    settings.language = Language::Ar;
    core.update_settings(settings).unwrap();

    assert_eq!(h.core().settings().calendar, CalendarSetting::Chinese);
    assert_eq!(h.core().settings().language, Language::Ar);
}

#[test]
fn an_unreadable_settings_file_falls_back_to_the_defaults() {
    let h = Harness::new();
    let dir = h.fixture.local_app_data().join("clawshell");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("settings.json"), "{ not json").unwrap();

    assert_eq!(h.core().settings(), Settings::default());
}

#[test]
fn turning_launch_at_boot_on_and_off_registers_and_unregisters_autostart() {
    let h = Harness::new();
    let core = h.core();
    let mut settings = core.settings();

    settings.launch_at_boot = true;
    core.update_settings(settings.clone()).unwrap();
    settings.launch_at_boot = false;
    core.update_settings(settings).unwrap();

    assert_eq!(*h.os.autostart.lock().unwrap(), vec![true, false]);
}

#[test]
fn autostart_is_left_alone_when_launch_at_boot_did_not_change() {
    let h = Harness::new();
    let core = h.core();
    let mut settings = core.settings();

    settings.close_to_tray = false;
    core.update_settings(settings).unwrap();

    assert!(h.os.autostart.lock().unwrap().is_empty());
}

#[test]
fn a_failed_autostart_change_is_refused_and_not_saved() {
    let h = Harness::new();
    *h.os.autostart_fails.lock().unwrap() = true;
    let core = h.core();
    let mut settings = core.settings();
    settings.launch_at_boot = true;
    settings.close_to_tray = false;

    let result = core.update_settings(settings);

    assert!(matches!(result, Err(SettingsError::Autostart(_))));
    assert_eq!(h.core().settings(), Settings::default());
}

// Claude.exe path override

fn with_path(h: &Harness, path: &std::path::Path) -> Result<Settings, SettingsError> {
    let core = h.core();
    let mut settings = core.settings();
    settings.claude_path = Some(path.to_path_buf());
    core.update_settings(settings)
}

fn fake_claude(dir: &std::path::Path, name: &str) -> std::path::PathBuf {
    let exe = dir.join(name);
    std::fs::write(&exe, b"MZ").unwrap();
    exe
}

#[test]
fn a_path_that_does_not_exist_is_rejected() {
    let h = Harness::new();
    let missing = h.fixture.local_app_data().join("nowhere/Claude.exe");

    let result = with_path(&h, &missing);

    assert!(matches!(result, Err(SettingsError::ClaudePathMissing)));
    assert_eq!(h.core().settings().claude_path, None);
}

#[test]
fn a_folder_is_rejected_as_the_claude_path() {
    let h = Harness::new();
    let folder = h.fixture.add_folder("Claude.exe");

    let result = with_path(&h, &folder);

    assert!(matches!(result, Err(SettingsError::ClaudePathNotAFile)));
}

#[test]
fn a_file_that_is_not_claude_exe_is_rejected() {
    let h = Harness::new();
    let notepad = fake_claude(&h.fixture.local_app_data(), "notepad.exe");

    let result = with_path(&h, &notepad);

    assert!(matches!(result, Err(SettingsError::ClaudePathNotClaude)));
    assert_eq!(h.core().settings().claude_path, None);
}

#[test]
fn a_rejected_path_keeps_the_other_changes_unsaved_too() {
    let h = Harness::new();
    let core = h.core();
    let mut settings = core.settings();
    settings.close_to_tray = false;
    settings.claude_path = Some(h.fixture.local_app_data().join("nowhere/Claude.exe"));

    assert!(core.update_settings(settings).is_err());

    assert!(h.core().settings().close_to_tray);
}

#[test]
fn a_real_claude_exe_is_accepted_whatever_its_letter_case() {
    let h = Harness::new();
    let exe = fake_claude(&h.fixture.local_app_data(), "CLAUDE.EXE");

    let saved = with_path(&h, &exe).unwrap();

    assert_eq!(saved.claude_path.as_deref(), Some(exe.as_path()));
    assert_eq!(h.core().settings().claude_path, Some(exe));
}

#[test]
fn clearing_the_override_is_always_allowed() {
    let h = Harness::new();
    let exe = fake_claude(&h.fixture.local_app_data(), "Claude.exe");
    with_path(&h, &exe).unwrap();
    std::fs::remove_file(&exe).unwrap();
    let core = h.core();
    let mut settings = core.settings();
    settings.claude_path = None;

    core.update_settings(settings).unwrap();

    assert_eq!(h.core().settings().claude_path, None);
}

#[test]
fn saving_other_settings_does_not_recheck_an_unchanged_override() {
    let h = Harness::new();
    let exe = fake_claude(&h.fixture.local_app_data(), "Claude.exe");
    with_path(&h, &exe).unwrap();
    std::fs::remove_file(&exe).unwrap();
    let core = h.core();
    let mut settings = core.settings();
    settings.close_to_tray = false;

    core.update_settings(settings).unwrap();

    assert!(!h.core().settings().close_to_tray);
}

#[test]
fn the_override_launches_claude_when_detection_finds_nothing() {
    let h = Harness::new();
    *h.os.install.lock().unwrap() = None;
    let exe = fake_claude(&h.fixture.local_app_data(), "Claude.exe");
    with_path(&h, &exe).unwrap();
    h.fixture.add_profile(ID);

    assert!(h.core().list_accounts().unwrap().claude_found);
    h.core().launch_account(ID).unwrap();

    let launched = h.os.launched.lock().unwrap();
    assert_eq!(launched.len(), 1);
    assert_eq!(launched[0].0, exe);
    assert_eq!(launched[0].1, h.fixture.profile_path(ID));
}

#[test]
fn without_an_override_a_missing_claude_still_blocks_launching() {
    let h = Harness::new();
    *h.os.install.lock().unwrap() = None;
    h.fixture.add_profile(ID);

    assert!(!h.core().list_accounts().unwrap().claude_found);
    assert!(h.core().launch_account(ID).is_err());
}

#[test]
fn a_detected_install_is_used_before_the_override() {
    let h = Harness::new();
    let exe = fake_claude(&h.fixture.local_app_data(), "Claude.exe");
    with_path(&h, &exe).unwrap();
    h.fixture.add_profile(ID);

    h.core().launch_account(ID).unwrap();

    assert_eq!(
        h.os.launched.lock().unwrap()[0].0,
        support::fake_install().exe
    );
}

#[test]
fn launching_an_account_uses_the_override_too() {
    let h = Harness::new();
    *h.os.install.lock().unwrap() = None;
    let exe = fake_claude(&h.fixture.local_app_data(), "Claude.exe");
    with_path(&h, &exe).unwrap();

    let added = add_default(&h.core());
    h.core().launch_account(&added.id).unwrap();

    assert_eq!(h.os.launched.lock().unwrap()[0].0, exe);
}
