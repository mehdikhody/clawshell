//! Accounts: listing with status, adding, launching.

use super::paths::same_path;
use super::signin::is_signed_in;
use super::store::Records;
use super::{Core, PROFILE_PREFIX, Profile};
use serde::Serialize;
use std::collections::HashSet;
use std::io;
use std::path::PathBuf;
use uuid::Uuid;

/// What an account row shows. Precedence: Signed out > Running > Idle
/// (Limited arrives with usage).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Status {
    Running,
    Idle,
    SignedOut,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Account {
    /// The uuid in the `clawshell-{uuid}` folder name; permanent.
    pub id: String,
    pub label: String,
    pub status: Status,
    /// A Claude Desktop process uses this profile, whatever the badge says.
    pub running: bool,
    #[serde(skip)]
    pub path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountList {
    /// False when Claude Desktop can't be found; the UI explains and blocks Add.
    pub claude_found: bool,
    pub accounts: Vec<Account>,
}

#[derive(Debug)]
pub enum AddError {
    /// Claude Desktop isn't installed (or can't be found).
    ClaudeNotFound,
    Io(io::Error),
    /// The account exists, but Claude would not start on it.
    Launch(io::Error),
}

impl From<io::Error> for AddError {
    fn from(e: io::Error) -> Self {
        AddError::Io(e)
    }
}

impl Core {
    pub fn list_accounts(&self) -> io::Result<AccountList> {
        let scan = self.scan_profiles()?;
        let records = self.synced_records(&scan.profiles);
        let running = self.ports.processes.running_profiles();
        let accounts = scan
            .profiles
            .into_iter()
            .map(|profile| {
                let label = records
                    .label_of(&profile.id)
                    .unwrap_or_default()
                    .to_string();
                account_of(profile, label, &running)
            })
            .collect();

        Ok(AccountList {
            claude_found: self.ports.os.locate().is_some(),
            accounts,
        })
    }

    /// Creates a profile folder, records it as "Account N" and opens Claude on
    /// it, which lands on the sign-in screen. If Claude fails to start the
    /// account is kept so it can be launched again.
    pub fn add_account(&self) -> Result<Account, AddError> {
        let install = self.ports.os.locate().ok_or(AddError::ClaudeNotFound)?;
        let local = &self.ports.roots.local_app_data;

        let scan = self.scan_profiles()?;
        let mut records = self.synced_records(&scan.profiles);

        let id = Uuid::new_v4().to_string();
        let path = local.join(format!("{PROFILE_PREFIX}{id}"));
        std::fs::create_dir_all(&path)?;
        let label = records.add(&id);
        records.save(local)?;

        self.ports
            .os
            .launch(&install.exe, &path)
            .map_err(AddError::Launch)?;

        let running = self.ports.processes.running_profiles();
        Ok(account_of(Profile { id, path }, label, &running))
    }

    /// Records for every profile folder; folders clawshell created but never
    /// recorded are adopted under the next free label.
    fn synced_records(&self, profiles: &[Profile]) -> Records {
        let local = &self.ports.roots.local_app_data;
        let mut records = Records::load(local);
        let mut adopted = false;
        for profile in profiles {
            if records.label_of(&profile.id).is_none() {
                records.add(&profile.id);
                adopted = true;
            }
        }
        if adopted {
            let _ = records.save(local);
        }
        records
    }
}

fn account_of(profile: Profile, label: String, running: &HashSet<PathBuf>) -> Account {
    let is_running = running.iter().any(|p| same_path(p, &profile.path));
    let status = if !is_signed_in(&profile.path) {
        Status::SignedOut
    } else if is_running {
        Status::Running
    } else {
        Status::Idle
    };
    Account {
        id: profile.id,
        label,
        status,
        running: is_running,
        path: profile.path,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum LaunchOutcome {
    /// Claude was started on the profile.
    Started,
    /// The profile was already running; its window was brought forward.
    Focused,
}

#[derive(Debug)]
pub enum LaunchError {
    /// No clawshell account has this id.
    UnknownAccount,
    ClaudeNotFound,
    Io(io::Error),
}

impl Core {
    /// Starts the account's Claude, or focuses it when it is already running
    /// so no second copy opens.
    pub fn launch_account(&self, id: &str) -> Result<LaunchOutcome, LaunchError> {
        let scan = self.scan_profiles().map_err(LaunchError::Io)?;
        let profile = scan
            .profiles
            .into_iter()
            .find(|p| p.id == id)
            .ok_or(LaunchError::UnknownAccount)?;

        let running = self.ports.processes.running_profiles();
        if running.iter().any(|p| same_path(p, &profile.path)) && self.ports.os.focus(&profile.path)
        {
            return Ok(LaunchOutcome::Focused);
        }

        // Also reached when the running copy has no window to focus (hidden
        // in its tray): Claude is single-instance per folder, so starting it
        // again hands over to the running copy, which shows its window.
        let install = self.ports.os.locate().ok_or(LaunchError::ClaudeNotFound)?;
        self.ports
            .os
            .launch(&install.exe, &profile.path)
            .map_err(LaunchError::Io)?;
        Ok(LaunchOutcome::Started)
    }
}
