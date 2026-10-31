mod support;

use clawshell_lib::core::AccountError;
use clawshell_lib::core::ports::ClaudeInstall;
use clawshell_lib::core::shortcuts::{
    ShortcutError, account_to_open, encode_ico, launch_args, shortcut_name,
};
use clawshell_lib::core::{LaunchError, LaunchOutcome};
use std::path::{Path, PathBuf};
use support::Harness;

const ID_A: &str = "3f2b8c1e-5a4d-4e7b-9c10-2d6f8a9b0c11";

fn args(list: &[&str]) -> Vec<String> {
    list.iter().map(|a| a.to_string()).collect()
}

// ---- pure: naming and arguments ------------------------------------------------

#[test]
fn a_shortcut_is_named_claude_dash_the_label() {
    assert_eq!(shortcut_name("Work"), "Claude - Work");
}

#[test]
fn characters_windows_forbids_in_file_names_are_replaced() {
    assert_eq!(
        shortcut_name(r#"a<b>c:d"e/f\g|h?i*j"#),
        "Claude - a_b_c_d_e_f_g_h_i_j"
    );
}

#[test]
fn trailing_dots_and_spaces_are_dropped_from_the_name() {
    assert_eq!(shortcut_name("Work. . "), "Claude - Work");
}

#[test]
fn a_very_long_label_is_cut_to_keep_the_path_short() {
    let name = shortcut_name(&"x".repeat(300));
    assert_eq!(name.chars().count(), "Claude - ".len() + 80);
}

#[test]
fn the_launch_arguments_name_the_account() {
    assert_eq!(launch_args(ID_A), format!("--account {ID_A}"));
}

#[test]
fn a_command_line_with_an_account_id_asks_to_open_that_account() {
    let line = args(&["clawshell.exe", "--account", ID_A]);
    assert_eq!(account_to_open(&line).as_deref(), Some(ID_A));
    let line = args(&["clawshell.exe", &format!("--account={ID_A}")]);
    assert_eq!(account_to_open(&line).as_deref(), Some(ID_A));
}

#[test]
fn a_command_line_without_a_well_formed_account_id_opens_nothing() {
    for line in [
        args(&["clawshell.exe"]),
        args(&["clawshell.exe", "--autostart"]),
        args(&["clawshell.exe", "--account"]),
        args(&["clawshell.exe", "--account", "..\\Claude-Data"]),
        args(&["clawshell.exe", "--account", "not-a-uuid"]),
    ] {
        assert_eq!(account_to_open(&line), None, "{line:?}");
    }
}

// ---- pure: the .ico file -------------------------------------------------------

#[test]
fn an_ico_holds_each_image_as_a_bottom_up_bmp_with_an_empty_mask() {
    // 2x2: the top row is red then green, the bottom row blue then white.
    let top = [0, 0, 255, 255, 0, 255, 0, 255];
    let bottom = [255, 0, 0, 255, 255, 255, 255, 255];
    let pixels: Vec<u8> = [top, bottom].concat();

    let ico = encode_ico(&[(2, pixels)]);

    let expected: Vec<u8> = [
        // ICONDIR: reserved, type 1 (icon), one image.
        &[0, 0, 1, 0, 1, 0][..],
        // ICONDIRENTRY: 2x2, no palette, planes 1, 32 bits, 64 bytes at offset 22.
        &[2, 2, 0, 0, 1, 0, 32, 0, 64, 0, 0, 0, 22, 0, 0, 0],
        // BITMAPINFOHEADER: 40, width 2, height 4 (image + mask), 1 plane, 32 bits.
        &[40, 0, 0, 0, 2, 0, 0, 0, 4, 0, 0, 0, 1, 0, 32, 0],
        &[0; 24],
        // Pixels, bottom row first.
        &bottom,
        &top,
        // AND mask: two rows, each padded to four bytes, all opaque.
        &[0; 8],
    ]
    .concat();
    assert_eq!(ico, expected);
}

#[test]
fn a_256_pixel_image_is_written_with_a_zero_size_byte() {
    let ico = encode_ico(&[(256, vec![0; 256 * 256 * 4])]);

    assert_eq!(&ico[6..8], &[0, 0]);
}

#[test]
fn several_images_each_get_an_entry_pointing_at_their_data() {
    let ico = encode_ico(&[(1, vec![0; 4]), (2, vec![0; 16])]);

    assert_eq!(&ico[4..6], &[2, 0]);
    // Entry sizes: 1x1 is 40 + 4 + 4 bytes, and its data starts after both entries.
    assert_eq!(&ico[14..18], &[48, 0, 0, 0]);
    assert_eq!(&ico[18..22], &[38, 0, 0, 0]);
    assert_eq!(&ico[30..34], &[64, 0, 0, 0]);
    assert_eq!(&ico[34..38], &[38 + 48, 0, 0, 0]);
    assert_eq!(ico.len(), 38 + 48 + 64);
}

// ---- adding --------------------------------------------------------------------

const ID_B: &str = "a0b1c2d3-e4f5-4a6b-8c7d-9e0f1a2b3c4d";

fn exists(path: &Path) -> bool {
    path.exists()
}

fn desktop_link(h: &Harness, name: &str) -> PathBuf {
    h.fixture.desktop().join(format!("{name}.lnk"))
}

fn start_link(h: &Harness, name: &str) -> PathBuf {
    h.fixture.start_menu().join(format!("{name}.lnk"))
}

fn icons_dir(h: &Harness) -> PathBuf {
    h.fixture.local_app_data().join("clawshell").join("icons")
}

/// A machine with one account called "Work".
fn work() -> Harness {
    let h = Harness::new();
    h.fixture.add_profile(ID_A);
    h.core().rename_account(ID_A, "Work").unwrap();
    h
}

#[test]
fn add_creates_a_claude_dash_name_shortcut_on_the_desktop_and_in_the_start_menu() {
    let h = work();

    h.core().add_shortcuts(ID_A).unwrap();

    assert!(exists(&desktop_link(&h, "Claude - Work")));
    assert!(exists(&start_link(&h, "Claude - Work")));
    let written: Vec<PathBuf> =
        h.os.links_written
            .lock()
            .unwrap()
            .iter()
            .map(|(path, _)| path.clone())
            .collect();
    assert_eq!(
        written,
        vec![
            desktop_link(&h, "Claude - Work"),
            start_link(&h, "Claude - Work")
        ]
    );
}

#[test]
fn a_shortcut_launches_clawshell_with_the_account_id() {
    let h = work();

    h.core().add_shortcuts(ID_A).unwrap();

    for (_, link) in h.os.links_written.lock().unwrap().iter() {
        assert_eq!(
            link.target,
            PathBuf::from("C:/Apps/clawshell/clawshell.exe")
        );
        assert_eq!(link.args, format!("--account {ID_A}"));
    }
}

#[test]
fn a_shortcut_never_points_at_the_claude_install_so_it_survives_updates() {
    let h = work();
    let versioned = ClaudeInstall {
        exe: PathBuf::from("C:/Claude/app-1.2.3/Claude.exe"),
        root: PathBuf::from("C:/Claude"),
    };
    *h.os.install.lock().unwrap() = Some(versioned);
    h.core().add_shortcuts(ID_A).unwrap();
    let before = h.os.links_written.lock().unwrap().clone();

    // Claude updates itself into a new versioned folder.
    *h.os.install.lock().unwrap() = Some(ClaudeInstall {
        exe: PathBuf::from("C:/Claude/app-1.2.4/Claude.exe"),
        root: PathBuf::from("C:/Claude"),
    });
    h.core().add_shortcuts(ID_A).unwrap();

    let after = h.os.links_written.lock().unwrap().clone();
    assert_eq!(after[2..], before[..], "same shortcuts after the update");
    for (_, link) in &before {
        assert!(!link.target.to_string_lossy().contains("Claude/app"));
        assert!(!link.args.contains("Claude"));
    }
}

#[test]
fn the_shortcut_icon_is_the_accounts_badge_in_clawshells_app_data() {
    let h = work();
    let account = h.core().list_accounts().unwrap().accounts.remove(0);

    h.core().add_shortcuts(ID_A).unwrap();

    let icons = h.os.icons_written.lock().unwrap().clone();
    assert_eq!(icons.len(), 1);
    let (path, badge, colour) = &icons[0];
    assert_eq!(path.parent().unwrap(), icons_dir(&h));
    assert_eq!(path.extension().unwrap(), "ico");
    assert_eq!((*badge, colour.as_str()), ('W', account.colour.as_str()));
    assert!(exists(path));
    for (_, link) in h.os.links_written.lock().unwrap().iter() {
        assert_eq!(&link.icon, path);
    }
}

#[test]
fn adding_twice_replaces_the_shortcuts_instead_of_duplicating_them() {
    let h = work();

    h.core().add_shortcuts(ID_A).unwrap();
    h.core().add_shortcuts(ID_A).unwrap();

    assert_eq!(std::fs::read_dir(h.fixture.desktop()).unwrap().count(), 1);
    assert_eq!(
        std::fs::read_dir(h.fixture.start_menu()).unwrap().count(),
        1
    );
    assert_eq!(std::fs::read_dir(icons_dir(&h)).unwrap().count(), 1);
}

#[test]
fn adding_for_an_unknown_account_writes_nothing() {
    let h = work();

    let error = h.core().add_shortcuts(ID_B).unwrap_err();

    assert!(matches!(
        error,
        ShortcutError::Account(AccountError::Unknown)
    ));
    assert!(h.os.links_written.lock().unwrap().is_empty());
    assert!(h.os.icons_written.lock().unwrap().is_empty());
}

#[test]
fn adding_for_an_account_whose_folder_is_gone_writes_nothing() {
    let h = work();
    std::fs::remove_dir_all(h.fixture.profile_path(ID_A)).unwrap();

    let error = h.core().add_shortcuts(ID_A).unwrap_err();

    assert!(matches!(error, ShortcutError::FolderMissing));
    assert!(h.os.links_written.lock().unwrap().is_empty());
}

// ---- rename, colour change, delete ----------------------------------------------

fn icon_files(h: &Harness) -> Vec<PathBuf> {
    std::fs::read_dir(icons_dir(h))
        .map(|dir| dir.map(|e| e.unwrap().path()).collect())
        .unwrap_or_default()
}

fn names_in(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .map(|d| {
            d.map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    names
}

#[test]
fn rename_moves_the_shortcuts_to_the_new_name_and_badge() {
    let h = work();
    h.core().add_shortcuts(ID_A).unwrap();
    let old_icon = icon_files(&h).remove(0);

    h.core().rename_account(ID_A, "Home").unwrap();

    assert_eq!(names_in(&h.fixture.desktop()), ["Claude - Home.lnk"]);
    assert_eq!(names_in(&h.fixture.start_menu()), ["Claude - Home.lnk"]);
    let icons = icon_files(&h);
    assert_eq!(icons.len(), 1, "the old badge's icon is gone");
    assert_ne!(icons[0], old_icon);
    let written = h.os.icons_written.lock().unwrap().clone();
    assert_eq!(written.last().unwrap().1, 'H');
    let links = h.os.links_written.lock().unwrap().clone();
    assert_eq!(links.last().unwrap().1.icon, icons[0]);
    assert_eq!(links.last().unwrap().1.args, format!("--account {ID_A}"));
}

#[test]
fn rename_keeping_the_initial_keeps_the_same_icon_file() {
    let h = work();
    h.core().add_shortcuts(ID_A).unwrap();
    let icon = icon_files(&h).remove(0);

    h.core().rename_account(ID_A, "Work 2").unwrap();

    assert_eq!(names_in(&h.fixture.desktop()), ["Claude - Work 2.lnk"]);
    assert_eq!(icon_files(&h), vec![icon.clone()]);
    assert!(!h.os.deleted.lock().unwrap().contains(&icon));
}

#[test]
fn a_new_colour_gives_the_shortcuts_a_new_icon() {
    let h = work();
    h.core().add_shortcuts(ID_A).unwrap();
    let old_icon = icon_files(&h).remove(0);

    h.core().set_account_colour(ID_A, "#ff00aa").unwrap();

    let icons = icon_files(&h);
    assert_eq!(icons.len(), 1);
    assert_ne!(icons[0], old_icon);
    let written = h.os.icons_written.lock().unwrap().clone();
    assert_eq!(written.last().unwrap().2, "#ff00aa");
    // Both links still exist under the same name and now show the new icon.
    assert_eq!(names_in(&h.fixture.desktop()), ["Claude - Work.lnk"]);
    assert_eq!(names_in(&h.fixture.start_menu()), ["Claude - Work.lnk"]);
    let links = h.os.links_written.lock().unwrap().clone();
    assert!(
        links[links.len() - 2..]
            .iter()
            .all(|(_, l)| l.icon == icons[0])
    );
}

#[test]
fn rename_and_colour_create_no_shortcuts_when_there_were_none() {
    let h = work();

    h.core().rename_account(ID_A, "Home").unwrap();
    h.core().set_account_colour(ID_A, "#ff00aa").unwrap();

    assert!(h.os.links_written.lock().unwrap().is_empty());
    assert!(h.os.icons_written.lock().unwrap().is_empty());
    assert!(icon_files(&h).is_empty());
}

#[test]
fn a_shortcut_the_user_deleted_by_hand_is_not_brought_back_by_a_rename() {
    let h = work();
    h.core().add_shortcuts(ID_A).unwrap();
    std::fs::remove_file(desktop_link(&h, "Claude - Work")).unwrap();

    h.core().rename_account(ID_A, "Home").unwrap();

    assert!(names_in(&h.fixture.desktop()).is_empty());
    assert_eq!(names_in(&h.fixture.start_menu()), ["Claude - Home.lnk"]);
}

#[test]
fn delete_removes_both_shortcuts_and_the_icon() {
    let h = work();
    h.core().add_shortcuts(ID_A).unwrap();

    h.core().delete_account(ID_A).unwrap();

    assert!(names_in(&h.fixture.desktop()).is_empty());
    assert!(names_in(&h.fixture.start_menu()).is_empty());
    assert!(icon_files(&h).is_empty());
}

#[test]
fn delete_leaves_other_accounts_shortcuts_alone() {
    let h = work();
    h.fixture.add_profile(ID_B);
    h.core().rename_account(ID_B, "Home").unwrap();
    h.core().add_shortcuts(ID_A).unwrap();
    h.core().add_shortcuts(ID_B).unwrap();

    h.core().delete_account(ID_A).unwrap();

    assert_eq!(names_in(&h.fixture.desktop()), ["Claude - Home.lnk"]);
    assert_eq!(names_in(&h.fixture.start_menu()), ["Claude - Home.lnk"]);
    assert_eq!(icon_files(&h).len(), 1);
}

// ---- clicking a shortcut ---------------------------------------------------------

#[test]
fn a_shortcut_click_starts_an_account_that_is_not_running() {
    let h = work();
    let line = args(&["clawshell.exe", "--account", ID_A]);

    let outcome = h.core().open_requested(&line).unwrap().unwrap();

    assert_eq!(outcome, LaunchOutcome::Started);
    assert_eq!(h.os.launched.lock().unwrap().len(), 1);
}

#[test]
fn a_shortcut_click_focuses_an_account_that_is_already_running() {
    let h = work();
    h.processes
        .0
        .lock()
        .unwrap()
        .insert(h.fixture.profile_path(ID_A));
    let line = args(&["clawshell.exe", "--account", ID_A]);

    let outcome = h.core().open_requested(&line).unwrap().unwrap();

    assert_eq!(outcome, LaunchOutcome::Focused);
    assert!(h.os.launched.lock().unwrap().is_empty());
    assert_eq!(
        h.os.focused.lock().unwrap().clone(),
        vec![h.fixture.profile_path(ID_A)]
    );
}

#[test]
fn a_command_line_without_an_account_is_not_a_shortcut_click() {
    let h = work();

    assert!(h.core().open_requested(&args(&["clawshell.exe"])).is_none());
    assert!(h.os.launched.lock().unwrap().is_empty());
}

#[test]
fn a_shortcut_click_for_a_deleted_account_reports_it() {
    let h = work();
    let line = args(&["clawshell.exe", "--account", ID_B]);

    let outcome = h.core().open_requested(&line).unwrap();

    assert!(matches!(
        outcome,
        Err(LaunchError::Account(AccountError::Unknown))
    ));
}
