mod support;

use clawshell_lib::core::AccountError;
use clawshell_lib::core::launch_options::{EnvVar, LaunchOptions};
use clawshell_lib::core::{LaunchOptionsError, LaunchOutcome};
use support::Harness;

const ID_A: &str = "3f2b8c1e-5a4d-4e7b-9c10-2d6f8a9b0c11";
const ID_B: &str = "a0b1c2d3-e4f5-4a6b-8c7d-9e0f1a2b3c4d";

fn env(name: &str, value: &str) -> EnvVar {
    EnvVar {
        name: name.to_string(),
        value: value.to_string(),
    }
}

fn options(vars: &[(&str, &str)], flags: &[&str]) -> LaunchOptions {
    LaunchOptions {
        env: vars.iter().map(|(n, v)| env(n, v)).collect(),
        flags: flags.iter().map(|f| f.to_string()).collect(),
    }
}

fn strings(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| s.to_string()).collect()
}

fn account(h: &Harness, id: &str) -> clawshell_lib::core::Account {
    h.core()
        .list_accounts()
        .unwrap()
        .accounts
        .into_iter()
        .find(|a| a.id == id)
        .unwrap()
}

#[test]
fn a_launch_carries_the_accounts_flags_after_its_profile_folder_and_sets_its_env() {
    let h = Harness::new();
    let profile = h.fixture.add_profile(ID_A);
    h.fixture.sign_in(ID_A);
    h.core()
        .set_launch_options(
            ID_A,
            options(
                &[("MCP_TOKEN", "s3cret"), ("LOG_LEVEL", "debug")],
                &["--lang=fa", "--enable-logging"],
            ),
        )
        .unwrap();

    assert_eq!(
        h.core().launch_account(ID_A).unwrap(),
        LaunchOutcome::Started
    );

    let specs = h.os.launch_specs.lock().unwrap().clone();
    assert_eq!(specs.len(), 1);
    assert_eq!(
        specs[0].args,
        vec![
            format!("--user-data-dir={}", profile.display()),
            "--lang=fa".to_string(),
            "--enable-logging".to_string(),
        ]
    );
    assert_eq!(
        specs[0].env,
        vec![
            ("MCP_TOKEN".to_string(), "s3cret".to_string()),
            ("LOG_LEVEL".to_string(), "debug".to_string()),
        ]
    );
}

#[test]
fn another_accounts_launch_gets_none_of_them() {
    let h = Harness::new();
    h.fixture.add_profile(ID_A);
    h.fixture.add_profile(ID_B);
    h.core()
        .set_launch_options(ID_A, options(&[("MCP_TOKEN", "s3cret")], &["--lang=fa"]))
        .unwrap();

    h.core().launch_account(ID_B).unwrap();

    let specs = h.os.launch_specs.lock().unwrap().clone();
    assert_eq!(specs.len(), 1);
    assert_eq!(specs[0].args.len(), 1);
    assert!(specs[0].env.is_empty());
}

#[test]
fn options_persist_across_a_restart_and_show_on_the_account() {
    let h = Harness::new();
    h.fixture.add_profile(ID_A);
    let saved = h
        .core()
        .set_launch_options(ID_A, options(&[("LOG_LEVEL", "debug")], &["--lang=fa"]))
        .unwrap();
    assert_eq!(saved.env, vec![env("LOG_LEVEL", "debug")]);

    // A fresh Core over the same machine, as after an app restart.
    let again = account(&h, ID_A);

    assert_eq!(again.env, vec![env("LOG_LEVEL", "debug")]);
    assert_eq!(again.flags, strings(&["--lang=fa"]));
}

#[test]
fn saving_replaces_the_old_options_and_empty_removes_them() {
    let h = Harness::new();
    h.fixture.add_profile(ID_A);
    h.core()
        .set_launch_options(
            ID_A,
            options(&[("A", "1"), ("B", "2")], &["--one", "--two"]),
        )
        .unwrap();
    h.core()
        .set_launch_options(ID_A, options(&[("B", "3")], &["--two"]))
        .unwrap();
    assert_eq!(account(&h, ID_A).env, vec![env("B", "3")]);
    assert_eq!(account(&h, ID_A).flags, strings(&["--two"]));

    h.core()
        .set_launch_options(ID_A, options(&[], &[]))
        .unwrap();

    assert!(account(&h, ID_A).env.is_empty());
    assert!(account(&h, ID_A).flags.is_empty());
}

#[test]
fn options_belong_to_the_named_account_only() {
    let h = Harness::new();
    h.fixture.add_profile(ID_A);
    h.fixture.add_profile(ID_B);

    h.core()
        .set_launch_options(ID_A, options(&[("A", "1")], &["--x"]))
        .unwrap();

    assert!(account(&h, ID_B).env.is_empty());
    assert!(account(&h, ID_B).flags.is_empty());
}

#[test]
fn options_for_an_unknown_account_are_refused() {
    let h = Harness::new();
    h.fixture.add_profile(ID_A);

    let result = h
        .core()
        .set_launch_options(ID_B, options(&[("A", "1")], &[]));

    assert!(matches!(
        result,
        Err(LaunchOptionsError::Account(AccountError::Unknown))
    ));
}

#[test]
fn every_launch_path_applies_them_a_shortcut_launch_included() {
    let h = Harness::new();
    h.fixture.add_profile(ID_A);
    h.core()
        .set_launch_options(ID_A, options(&[("A", "1")], &["--lang=fa"]))
        .unwrap();
    h.core().add_shortcuts(ID_A).unwrap();
    let shortcut_args = h.os.links_written.lock().unwrap()[0].1.args.clone();
    let mut line = vec!["clawshell.exe".to_string()];
    line.extend(shortcut_args.split(' ').map(str::to_string));

    h.core().open_requested(&line).unwrap().unwrap();

    let specs = h.os.launch_specs.lock().unwrap().clone();
    assert_eq!(specs.len(), 1);
    assert!(specs[0].args.contains(&"--lang=fa".to_string()));
    assert_eq!(specs[0].env, vec![("A".to_string(), "1".to_string())]);
}

// ---- validation --------------------------------------------------------------

fn rejected(name: &str) -> bool {
    let h = Harness::new();
    h.fixture.add_profile(ID_A);
    let before = options(&[("KEEP", "1")], &[]);
    h.core().set_launch_options(ID_A, before).unwrap();

    let result = h
        .core()
        .set_launch_options(ID_A, options(&[(name, "v")], &[]));

    let refused = matches!(result, Err(LaunchOptionsError::InvalidEnvName));
    // A refused save changes nothing.
    assert_eq!(account(&h, ID_A).env, vec![env("KEEP", "1")]);
    refused
}

#[test]
fn env_names_may_only_hold_letters_digits_and_underscore() {
    for bad in [
        "", " ", "A B", "A-B", "A=B", "A.B", "NÄME", "A\u{0}B", "PATH ", "$HOME",
    ] {
        assert!(rejected(bad), "{bad:?} should be refused");
    }
}

#[test]
fn env_names_may_not_start_with_a_digit() {
    assert!(rejected("1ABC"));
    assert!(rejected("9"));
}

#[test]
fn ordinary_env_names_are_accepted() {
    let h = Harness::new();
    h.fixture.add_profile(ID_A);

    let saved = h
        .core()
        .set_launch_options(
            ID_A,
            options(&[("_", "1"), ("a1_B2", ""), ("MCP_TOKEN", "x")], &[]),
        )
        .unwrap();

    assert_eq!(saved.env.len(), 3);
    // An empty value is a value.
    assert_eq!(saved.env[1], env("a1_B2", ""));
}

#[test]
fn a_name_given_twice_is_refused_whatever_its_case() {
    let h = Harness::new();
    h.fixture.add_profile(ID_A);

    for pair in [[("A", "1"), ("A", "2")], [("Path", "1"), ("PATH", "2")]] {
        let result = h.core().set_launch_options(ID_A, options(&pair, &[]));
        assert!(matches!(result, Err(LaunchOptionsError::DuplicateEnvName)));
    }
    assert!(account(&h, ID_A).env.is_empty());
}

#[test]
fn a_value_with_a_nul_character_is_refused_without_echoing_it() {
    let h = Harness::new();
    h.fixture.add_profile(ID_A);

    let err = h
        .core()
        .set_launch_options(ID_A, options(&[("TOKEN", "hunter2\u{0}x")], &[]))
        .unwrap_err();

    assert!(matches!(err, LaunchOptionsError::InvalidEnvValue));
    assert!(!format!("{err:?}").contains("hunter2"));
}

#[test]
fn flags_are_trimmed_and_blank_ones_dropped() {
    let h = Harness::new();
    h.fixture.add_profile(ID_A);

    let saved = h
        .core()
        .set_launch_options(ID_A, options(&[], &["  --lang=fa ", "", "   ", "--v=1"]))
        .unwrap();

    assert_eq!(saved.flags, strings(&["--lang=fa", "--v=1"]));
    assert_eq!(account(&h, ID_A).flags, strings(&["--lang=fa", "--v=1"]));
}

#[test]
fn a_flag_may_not_redirect_the_profile_folder() {
    let h = Harness::new();
    h.fixture.add_profile(ID_A);

    for flag in [
        "--user-data-dir=C:/elsewhere",
        "--user-data-dir",
        " --USER-DATA-DIR=x",
    ] {
        let result = h.core().set_launch_options(ID_A, options(&[], &[flag]));
        assert!(
            matches!(result, Err(LaunchOptionsError::ReservedFlag)),
            "{flag}"
        );
    }
    assert!(account(&h, ID_A).flags.is_empty());
}

#[test]
fn a_flag_with_spaces_stays_one_argument() {
    let h = Harness::new();
    h.fixture.add_profile(ID_A);
    h.core()
        .set_launch_options(ID_A, options(&[], &["--log-file=C:/My Logs/claude.log"]))
        .unwrap();

    h.core().launch_account(ID_A).unwrap();

    let specs = h.os.launch_specs.lock().unwrap().clone();
    assert_eq!(specs[0].args[1], "--log-file=C:/My Logs/claude.log");
}

#[test]
fn a_launch_spec_never_shows_env_values_when_printed() {
    let h = Harness::new();
    h.fixture.add_profile(ID_A);
    h.core()
        .set_launch_options(ID_A, options(&[("TOKEN", "hunter2")], &[]))
        .unwrap();
    h.core().launch_account(ID_A).unwrap();

    let spec = h.os.launch_specs.lock().unwrap()[0].clone();
    let shown = format!("{spec:?} {:?}", account(&h, ID_A));

    assert!(shown.contains("TOKEN"));
    assert!(!shown.contains("hunter2"));
}

#[test]
fn a_missing_account_can_still_have_options_edited() {
    let h = Harness::new();
    h.fixture.add_profile(ID_A);
    h.core().list_accounts().unwrap();
    std::fs::remove_dir_all(h.fixture.profile_path(ID_A)).unwrap();

    let saved = h
        .core()
        .set_launch_options(ID_A, options(&[("A", "1")], &[]));

    assert!(saved.is_ok());
}

#[test]
fn records_saved_before_options_existed_load_with_none() {
    let h = Harness::new();
    h.fixture.add_profile(ID_A);
    let folder = h.fixture.local_app_data().join("clawshell");
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(
        folder.join("accounts.json"),
        format!(r##"{{"accounts":[{{"id":"{ID_A}","label":"Work","colour":"#2563eb"}}]}}"##),
    )
    .unwrap();

    let a = account(&h, ID_A);

    assert_eq!(a.label, "Work");
    assert!(a.env.is_empty() && a.flags.is_empty());
}
