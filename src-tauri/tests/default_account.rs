mod support;

use clawshell_lib::core::launch_options::{EnvVar, LaunchOptions};
use clawshell_lib::core::ports::ClaudeInstall;
use clawshell_lib::core::routing::Target;
use clawshell_lib::core::{
    Account, ColourError, DeleteError, LaunchOutcome, ShortcutError, Status,
};
use std::fs;
use std::path::PathBuf;
use support::{Harness, add_default};

const DEFAULT: &str = "default";
const LINK: &str = "claude://login/callback?code=one-time-code";

fn default_account(h: &Harness) -> Account {
    h.core().list_accounts().unwrap().accounts.remove(0)
}

fn run_default(h: &Harness) {
    h.processes
        .0
        .lock()
        .unwrap()
        .insert(h.fixture.default_profile_path());
}

// Listing

#[test]
fn claudes_own_profile_is_listed_first_as_claude() {
    let h = Harness::new();
    h.fixture.sign_in_default();
    add_default(&h.core());

    let list = h.core().list_accounts().unwrap();

    let first = &list.accounts[0];
    assert_eq!(first.id, DEFAULT);
    assert_eq!(first.label, "Claude");
    assert!(first.is_default);
    assert!(list.accounts[1..].iter().all(|a| !a.is_default));
}

#[test]
fn it_is_listed_signed_out_before_claude_has_ever_run() {
    let h = Harness::new();

    let account = default_account(&h);

    assert!(account.is_default);
    assert_eq!(account.status, Status::SignedOut);
}

#[test]
fn it_is_idle_once_signed_in_and_running_while_claude_uses_it() {
    let h = Harness::new();
    h.fixture.sign_in_default();
    assert_eq!(default_account(&h).status, Status::Idle);

    run_default(&h);

    let account = default_account(&h);
    assert_eq!(account.status, Status::Running);
    assert!(account.running);
}

fn listed_ids(h: &Harness) -> Vec<String> {
    h.core()
        .list_accounts()
        .unwrap()
        .accounts
        .into_iter()
        .map(|a| a.id)
        .collect()
}

#[test]
fn it_is_listed_where_it_was_dropped_and_it_survives_a_restart() {
    let h = Harness::new();
    let added = add_default(&h.core());

    h.core()
        .reorder_accounts(&[added.id.clone(), DEFAULT.to_string()])
        .unwrap();

    assert_eq!(listed_ids(&h), vec![added.id, DEFAULT.to_string()]);
}

#[test]
fn an_older_records_file_lists_it_first_until_it_is_moved() {
    let h = Harness::new();
    let added = add_default(&h.core());
    // Saved while the default account was always listed first, wherever its record sat.
    fs::write(
        h.fixture.local_app_data().join("clawshell/accounts.json"),
        format!(
            r#"{{"ordered":true,"accounts":[{{"id":"{}","label":"Work"}},{{"id":"{DEFAULT}","label":"Claude"}}]}}"#,
            added.id
        ),
    )
    .unwrap();

    assert_eq!(listed_ids(&h), vec![DEFAULT.to_string(), added.id.clone()]);

    h.core()
        .reorder_accounts(&[added.id.clone(), DEFAULT.to_string()])
        .unwrap();

    assert_eq!(listed_ids(&h), vec![added.id, DEFAULT.to_string()]);
}

#[test]
fn the_store_build_keeps_it_under_claude_data_by_package_family() {
    let h = Harness::new();
    *h.os.install.lock().unwrap() = Some(ClaudeInstall {
        exe: PathBuf::from(r"C:\Program Files\WindowsApps\Claude_1.0_x64__abc\app\Claude.exe"),
        root: PathBuf::from(r"C:\Program Files\WindowsApps\Claude_1.0_x64__abc"),
    });
    let store = h.fixture.local_app_data().join("Claude-Data/claude_abc");
    fs::create_dir_all(&store).unwrap();
    fs::write(
        store.join("config.json"),
        r#"{"windowSizeWasSignedIn": true}"#,
    )
    .unwrap();

    assert_eq!(default_account(&h).status, Status::Idle);
}

#[test]
fn older_claude_versions_keep_it_in_roaming_app_data() {
    let h = Harness::new();
    let legacy = h.fixture.roaming_app_data().join("Claude");
    fs::create_dir_all(&legacy).unwrap();
    fs::write(
        legacy.join("config.json"),
        r#"{"windowSizeWasSignedIn": true}"#,
    )
    .unwrap();

    assert_eq!(default_account(&h).status, Status::Idle);
}

// What can be changed

#[test]
fn it_can_be_renamed_and_the_name_survives_a_restart() {
    let h = Harness::new();

    h.core().rename_account(DEFAULT, "Personal").unwrap();

    assert_eq!(default_account(&h).label, "Personal");
}

#[test]
fn it_keeps_its_launch_options() {
    let h = Harness::new();
    let options = LaunchOptions {
        env: vec![EnvVar {
            name: "HTTPS_PROXY".into(),
            value: "http://proxy:8080".into(),
        }],
        flags: vec!["--disable-gpu".into()],
    };

    h.core()
        .set_launch_options(DEFAULT, options.clone())
        .unwrap();

    let account = default_account(&h);
    assert_eq!(account.env, options.env);
    assert_eq!(account.flags, options.flags);
}

#[test]
fn it_has_no_badge_colour_to_change() {
    let h = Harness::new();

    let refused = h.core().set_account_colour(DEFAULT, "#2563eb");

    assert!(matches!(refused, Err(ColourError::DefaultAccount)));
}

#[test]
fn it_can_never_be_deleted_and_its_folder_is_left_alone() {
    let h = Harness::new();
    h.fixture.sign_in_default();

    let refused = h.core().delete_account(DEFAULT);

    assert!(matches!(refused, Err(DeleteError::DefaultAccount)));
    assert!(
        h.fixture
            .default_profile_path()
            .join("config.json")
            .exists()
    );
    assert_eq!(default_account(&h).id, DEFAULT);
}

#[test]
fn it_gets_no_shortcuts() {
    let h = Harness::new();
    h.fixture.sign_in_default();

    let refused = h.core().add_shortcuts(DEFAULT);

    assert!(matches!(refused, Err(ShortcutError::DefaultAccount)));
    assert!(h.os.links_written.lock().unwrap().is_empty());
}

#[test]
fn a_rename_writes_no_shortcuts() {
    let h = Harness::new();

    h.core().rename_account(DEFAULT, "Personal").unwrap();

    assert!(h.os.links_written.lock().unwrap().is_empty());
}

// Launching

#[test]
fn launching_starts_claude_without_a_profile_folder_of_its_own() {
    let h = Harness::new();
    h.fixture.sign_in_default();
    h.core()
        .set_launch_options(
            DEFAULT,
            LaunchOptions {
                env: vec![EnvVar {
                    name: "HTTPS_PROXY".into(),
                    value: "http://proxy:8080".into(),
                }],
                flags: vec!["--disable-gpu".into()],
            },
        )
        .unwrap();

    let outcome = h.core().launch_account(DEFAULT).unwrap();

    assert_eq!(outcome, LaunchOutcome::Started);
    let specs = h.os.launch_specs.lock().unwrap();
    assert_eq!(specs.len(), 1);
    assert_eq!(specs[0].exe, PathBuf::from("C:/Claude/app/Claude.exe"));
    assert_eq!(specs[0].args, vec!["--disable-gpu".to_string()]);
    assert_eq!(
        specs[0].env,
        vec![("HTTPS_PROXY".to_string(), "http://proxy:8080".to_string())]
    );
}

#[test]
fn launching_it_while_it_runs_focuses_its_window() {
    let h = Harness::new();
    h.fixture.sign_in_default();
    run_default(&h);

    let outcome = h.core().launch_account(DEFAULT).unwrap();

    assert_eq!(outcome, LaunchOutcome::Focused);
    assert_eq!(
        h.os.focused.lock().unwrap().clone(),
        vec![h.fixture.default_profile_path()]
    );
    assert!(h.os.launched.lock().unwrap().is_empty());
}

#[test]
fn its_window_keeps_claudes_own_taskbar_button() {
    let h = Harness::new();
    h.fixture.sign_in_default();
    run_default(&h);

    h.core().sync_taskbar_identities().unwrap();
    h.core().launch_account(DEFAULT).unwrap();

    assert!(h.os.taskbar.lock().unwrap().is_empty());
}

// Sign-in routing

#[test]
fn a_sign_in_link_reaches_it_when_it_was_launched_signed_out() {
    let h = Harness::new();
    let core = h.core();
    core.set_sign_in_routing(true).unwrap();
    add_default(&core);
    core.launch_account(DEFAULT).unwrap();

    let target = core.route_link(LINK).unwrap();

    assert_eq!(target, Target::Account(DEFAULT.to_string()));
    let forwarded = h.os.forwarded.lock().unwrap();
    let last = forwarded.last().unwrap();
    assert!(
        last.args.iter().all(|a| !a.starts_with("--user-data-dir")),
        "the default account opens Claude's own profile"
    );
}
