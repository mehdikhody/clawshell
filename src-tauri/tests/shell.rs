//! Window and tray behaviour decided by the core: what the close button does,
//! whether a launch shows the window, and what the tray menu offers.

mod support;

use clawshell_lib::core::{CloseAction, Language, TrayEntry};
use support::Harness;

const ID_A: &str = "3f2b8c1e-5a4d-4e7b-9c10-2d6f8a9b0c11";
const ID_B: &str = "a0b1c2d3-e4f5-4a6b-8c7d-9e0f1a2b3c4d";

#[test]
fn closing_hides_to_the_tray_by_default() {
    let h = Harness::new();
    h.core().accept_disclaimer().unwrap();

    assert_eq!(h.core().close_action(), CloseAction::HideToTray);
}

#[test]
fn closing_quits_when_close_to_tray_is_off() {
    let h = Harness::new();
    let core = h.core();
    let mut settings = core.settings();
    settings.close_to_tray = false;
    core.update_settings(settings).unwrap();

    assert_eq!(h.core().close_action(), CloseAction::Quit);
}

#[test]
fn closing_quits_while_the_disclaimer_is_pending_whatever_close_to_tray_says() {
    let h = Harness::new();
    let core = h.core();
    let mut settings = core.settings();
    settings.close_to_tray = true;
    core.update_settings(settings).unwrap();
    assert_eq!(h.core().close_action(), CloseAction::Quit);

    core.accept_disclaimer().unwrap();

    assert_eq!(h.core().close_action(), CloseAction::HideToTray);
}

#[test]
fn a_normal_launch_shows_the_window() {
    let h = Harness::new();

    assert!(h.core().shows_window_at_launch(false));
}

#[test]
fn a_boot_launch_shows_the_window_unless_start_hidden_is_on() {
    let h = Harness::new();
    h.core().accept_disclaimer().unwrap();
    assert!(h.core().shows_window_at_launch(true));

    let core = h.core();
    let mut settings = core.settings();
    settings.start_hidden = true;
    core.update_settings(settings).unwrap();

    assert!(!h.core().shows_window_at_launch(true));
    assert!(h.core().shows_window_at_launch(false));
}

#[test]
fn a_boot_launch_shows_the_window_with_start_hidden_until_the_disclaimer_is_accepted() {
    let h = Harness::new();
    let core = h.core();
    let mut settings = core.settings();
    settings.start_hidden = true;
    core.update_settings(settings).unwrap();

    assert!(h.core().shows_window_at_launch(true));
}

fn account(id: &str, label: &str, running: bool) -> TrayEntry {
    TrayEntry::Account {
        id: id.to_string(),
        label: label.to_string(),
        running,
    }
}

#[test]
fn the_tray_menu_with_no_added_accounts_offers_the_default_one() {
    let h = Harness::new();

    assert_eq!(
        h.core().tray_entries().unwrap(),
        vec![
            TrayEntry::Open,
            TrayEntry::Separator,
            account("default", "Claude", false),
            TrayEntry::Separator,
            TrayEntry::AddAccount,
            TrayEntry::Quit
        ]
    );
}

#[test]
fn the_tray_menu_lists_accounts_and_ticks_the_running_ones() {
    let h = Harness::new();
    h.fixture.add_profile(ID_A);
    h.fixture.add_profile(ID_B);
    h.fixture.sign_in(ID_A);
    h.fixture.sign_in(ID_B);
    h.processes
        .0
        .lock()
        .unwrap()
        .insert(h.fixture.profile_path(ID_B));

    assert_eq!(
        h.core().tray_entries().unwrap(),
        vec![
            TrayEntry::Open,
            TrayEntry::Separator,
            account("default", "Claude", false),
            account(ID_A, "Account 2", false),
            account(ID_B, "Account 3", true),
            TrayEntry::Separator,
            TrayEntry::AddAccount,
            TrayEntry::Quit,
        ]
    );
}

#[test]
fn an_account_whose_folder_is_gone_is_not_offered_in_the_tray() {
    let h = Harness::new();
    h.fixture.add_profile(ID_A);
    h.core().list_accounts().unwrap(); // records the label
    std::fs::remove_dir_all(h.fixture.profile_path(ID_A)).unwrap();

    assert_eq!(
        h.core().tray_entries().unwrap(),
        vec![
            TrayEntry::Open,
            TrayEntry::Separator,
            account("default", "Claude", false),
            TrayEntry::Separator,
            TrayEntry::AddAccount,
            TrayEntry::Quit
        ]
    );
}

// Tray labels follow the language setting

#[test]
fn tray_labels_are_english_by_default() {
    let h = Harness::new();

    let labels = h.core().tray_labels();

    assert_eq!(labels.open, "Open clawshell");
    assert_eq!(labels.add_account, "Add account");
    assert_eq!(labels.quit, "Quit");
}

#[test]
fn tray_labels_switch_to_persian_with_the_language_setting() {
    let h = Harness::new();
    let core = h.core();
    let mut settings = core.settings();
    settings.language = Language::Fa;
    core.update_settings(settings).unwrap();

    let labels = h.core().tray_labels();

    assert_eq!(labels.open, "باز کردن clawshell");
    assert_eq!(labels.add_account, "افزودن حساب");
    assert_eq!(labels.quit, "خروج");
}

#[test]
fn tray_labels_switch_to_arabic_russian_and_chinese_with_the_language_setting() {
    for (language, open, add_account, quit) in [
        (Language::Ar, "فتح clawshell", "إضافة حساب", "خروج"),
        (
            Language::Ru,
            "Открыть clawshell",
            "Добавить аккаунт",
            "Выход",
        ),
        (Language::Zh, "打开 clawshell", "添加账户", "退出"),
    ] {
        let h = Harness::new();
        let core = h.core();
        let mut settings = core.settings();
        settings.language = language;
        core.update_settings(settings).unwrap();

        let labels = h.core().tray_labels();

        assert_eq!(labels.open, open, "{language:?}");
        assert_eq!(labels.add_account, add_account, "{language:?}");
        assert_eq!(labels.quit, quit, "{language:?}");
    }
}
