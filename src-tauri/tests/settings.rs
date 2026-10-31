mod support;

use clawshell_lib::core::{Language, Settings, SettingsError};
use support::Harness;

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
fn the_language_is_saved_as_en_or_fa() {
    let h = Harness::new();
    let core = h.core();
    let mut settings = core.settings();
    settings.language = Language::Fa;
    core.update_settings(settings).unwrap();

    let text = std::fs::read_to_string(h.fixture.local_app_data().join("clawshell/settings.json"))
        .unwrap();
    assert!(text.contains("\"language\": \"fa\""), "{text}");
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
fn adding_an_account_uses_the_override_too() {
    let h = Harness::new();
    *h.os.install.lock().unwrap() = None;
    let exe = fake_claude(&h.fixture.local_app_data(), "Claude.exe");
    with_path(&h, &exe).unwrap();

    h.core().add_account().unwrap();

    assert_eq!(h.os.launched.lock().unwrap()[0].0, exe);
}
