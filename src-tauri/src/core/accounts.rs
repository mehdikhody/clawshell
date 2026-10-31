//! Accounts: listing with status, adding, launching, renaming, opening the
//! folder and deleting.

use super::badge::{PALETTE, app_user_model_id, badge_letter, is_valid_colour};
use super::paths::same_path;
use super::pooled::{PooledRemaining, pool_remaining};
use super::signin::is_signed_in;
use super::store::Records;
use super::usage::UsageSnapshot;
use super::{Core, PROFILE_PREFIX, Profile};
use serde::Serialize;
use std::collections::HashSet;
use std::io;
use std::path::PathBuf;
use uuid::Uuid;

/// What an account row shows. Precedence: Missing > Signed out > Limited >
/// Running > Idle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Status {
    /// A 5-hour or weekly window is at or above 100%. A limited account that
    /// is also running still has `running` set.
    Limited,
    Running,
    Idle,
    SignedOut,
    /// The account is recorded but its profile folder is gone (deleted
    /// outside clawshell). Offered for removal.
    Missing,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    /// The uuid in the `clawshell-{uuid}` folder name; permanent.
    pub id: String,
    pub label: String,
    /// The first letter of the label, in capitals; shown on the row and the
    /// taskbar icon.
    pub badge: char,
    /// The badge colour, `#rrggbb`.
    pub colour: String,
    pub status: Status,
    /// A Claude Desktop process uses this profile, whatever the badge says.
    pub running: bool,
    /// The sign-in's email, once the profile endpoint has answered.
    pub email: Option<String>,
    /// The latest usage reading; none until the first poll.
    pub usage: Option<UsageSnapshot>,
    #[serde(skip)]
    pub path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountList {
    /// False when Claude Desktop can't be found; the UI explains and blocks Add.
    pub claude_found: bool,
    /// Epoch ms before which a manual usage refresh is refused; `None` when
    /// one is allowed now.
    pub refresh_allowed_at: Option<i64>,
    /// The 5-hour and weekly windows pooled over the signed-in accounts.
    pub pooled: PooledRemaining,
    /// The colours the picker offers.
    pub palette: Vec<&'static str>,
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
        let accounts = self.known_accounts()?;
        Ok(AccountList {
            claude_found: self.claude_install().is_some(),
            refresh_allowed_at: self.refresh_allowed_at(&accounts),
            pooled: pool_remaining(&accounts),
            palette: PALETTE.to_vec(),
            accounts,
        })
    }

    /// Every account clawshell knows: profile folders on disk plus recorded
    /// accounts whose folder has vanished (status Missing), ordered by id.
    pub(super) fn known_accounts(&self) -> io::Result<Vec<Account>> {
        let scan = self.scan_profiles()?;
        let records = self.synced_records(&scan.profiles);
        let running = self.ports.processes.running_profiles();
        let local = &self.ports.roots.local_app_data;
        let usage = self.usage.lock().unwrap();

        let mut accounts: Vec<Account> = scan
            .profiles
            .into_iter()
            .map(|profile| {
                let label = records
                    .label_of(&profile.id)
                    .unwrap_or_default()
                    .to_string();
                let colour = colour_in(&records, &profile.id);
                let known = usage.accounts.get(&profile.id);
                let email = known.and_then(|u| u.email.clone());
                let snapshot = known.and_then(|u| u.snapshot.clone());
                account_of(profile, label, colour, &running, email, snapshot)
            })
            .collect();

        // The id becomes part of a path, so only well-formed uuids count.
        let missing: Vec<Account> = records
            .accounts
            .iter()
            .filter(|r| is_account_id(&r.id) && accounts.iter().all(|a| a.id != r.id))
            .map(|r| Account {
                id: r.id.clone(),
                label: r.label.clone(),
                badge: badge_letter(&r.label),
                colour: colour_in(&records, &r.id),
                status: Status::Missing,
                running: false,
                email: None,
                usage: None,
                path: local.join(format!("{PROFILE_PREFIX}{}", r.id)),
            })
            .collect();
        accounts.extend(missing);
        accounts.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(accounts)
    }

    fn find_account(&self, id: &str) -> io::Result<Option<Account>> {
        Ok(self.known_accounts()?.into_iter().find(|a| a.id == id))
    }

    /// Changes the account's label and nothing else: the folder and id stay.
    pub fn rename_account(&self, id: &str, label: &str) -> Result<Account, RenameError> {
        let label = label.trim();
        if label.is_empty() {
            return Err(RenameError::EmptyLabel);
        }
        let mut account = self.find_account(id)?.ok_or(RenameError::UnknownAccount)?;
        let local = &self.ports.roots.local_app_data;
        let mut records = Records::load(local);
        if !records.rename(id, label) {
            return Err(RenameError::UnknownAccount);
        }
        records.save(local)?;
        let before = account.clone();
        account.label = label.to_string();
        account.badge = badge_letter(label);
        // A new initial is a new identity for a running window.
        self.apply_taskbar_identities(std::slice::from_ref(&account));
        self.update_shortcuts(&before, &account);
        Ok(account)
    }

    /// Picks the account's badge colour (`#rrggbb`). A running window takes
    /// the new colour on its taskbar button at once.
    pub fn set_account_colour(&self, id: &str, colour: &str) -> Result<Account, ColourError> {
        if !is_valid_colour(colour) {
            return Err(ColourError::InvalidColour);
        }
        let colour = colour.to_ascii_lowercase();
        let mut account = self.find_account(id)?.ok_or(ColourError::UnknownAccount)?;
        let local = &self.ports.roots.local_app_data;
        let mut records = Records::load(local);
        if !records.set_colour(id, &colour) {
            return Err(ColourError::UnknownAccount);
        }
        records.save(local)?;
        let before = account.clone();
        account.colour = colour;
        self.apply_taskbar_identities(std::slice::from_ref(&account));
        self.update_shortcuts(&before, &account);
        Ok(account)
    }

    /// Gives each running account's Claude window its own taskbar identity
    /// (AppUserModelID and badged icon), once per identity. A renamed initial
    /// or a new colour changes the identity, so the window is applied again
    /// and its taskbar group recomputed. Runs on every watcher tick, which
    /// also catches windows opened outside clawshell.
    pub fn sync_taskbar_identities(&self) -> io::Result<()> {
        let accounts = self.known_accounts()?;
        self.apply_taskbar_identities(&accounts);
        Ok(())
    }

    /// Applies the identity of every running account in `accounts` whose
    /// window has not got it yet. An account that is not running is
    /// forgotten, so its next window is applied afresh.
    pub fn apply_taskbar_identities(&self, accounts: &[Account]) {
        let mut applied = self.taskbar.lock().unwrap();
        for account in accounts {
            if !account.running {
                applied.remove(&account.id);
                continue;
            }
            let aumid = app_user_model_id(&account.id, account.badge, &account.colour);
            if applied.get(&account.id) == Some(&aumid) {
                continue;
            }
            // Not recorded on failure, so the next tick tries again.
            if self.apply_identity(account, &aumid).is_ok() {
                applied.insert(account.id.clone(), aumid);
            }
        }
    }

    fn apply_identity(&self, account: &Account, aumid: &str) -> io::Result<()> {
        self.ports
            .os
            .apply(&account.path, aumid, account.badge, &account.colour)
    }

    /// Asks the OS to badge the window Claude is about to create, so the
    /// identity is in place before it first shows. Best effort: the watcher
    /// tick applies it again once the window is up.
    fn identify_new_window(&self, account: &Account) {
        let aumid = app_user_model_id(&account.id, account.badge, &account.colour);
        let _ = self.apply_identity(account, &aumid);
    }

    /// Shows the account's `clawshell-{uuid}` folder in Explorer.
    pub fn open_account_folder(&self, id: &str) -> Result<(), OpenFolderError> {
        let account = self
            .find_account(id)?
            .ok_or(OpenFolderError::UnknownAccount)?;
        if account.status == Status::Missing {
            return Err(OpenFolderError::FolderMissing);
        }
        self.ports.os.open_folder(&account.path)?;
        Ok(())
    }

    /// Removes the profile folder, the account's record and its shortcuts.
    /// Refused while Claude is running on the account. A Missing account has
    /// no folder left, so only its record goes. The caller confirms first.
    pub fn delete_account(&self, id: &str) -> Result<(), DeleteError> {
        let account = self.find_account(id)?.ok_or(DeleteError::UnknownAccount)?;
        if account.running {
            return Err(DeleteError::Running);
        }
        if account.status != Status::Missing {
            std::fs::remove_dir_all(&account.path)?;
        }
        let local = &self.ports.roots.local_app_data;
        let mut records = Records::load(local);
        records.remove(id);
        records.save(local)?;
        // Best effort: the account is already gone, stale shortcuts are not fatal.
        self.remove_shortcuts(&account);
        Ok(())
    }

    /// Creates a profile folder, records it as "Account N" and opens Claude on
    /// it, which lands on the sign-in screen. If Claude fails to start the
    /// account is kept so it can be launched again.
    pub fn add_account(&self) -> Result<Account, AddError> {
        let install = self.claude_install().ok_or(AddError::ClaudeNotFound)?;
        let local = &self.ports.roots.local_app_data;

        let scan = self.scan_profiles()?;
        let mut records = self.synced_records(&scan.profiles);

        let id = Uuid::new_v4().to_string();
        let path = local.join(format!("{PROFILE_PREFIX}{id}"));
        std::fs::create_dir_all(&path)?;
        let label = records.add(&id);
        let colour = colour_in(&records, &id);
        records.save(local)?;

        self.ports
            .os
            .launch(&install.exe, &path)
            .map_err(AddError::Launch)?;

        let running = self.ports.processes.running_profiles();
        let account = account_of(Profile { id, path }, label, colour, &running, None, None);
        self.identify_new_window(&account);
        Ok(account)
    }

    /// Records for every profile folder; folders clawshell created but never
    /// recorded are adopted under the next free label.
    fn synced_records(&self, profiles: &[Profile]) -> Records {
        let local = &self.ports.roots.local_app_data;
        let mut records = Records::load(local);
        let mut changed = false;
        for profile in profiles {
            if records.label_of(&profile.id).is_none() {
                records.add(&profile.id);
                changed = true;
            }
        }
        changed |= records.assign_missing_colours();
        if changed {
            let _ = records.save(local);
        }
        records
    }
}

/// The record's colour; every record has one once the records are synced.
fn colour_in(records: &Records, id: &str) -> String {
    records.colour_of(id).unwrap_or(PALETTE[0]).to_string()
}

fn account_of(
    profile: Profile,
    label: String,
    colour: String,
    running: &HashSet<PathBuf>,
    email: Option<String>,
    usage: Option<UsageSnapshot>,
) -> Account {
    let is_running = running.iter().any(|p| same_path(p, &profile.path));
    let status = if !is_signed_in(&profile.path) {
        Status::SignedOut
    } else if usage.as_ref().is_some_and(UsageSnapshot::is_limited) {
        Status::Limited
    } else if is_running {
        Status::Running
    } else {
        Status::Idle
    };
    Account {
        id: profile.id,
        badge: badge_letter(&label),
        label,
        colour,
        status,
        running: is_running,
        email,
        usage,
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
        let install = self.claude_install().ok_or(LaunchError::ClaudeNotFound)?;
        self.ports
            .os
            .launch(&install.exe, &profile.path)
            .map_err(LaunchError::Io)?;
        if let Ok(Some(account)) = self.find_account(id) {
            self.identify_new_window(&account);
        }
        Ok(LaunchOutcome::Started)
    }
}

/// A well-formed account id: the hyphenated uuid in a `clawshell-{uuid}` name.
pub(super) fn is_account_id(id: &str) -> bool {
    id.len() == 36 && Uuid::parse_str(id).is_ok()
}

#[derive(Debug)]
pub enum RenameError {
    UnknownAccount,
    /// The new label is empty or only spaces.
    EmptyLabel,
    Io(io::Error),
}

impl From<io::Error> for RenameError {
    fn from(e: io::Error) -> Self {
        RenameError::Io(e)
    }
}

#[derive(Debug)]
pub enum ColourError {
    UnknownAccount,
    /// Not `#rrggbb`.
    InvalidColour,
    Io(io::Error),
}

impl From<io::Error> for ColourError {
    fn from(e: io::Error) -> Self {
        ColourError::Io(e)
    }
}

#[derive(Debug)]
pub enum OpenFolderError {
    UnknownAccount,
    /// The profile folder no longer exists.
    FolderMissing,
    Io(io::Error),
}

impl From<io::Error> for OpenFolderError {
    fn from(e: io::Error) -> Self {
        OpenFolderError::Io(e)
    }
}

#[derive(Debug)]
pub enum DeleteError {
    UnknownAccount,
    /// Claude is running on the account; deleting would corrupt a live profile.
    Running,
    Io(io::Error),
}

impl From<io::Error> for DeleteError {
    fn from(e: io::Error) -> Self {
        DeleteError::Io(e)
    }
}
