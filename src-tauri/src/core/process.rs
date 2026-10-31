//! Telling Claude Desktop's processes from everything else on the machine.

use super::default_profile::default_profile_dir;
use super::paths::{is_under, same_path};
use super::ports::{
    ClaudeInstall, ClaudeLocator, ProcessInfo, ProcessInspector, ProcessLister, Roots,
};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

const USER_DATA_DIR: &str = "--user-data-dir";
/// Claude Desktop's own executable; the install folder holds others (the
/// installer build's updater) that are not Claude.
const CLAUDE_EXE: &str = "claude.exe";

/// The running check. A process counts only when its executable lives inside
/// the Claude Desktop install. One started with a `--user-data-dir` uses that
/// folder; Claude's own executable without one uses the default profile.
/// Code CLI children (inside the profile folder) are ignored.
pub struct DesktopProcesses {
    lister: Arc<dyn ProcessLister>,
    locator: Arc<dyn ClaudeLocator>,
    roots: Roots,
}

impl DesktopProcesses {
    pub fn new(
        lister: Arc<dyn ProcessLister>,
        locator: Arc<dyn ClaudeLocator>,
        roots: Roots,
    ) -> Self {
        DesktopProcesses {
            lister,
            locator,
            roots,
        }
    }

    fn desktop_processes(&self) -> Vec<(PathBuf, ProcessInfo)> {
        let Some(install) = self.locator.locate() else {
            return Vec::new();
        };
        let default = default_profile_dir(&self.roots, Some(&install));
        self.lister
            .processes()
            .into_iter()
            .filter(|p| is_desktop(p, &install))
            .filter_map(|p| match user_data_dir(&p.args) {
                Some(dir) => Some((dir, p)),
                None => is_claude_exe(&p).then(|| (default.clone(), p)),
            })
            .collect()
    }
}

impl ProcessInspector for DesktopProcesses {
    fn running_profiles(&self) -> HashSet<PathBuf> {
        self.desktop_processes()
            .into_iter()
            .map(|(dir, _)| dir)
            .collect()
    }

    fn pids_for(&self, profile: &Path) -> Vec<u32> {
        self.desktop_processes()
            .into_iter()
            .filter(|(dir, _)| same_path(dir, profile))
            .map(|(_, p)| p.pid)
            .collect()
    }
}

fn is_desktop(process: &ProcessInfo, install: &ClaudeInstall) -> bool {
    process
        .exe
        .as_deref()
        .is_some_and(|exe| is_under(exe, &install.root))
}

fn is_claude_exe(process: &ProcessInfo) -> bool {
    process
        .exe
        .as_deref()
        .and_then(Path::file_name)
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.eq_ignore_ascii_case(CLAUDE_EXE))
}

/// `--user-data-dir=PATH` or `--user-data-dir PATH`, with optional quotes.
fn user_data_dir(args: &[String]) -> Option<PathBuf> {
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        let value = match arg.strip_prefix(USER_DATA_DIR) {
            Some(rest) if rest.starts_with('=') => Some(rest[1..].to_string()),
            Some("") => args.next().cloned(),
            _ => None,
        };
        if let Some(value) = value {
            let path = value.trim_matches('"');
            return (!path.is_empty()).then(|| PathBuf::from(path));
        }
    }
    None
}
