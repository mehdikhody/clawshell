mod support;

use clawshell_lib::core::AccountError;
use clawshell_lib::core::badge::{PALETTE, app_user_model_id, badge_letter, is_valid_colour};
use support::{Added, Harness, add_default};

const ID_A: &str = "3f2b8c1e-5a4d-4e7b-9c10-2d6f8a9b0c11";
const ID_B: &str = "a0b1c2d3-e4f5-4a6b-8c7d-9e0f1a2b3c4d";

// ---- pure: badge letter -------------------------------------------------------

#[test]
fn the_badge_letter_is_the_first_letter_of_the_label_in_capitals() {
    assert_eq!(badge_letter("Account 2"), 'A');
    assert_eq!(badge_letter("work"), 'W');
    assert_eq!(badge_letter("  éclair"), 'É');
}

#[test]
fn the_badge_letter_skips_leading_symbols_and_falls_back_to_a_question_mark() {
    assert_eq!(badge_letter("(Work)"), 'W');
    assert_eq!(badge_letter("7th floor"), '7');
    assert_eq!(badge_letter("***"), '?');
    assert_eq!(badge_letter(""), '?');
}

// ---- pure: taskbar identity ---------------------------------------------------

#[test]
fn the_app_user_model_id_names_the_account_letter_and_colour() {
    assert_eq!(
        app_user_model_id(ID_A, 'W', "#2563eb"),
        format!("clawshell.{ID_A}.57.2563eb")
    );
}

#[test]
fn a_new_letter_or_colour_is_a_new_identity() {
    let base = app_user_model_id(ID_A, 'W', "#2563eb");

    assert_ne!(base, app_user_model_id(ID_A, 'X', "#2563eb"));
    assert_ne!(base, app_user_model_id(ID_A, 'W', "#dc2626"));
    assert_ne!(base, app_user_model_id(ID_B, 'W', "#2563eb"));
    assert_eq!(base, app_user_model_id(ID_A, 'W', "#2563eb"));
}

#[test]
fn an_app_user_model_id_stays_within_the_windows_limit() {
    let id = app_user_model_id(ID_A, '\u{10FFFF}', "#ffffff");

    assert!(id.len() <= 128);
    assert!(id.is_ascii());
}

#[test]
fn only_seven_character_hex_colours_are_valid() {
    assert!(is_valid_colour("#2563eb"));
    assert!(is_valid_colour("#ABCDEF"));
    for bad in ["", "2563eb", "#2563e", "#2563ebf", "#25g3eb", "red"] {
        assert!(!is_valid_colour(bad), "{bad}");
    }
}

#[test]
fn the_palette_is_distinct_valid_colours() {
    let mut sorted: Vec<_> = PALETTE.to_vec();
    sorted.sort();
    sorted.dedup();
    assert_eq!(sorted.len(), PALETTE.len());
    assert!(PALETTE.len() >= 8);
    assert!(PALETTE.iter().all(|c| is_valid_colour(c)));
}

// ---- colour assignment and persistence ---------------------------------------

#[test]
fn new_accounts_get_distinct_colours() {
    let h = Harness::new();
    let core = h.core();

    let colours: Vec<_> = (0..PALETTE.len())
        .map(|_| add_default(&core).colour)
        .collect();

    let mut unique = colours.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(unique.len(), PALETTE.len());
}

#[test]
fn colours_are_reused_only_once_the_palette_runs_out() {
    let h = Harness::new();
    let core = h.core();
    for _ in 0..PALETTE.len() {
        add_default(&core);
    }

    let next = add_default(&core);

    assert!(PALETTE.contains(&next.colour.as_str()));
}

#[test]
fn an_accounts_colour_survives_a_restart() {
    let h = Harness::new();
    let added = add_default(&h.core());

    // A fresh Core over the same machine, as after an app restart.
    let again = h.core().list_accounts().unwrap().added();

    assert_eq!(again.len(), 1);
    assert_eq!(again[0].colour, added.colour);
}

#[test]
fn the_list_offers_the_palette_to_pick_from() {
    let h = Harness::new();

    let list = h.core().list_accounts().unwrap();

    assert_eq!(list.palette, PALETTE.to_vec());
}

#[test]
fn an_existing_folder_with_no_recorded_colour_gets_one_and_keeps_it() {
    let h = Harness::new();
    h.fixture.add_profile(ID_A);
    h.fixture.add_profile(ID_B);

    let first = h.core().list_accounts().unwrap().added();
    let second = h.core().list_accounts().unwrap().added();

    assert_ne!(first[0].colour, first[1].colour);
    assert_eq!(first[0].colour, second[0].colour);
    assert_eq!(first[1].colour, second[1].colour);
}

#[test]
fn records_saved_before_colours_existed_are_given_colours() {
    let h = Harness::new();
    h.fixture.add_profile(ID_A);
    let dir = h.fixture.local_app_data().join("clawshell");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("accounts.json"),
        format!(r#"{{"accounts":[{{"id":"{ID_A}","label":"Work"}}]}}"#),
    )
    .unwrap();

    let accounts = h.core().list_accounts().unwrap().added();

    assert_eq!(accounts[0].label, "Work");
    assert!(is_valid_colour(&accounts[0].colour));
}

#[test]
fn the_picker_changes_the_colour_and_it_persists() {
    let h = Harness::new();
    h.fixture.add_profile(ID_A);
    h.fixture.add_profile(ID_B);
    let before = h.core().list_accounts().unwrap().added();

    let changed = h.core().set_account_colour(ID_A, "#123456").unwrap();

    assert_eq!(changed.colour, "#123456");
    let after = h.core().list_accounts().unwrap().added();
    assert_eq!(after[0].colour, "#123456");
    assert_eq!(after[1].colour, before[1].colour);
}

#[test]
fn a_colour_is_stored_in_lower_case() {
    let h = Harness::new();
    h.fixture.add_profile(ID_A);

    let changed = h.core().set_account_colour(ID_A, "#ABCDEF").unwrap();

    assert_eq!(changed.colour, "#abcdef");
}

#[test]
fn an_invalid_colour_or_unknown_account_is_refused_and_changes_nothing() {
    use clawshell_lib::core::ColourError;
    let h = Harness::new();
    h.fixture.add_profile(ID_A);
    let before = h.core().list_accounts().unwrap().added()[0].colour.clone();

    assert!(matches!(
        h.core().set_account_colour(ID_A, "blue").unwrap_err(),
        ColourError::InvalidColour
    ));
    assert!(matches!(
        h.core().set_account_colour(ID_B, "#123456").unwrap_err(),
        ColourError::Account(AccountError::Unknown)
    ));
    assert_eq!(h.core().list_accounts().unwrap().added()[0].colour, before);
}

#[test]
fn rows_carry_the_badge_letter_and_a_rename_updates_it() {
    let h = Harness::new();
    h.fixture.add_profile(ID_A);
    assert_eq!(h.core().list_accounts().unwrap().added()[0].badge, 'A');

    let renamed = h.core().rename_account(ID_A, "work").unwrap();

    assert_eq!(renamed.badge, 'W');
    assert_eq!(h.core().list_accounts().unwrap().added()[0].badge, 'W');
}

// ---- taskbar identity of running windows -------------------------------------

fn run(h: &Harness, id: &str) {
    h.fixture.sign_in(id);
    h.processes
        .0
        .lock()
        .unwrap()
        .insert(h.fixture.profile_path(id));
}

fn stop(h: &Harness, id: &str) {
    h.processes
        .0
        .lock()
        .unwrap()
        .remove(&h.fixture.profile_path(id));
}

fn applied(h: &Harness) -> Vec<(String, String)> {
    h.os.taskbar
        .lock()
        .unwrap()
        .iter()
        .map(|i| (i.aumid.clone(), i.colour.clone()))
        .collect()
}

#[test]
fn each_running_account_gets_its_own_identity() {
    let h = Harness::new();
    h.fixture.add_profile(ID_A);
    h.fixture.add_profile(ID_B);
    run(&h, ID_A);
    run(&h, ID_B);
    let core = h.core();

    core.sync_taskbar_identities().unwrap();

    let calls = h.os.taskbar.lock().unwrap().clone();
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0].profile, h.fixture.profile_path(ID_A));
    assert_eq!(calls[1].profile, h.fixture.profile_path(ID_B));
    assert_ne!(calls[0].aumid, calls[1].aumid);
    assert_eq!(calls[0].badge, 'A');
    let accounts = core.list_accounts().unwrap().added();
    assert_eq!(calls[0].colour, accounts[0].colour);
    assert_eq!(
        calls[0].aumid,
        app_user_model_id(ID_A, 'A', &accounts[0].colour)
    );
}

#[test]
fn accounts_that_are_not_running_are_left_alone() {
    let h = Harness::new();
    h.fixture.add_profile(ID_A);
    h.fixture.sign_in(ID_A);

    h.core().sync_taskbar_identities().unwrap();

    assert!(applied(&h).is_empty());
}

#[test]
fn an_unchanged_identity_is_applied_only_once() {
    let h = Harness::new();
    h.fixture.add_profile(ID_A);
    run(&h, ID_A);
    let core = h.core();

    core.sync_taskbar_identities().unwrap();
    core.sync_taskbar_identities().unwrap();
    core.sync_taskbar_identities().unwrap();

    assert_eq!(applied(&h).len(), 1);
}

#[test]
fn a_new_colour_gives_the_running_window_a_new_identity() {
    let h = Harness::new();
    h.fixture.add_profile(ID_A);
    run(&h, ID_A);
    let core = h.core();
    core.sync_taskbar_identities().unwrap();

    core.set_account_colour(ID_A, "#123456").unwrap();

    let calls = applied(&h);
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[1].0, app_user_model_id(ID_A, 'A', "#123456"));
    assert_eq!(calls[1].1, "#123456");
    assert_ne!(calls[0].0, calls[1].0);
}

#[test]
fn renaming_to_a_new_initial_gives_the_running_window_a_new_identity() {
    let h = Harness::new();
    h.fixture.add_profile(ID_A);
    run(&h, ID_A);
    let core = h.core();
    core.sync_taskbar_identities().unwrap();

    core.rename_account(ID_A, "Work").unwrap();

    let calls = h.os.taskbar.lock().unwrap().clone();
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[1].badge, 'W');
    assert_ne!(calls[0].aumid, calls[1].aumid);
}

#[test]
fn renaming_within_the_same_initial_keeps_the_identity() {
    let h = Harness::new();
    h.fixture.add_profile(ID_A);
    run(&h, ID_A);
    let core = h.core();
    core.sync_taskbar_identities().unwrap();

    core.rename_account(ID_A, "Another").unwrap();

    assert_eq!(applied(&h).len(), 1);
}

#[test]
fn a_rename_or_colour_change_of_an_idle_account_touches_no_window() {
    let h = Harness::new();
    h.fixture.add_profile(ID_A);
    let core = h.core();

    core.rename_account(ID_A, "Work").unwrap();
    core.set_account_colour(ID_A, "#123456").unwrap();

    assert!(applied(&h).is_empty());
}

#[test]
fn a_restarted_account_is_given_its_identity_again() {
    let h = Harness::new();
    h.fixture.add_profile(ID_A);
    run(&h, ID_A);
    let core = h.core();
    core.sync_taskbar_identities().unwrap();

    stop(&h, ID_A);
    core.sync_taskbar_identities().unwrap();
    run(&h, ID_A);
    core.sync_taskbar_identities().unwrap();

    assert_eq!(applied(&h).len(), 2);
}

#[test]
fn a_failed_apply_is_retried_on_the_next_tick() {
    let h = Harness::new();
    h.fixture.add_profile(ID_A);
    run(&h, ID_A);
    let core = h.core();
    *h.os.taskbar_fails.lock().unwrap() = true;
    core.sync_taskbar_identities().unwrap();
    assert!(applied(&h).is_empty());

    *h.os.taskbar_fails.lock().unwrap() = false;
    core.sync_taskbar_identities().unwrap();

    assert_eq!(applied(&h).len(), 1);
}

#[test]
fn launching_asks_for_the_badge_before_the_window_first_shows() {
    let h = Harness::new();
    h.fixture.add_profile(ID_A);
    h.fixture.sign_in(ID_A);
    let core = h.core();

    core.launch_account(ID_A).unwrap();

    // Claude is not even in the process table yet; the port is already told.
    let calls = h.os.taskbar.lock().unwrap().clone();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].profile, h.fixture.profile_path(ID_A));
    assert_eq!(calls[0].badge, 'A');
}

#[test]
fn a_new_account_is_badged_as_its_sign_in_window_opens() {
    let h = Harness::new();
    let core = h.core();
    let added = add_default(&core);

    core.launch_account(&added.id).unwrap();

    let calls = h.os.taskbar.lock().unwrap().clone();
    assert_eq!(calls.len(), 1);
    assert_eq!(
        calls[0].aumid,
        app_user_model_id(&added.id, added.badge, &added.colour)
    );
}

#[test]
fn focusing_an_open_account_does_not_reapply_the_identity() {
    let h = Harness::new();
    h.fixture.add_profile(ID_A);
    run(&h, ID_A);

    h.core().launch_account(ID_A).unwrap();

    assert!(applied(&h).is_empty());
}

// ---- pure: the badge icon -----------------------------------------------------

use clawshell_lib::core::badge::{
    BASE_SIDE, plain_base, render_badge, resample, rgb, rounded_square,
};

#[test]
fn a_colour_splits_into_red_green_and_blue() {
    assert_eq!(rgb("#2563eb"), Some([0x25, 0x63, 0xeb]));
    assert_eq!(rgb("#FF0080"), Some([255, 0, 128]));
    assert_eq!(rgb("blue"), None);
}

fn pixel(icon: &[u8], size: usize, x: usize, y: usize) -> [u8; 4] {
    let i = (y * size + x) * 4;
    [icon[i], icon[i + 1], icon[i + 2], icon[i + 3]]
}

#[test]
fn a_rounded_square_is_see_through_in_the_corners() {
    let size = 32;
    let square = rounded_square(size, [0xeb, 0x63, 0x25]);

    assert_eq!(square.len(), size * size * 4);
    assert_eq!(pixel(&square, size, 16, 16), [0x25, 0x63, 0xeb, 255]);
    assert_eq!(pixel(&square, size, 0, 0)[3], 0);
    assert_eq!(pixel(&square, size, 31, 31)[3], 0);
    assert_eq!(pixel(&square, size, 16, 0)[3], 255);
}

#[test]
fn the_label_is_the_colour_and_the_base_shows_around_it() {
    let size = 32;
    let base = rounded_square(size, [0xeb, 0x63, 0x25]);
    let icon = render_badge(size, [0x25, 0x63, 0xeb], &vec![0; size * size], &base);

    // The disc's centre, bottom-right, is its colour, opaque.
    assert_eq!(pixel(&icon, size, 22, 22), [0xeb, 0x63, 0x25, 255]);
    // Upper middle is outside the disc: the base as it was.
    assert_eq!(pixel(&icon, size, 16, 8), pixel(&base, size, 16, 8));
    assert_eq!(pixel(&icon, size, 0, 0)[3], 0);
}

#[test]
fn the_letter_is_drawn_white_over_the_label() {
    let size = 16;
    let mut coverage = vec![0u8; size * size];
    coverage[11 * size + 11] = 255;
    coverage[11 * size + 12] = 128;
    let base = plain_base_at(size);

    let icon = render_badge(size, [0x25, 0x63, 0xeb], &coverage, &base);

    assert_eq!(pixel(&icon, size, 11, 11), [255, 255, 255, 255]);
    // Half inked: halfway between the disc colour and white.
    assert_eq!(pixel(&icon, size, 12, 11), [245, 177, 146, 255]);
    // No ink: the disc colour itself.
    assert_eq!(pixel(&icon, size, 10, 11), [0xeb, 0x63, 0x25, 255]);
}

fn plain_base_at(size: usize) -> Vec<u8> {
    resample(&plain_base(), BASE_SIDE, size)
}

#[test]
fn a_base_is_resampled_to_any_size_and_keeps_its_colour() {
    let big = plain_base();
    let small = resample(&big, BASE_SIDE, 16);

    assert_eq!(small.len(), 16 * 16 * 4);
    assert_eq!(pixel(&small, 16, 8, 8), [0x57, 0x77, 0xd9, 255]);
    assert!(
        pixel(&small, 16, 0, 0)[3] < 64,
        "the corner is mostly outside"
    );
}
