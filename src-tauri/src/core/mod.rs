//! clawshell's domain core. Independent of Tauri: every outside dependency
//! arrives through a port in [`ports`], so tests drive it with fakes.

pub mod accounts;
pub mod cost;
pub mod desktop_token;
pub mod install;
mod paths;
pub mod polling;
pub mod pooled;
pub mod ports;
pub mod process;
mod settings;
mod shell;
mod signin;
mod store;
pub mod usage;

pub use accounts::{
    Account, AccountList, AddError, DeleteError, LaunchError, LaunchOutcome, OpenFolderError,
    RenameError, Status,
};
pub use cost::{AccountCost, Heatmap, MonthCost, NoEstimate};
pub use pooled::{PooledRemaining, PooledWindow};
pub use settings::{OverrideAware, Settings, SettingsError};
pub use shell::{CloseAction, TrayEntry};
pub use usage::{RefreshOutcome, UsageSnapshot, UsageSource};

use ports::{Clock, HttpClient, OsAdapters, ProcessInspector, Roots, TokenReader};
use std::io;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;
use uuid::Uuid;

const PROFILE_PREFIX: &str = "clawshell-";

/// Every dependency the core needs, injected at construction.
pub struct Ports {
    pub roots: Roots,
    pub http: Arc<dyn HttpClient>,
    pub clock: Arc<dyn Clock>,
    pub tokens: Arc<dyn TokenReader>,
    pub processes: Arc<dyn ProcessInspector>,
    pub os: Arc<dyn OsAdapters>,
}

/// A `clawshell-{uuid}` profile folder. The uuid is the account's permanent id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Profile {
    pub id: String,
    pub path: std::path::PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileScan {
    pub profiles: Vec<Profile>,
    pub scanned_at: SystemTime,
}

pub struct Core {
    ports: Ports,
    /// Emails and usage snapshots, polling schedules: in memory only.
    usage: Mutex<usage::UsageState>,
    /// Held for a whole refresh so two refreshes never overlap.
    refresh_gate: Mutex<()>,
    /// The last price-list fetch, so a refresh does not refetch it.
    prices: Mutex<Option<cost::PriceCache>>,
}

impl Core {
    pub fn new(ports: Ports) -> Self {
        Core {
            ports,
            usage: Mutex::default(),
            refresh_gate: Mutex::new(()),
            prices: Mutex::new(None),
        }
    }

    /// Finds the profile folders clawshell created under local app data.
    /// Anything else (the original Claude data, the old switcher's folders,
    /// clawshell's own app-data folder) is never listed.
    pub fn scan_profiles(&self) -> io::Result<ProfileScan> {
        let mut profiles = Vec::new();
        for entry in std::fs::read_dir(&self.ports.roots.local_app_data)? {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }
            let name = entry.file_name();
            let Some(id) = name
                .to_str()
                .and_then(|n| n.strip_prefix(PROFILE_PREFIX))
                .filter(|id| Uuid::parse_str(id).is_ok())
            else {
                continue;
            };
            profiles.push(Profile {
                id: id.to_string(),
                path: entry.path(),
            });
        }
        profiles.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(ProfileScan {
            profiles,
            scanned_at: self.ports.clock.now(),
        })
    }
}
