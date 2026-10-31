//! Finding Claude Desktop and telling its processes from everything else.

mod support;

use clawshell_lib::core::install::detect_install;
use clawshell_lib::core::ports::{ClaudeInstall, ProcessInfo, ProcessInspector, Roots};
use clawshell_lib::core::process::DesktopProcesses;
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use support::{FakeOs, FakeProcessLister};

// ---- detection ---------------------------------------------------------------

fn touch(path: &std::path::Path) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, "").unwrap();
}

#[test]
fn finds_the_store_build_through_its_package_folder() {
    let machine = support::Fixture::new();
    let package = machine.add_folder("WindowsAppsStandIn/Claude_1.0_x64__abc");
    touch(&package.join("app/Claude.exe"));

    let install = detect_install(&machine.local_app_data(), Some(&package)).unwrap();

    assert_eq!(install.exe, package.join("app/Claude.exe"));
    assert_eq!(install.root, package);
}

#[test]
fn finds_the_installer_build_by_its_newest_app_folder() {
    let machine = support::Fixture::new();
    let root = machine.local_app_data().join("AnthropicClaude");
    touch(&root.join("app-1.9.0/claude.exe"));
    touch(&root.join("app-1.10.2/claude.exe"));
    touch(&root.join("app-1.2.0/claude.exe"));

    let install = detect_install(&machine.local_app_data(), None).unwrap();

    assert_eq!(install.exe, root.join("app-1.10.2/claude.exe"));
    assert_eq!(install.root, root);
}

#[test]
fn a_store_package_without_the_exe_falls_back_to_the_installer_build() {
    let machine = support::Fixture::new();
    let package = machine.add_folder("empty-package");
    let root = machine.local_app_data().join("AnthropicClaude");
    touch(&root.join("app-1.0.0/claude.exe"));

    let install = detect_install(&machine.local_app_data(), Some(&package)).unwrap();

    assert_eq!(install.root, root);
}

#[test]
fn the_store_build_wins_when_both_are_installed() {
    let machine = support::Fixture::new();
    let package = machine.add_folder("pkg");
    touch(&package.join("app/Claude.exe"));
    touch(
        &machine
            .local_app_data()
            .join("AnthropicClaude/app-1.0.0/claude.exe"),
    );

    let install = detect_install(&machine.local_app_data(), Some(&package)).unwrap();

    assert_eq!(install.root, package);
}

#[test]
fn reports_nothing_when_claude_is_not_installed() {
    let machine = support::Fixture::new();
    machine.add_folder("Claude-Data"); // user data is not an install

    assert_eq!(detect_install(&machine.local_app_data(), None), None);
}

// ---- running check -----------------------------------------------------------

const ROOT: &str = r"C:\Program Files\WindowsApps\Claude_1.0_x64__abc";

fn inspector(procs: Vec<ProcessInfo>) -> DesktopProcesses {
    let lister = Arc::new(FakeProcessLister::default());
    *lister.0.lock().unwrap() = procs;
    let os = Arc::new(FakeOs::default());
    *os.install.lock().unwrap() = Some(ClaudeInstall {
        exe: PathBuf::from(format!(r"{ROOT}\app\Claude.exe")),
        root: PathBuf::from(ROOT),
    });
    DesktopProcesses::new(lister, os, roots())
}

/// Folders that hold no Claude data, so the default profile is where current
/// Claude would create it: `Claude-Data` under the package family name.
fn roots() -> Roots {
    Roots {
        local_app_data: PathBuf::from(r"C:\Users\me\AppData\Local"),
        roaming_app_data: PathBuf::from(r"C:\Users\me\AppData\Roaming"),
        home: PathBuf::from(r"C:\Users\me"),
    }
}

const DEFAULT_PROFILE: &str = r"C:\Users\me\AppData\Local\Claude-Data\claude_abc";

fn desktop(pid: u32, args: &[&str]) -> ProcessInfo {
    ProcessInfo {
        pid,
        exe: Some(PathBuf::from(format!(r"{ROOT}\app\Claude.exe"))),
        args: args.iter().map(|a| a.to_string()).collect(),
    }
}

fn set(paths: &[&str]) -> HashSet<PathBuf> {
    paths.iter().map(PathBuf::from).collect()
}

const PROFILE: &str = r"C:\Users\me\AppData\Local\clawshell-3f2b8c1e-5a4d-4e7b-9c10-2d6f8a9b0c11";

#[test]
fn desktop_processes_mark_their_profile_running_whatever_their_role() {
    let user_dir = format!("--user-data-dir={PROFILE}");
    let procs = inspector(vec![
        desktop(1, &[&user_dir]),
        desktop(2, &["--type=renderer", &user_dir]),
        desktop(3, &["--type=gpu-process", &user_dir]),
    ]);

    assert_eq!(procs.running_profiles(), set(&[PROFILE]));
}

#[test]
fn accepts_a_user_data_dir_given_as_a_separate_argument() {
    let procs = inspector(vec![desktop(1, &["--user-data-dir", PROFILE])]);

    assert_eq!(procs.running_profiles(), set(&[PROFILE]));
}

#[test]
fn quotes_around_the_path_are_ignored() {
    let quoted = format!("--user-data-dir=\"{PROFILE}\"");
    let procs = inspector(vec![desktop(1, &[&quoted])]);

    assert_eq!(procs.running_profiles(), set(&[PROFILE]));
}

#[test]
fn two_profiles_running_side_by_side_are_both_reported() {
    let other = r"C:\Users\me\AppData\Local\clawshell-a0b1c2d3-e4f5-4a6b-8c7d-9e0f1a2b3c4d";
    let procs = inspector(vec![
        desktop(1, &[&format!("--user-data-dir={PROFILE}")]),
        desktop(2, &[&format!("--user-data-dir={other}")]),
    ]);

    assert_eq!(procs.running_profiles(), set(&[PROFILE, other]));
}

#[test]
fn code_cli_children_do_not_count_as_running() {
    // Claude Code's CLI lives inside the profile and is not Claude Desktop.
    let cli = ProcessInfo {
        pid: 9,
        exe: Some(PathBuf::from(format!(
            r"{PROFILE}\claude-code\2.1.309\9078b9c1d466\claude.exe"
        ))),
        args: vec![
            "--output-format".into(),
            "stream-json".into(),
            format!("--user-data-dir={PROFILE}"),
        ],
    };
    let procs = inspector(vec![cli]);

    assert!(procs.running_profiles().is_empty());
}

#[test]
fn a_desktop_process_without_a_user_data_dir_runs_the_default_profile() {
    let procs = inspector(vec![desktop(1, &[]), desktop(2, &["--type=renderer"])]);

    assert_eq!(procs.running_profiles(), set(&[DEFAULT_PROFILE]));
    assert_eq!(procs.pids_for(&PathBuf::from(DEFAULT_PROFILE)), vec![1, 2]);
}

#[test]
fn other_programs_in_the_install_folder_are_not_the_default_profile() {
    // The installer build keeps its updater next to the app folders.
    let updater = ProcessInfo {
        pid: 4,
        exe: Some(PathBuf::from(format!(r"{ROOT}\Update.exe"))),
        args: vec!["--processStart".into(), "claude.exe".into()],
    };

    assert!(inspector(vec![updater]).running_profiles().is_empty());
}

#[test]
fn other_programs_with_the_same_argument_do_not_count() {
    let imposter = ProcessInfo {
        pid: 5,
        exe: Some(PathBuf::from(r"C:\Tools\other.exe")),
        args: vec![format!("--user-data-dir={PROFILE}")],
    };
    let unknown_exe = ProcessInfo {
        pid: 6,
        exe: None,
        args: vec![format!("--user-data-dir={PROFILE}")],
    };

    assert!(
        inspector(vec![imposter, unknown_exe])
            .running_profiles()
            .is_empty()
    );
}

#[test]
fn matching_ignores_case_and_separator_style() {
    let shouting = ProcessInfo {
        pid: 1,
        exe: Some(PathBuf::from(format!(
            r"{}\APP\CLAUDE.EXE",
            ROOT.to_uppercase()
        ))),
        args: vec![format!("--user-data-dir={}", PROFILE.to_uppercase())],
    };
    let procs = inspector(vec![shouting]);

    assert_eq!(procs.running_profiles().len(), 1);
}

#[test]
fn nothing_runs_when_claude_cannot_be_found() {
    let lister = Arc::new(FakeProcessLister::default());
    *lister.0.lock().unwrap() = vec![desktop(1, &[&format!("--user-data-dir={PROFILE}")])];
    let os = Arc::new(FakeOs::default());
    *os.install.lock().unwrap() = None;

    assert!(
        DesktopProcesses::new(lister, os, roots())
            .running_profiles()
            .is_empty()
    );
}

#[test]
fn lists_the_pids_that_belong_to_one_profile() {
    let other = r"C:\Users\me\AppData\Local\clawshell-a0b1c2d3-e4f5-4a6b-8c7d-9e0f1a2b3c4d";
    let procs = inspector(vec![
        desktop(1, &[&format!("--user-data-dir={PROFILE}")]),
        desktop(2, &[&format!("--user-data-dir={other}")]),
        desktop(
            3,
            &["--type=renderer", &format!("--user-data-dir={PROFILE}")],
        ),
    ]);

    assert_eq!(procs.pids_for(std::path::Path::new(PROFILE)), vec![1, 3]);
}
