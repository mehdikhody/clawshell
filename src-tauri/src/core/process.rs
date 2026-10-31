//! Telling Claude Desktop's processes from everything else on the machine.

use super::paths::{is_under, same_path};
use super::ports::{ClaudeInstall, ClaudeLocator, ProcessInfo, ProcessInspector, ProcessLister};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

const USER_DATA_DIR: &str = "--user-data-dir";

/// The running check. A process counts only when its executable lives inside
/// the Claude Desktop install and it was started with a `--user-data-dir`.
/// Code CLI children (inside the profile folder) and the original Claude
/// (no `--user-data-dir`) are ignored.
pub struct DesktopProcesses {
    lister: Arc<dyn ProcessLister>,
    locator: Arc<dyn ClaudeLocator>,
}

impl DesktopProcesses {
    pub fn new(lister: Arc<dyn ProcessLister>, locator: Arc<dyn ClaudeLocator>) -> Self {
        DesktopProcesses { lister, locator }
    }

    fn desktop_processes(&self) -> Vec<(PathBuf, ProcessInfo)> {
        let Some(install) = self.locator.locate() else {
            return Vec::new();
        };
        self.lister
            .processes()
            .into_iter()
            .filter(|p| is_desktop(p, &install))
            .filter_map(|p| Some((user_data_dir(&p.args)?, p)))
            .collect()
    }

    /// Pids of the Desktop processes using `profile`, for window lookup.
    pub fn pids_for(&self, profile: &Path) -> Vec<u32> {
        self.desktop_processes()
            .into_iter()
            .filter(|(dir, _)| same_path(dir, profile))
            .map(|(_, p)| p.pid)
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
}

fn is_desktop(process: &ProcessInfo, install: &ClaudeInstall) -> bool {
    process
        .exe
        .as_deref()
        .is_some_and(|exe| is_under(exe, &install.root))
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
