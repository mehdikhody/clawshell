//! Sign-in routing: where a `claude://` link goes, and the registry handler
//! that makes links arrive here in the first place.

use super::Core;
use super::accounts::Status;
use super::install::is_store_install;
use super::launch_options::launch_args;
use super::link::{ClaudeLink, LinkError};
use super::ports::LinkForward;
use super::settings::Settings;
use serde::Serialize;
use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};

const APP_DATA_DIR: &str = "clawshell";
const BACKUP_FILE: &str = "link-handler-backup.json";
const OFFERED_FILE: &str = "routing-offered";

/// What the router knows about one account. `launched_signed_out` and
/// `last_used` are counters, not times: a larger one is more recent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    pub id: String,
    /// The account has no valid sign-in right now.
    pub signed_out: bool,
    /// A Claude window is open on the account.
    pub running: bool,
    /// When clawshell last started or focused the account while it was signed out.
    pub launched_signed_out: Option<u64>,
    /// When clawshell last started or focused the account's window.
    pub last_used: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    Account(String),
    /// The Claude install's own, default profile.
    OriginalClaude,
}

/// Picks where a sign-in link goes, in this order:
/// 1. the account launched while signed out (the latest, if several), as long
///    as it is still signed out;
/// 2. the only open signed-out account;
/// 3. the open account whose window was used last;
/// 4. the original Claude install.
pub fn pick_target(candidates: &[Candidate]) -> Target {
    let launched = candidates
        .iter()
        .filter(|c| c.signed_out)
        .filter_map(|c| Some((c.launched_signed_out?, c)))
        .max_by_key(|(at, _)| *at);
    if let Some((_, c)) = launched {
        return Target::Account(c.id.clone());
    }

    let mut open_signed_out = candidates.iter().filter(|c| c.signed_out && c.running);
    if let (Some(only), None) = (open_signed_out.next(), open_signed_out.next()) {
        return Target::Account(only.id.clone());
    }

    candidates
        .iter()
        .filter(|c| c.running)
        .filter_map(|c| Some((c.last_used?, c)))
        .max_by_key(|(at, _)| *at)
        .map_or(Target::OriginalClaude, |(_, c)| {
            Target::Account(c.id.clone())
        })
}

/// When clawshell last started or focused each account, as counters.
#[derive(Default)]
pub(super) struct Launches {
    counter: u64,
    signed_out: HashMap<String, u64>,
    used: HashMap<String, u64>,
}

impl Launches {
    /// An account's window was started or brought forward. `signed_out`: it
    /// had no valid sign-in at that moment.
    pub(super) fn note(&mut self, id: &str, signed_out: bool) {
        self.counter += 1;
        self.used.insert(id.to_string(), self.counter);
        if signed_out {
            self.signed_out.insert(id.to_string(), self.counter);
        }
    }
}

/// What turning routing on or off changed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RoutingChange {
    pub settings: Settings,
    /// The Store build keeps one step for the user: pick clawshell for
    /// `claude` links on the Default Apps page, which has been opened.
    pub needs_default_apps_choice: bool,
}

#[derive(Debug)]
pub enum RoutingError {
    /// Windows refused to read or write the link handler.
    Registry(io::Error),
    Io(io::Error),
}

/// Why a link was not forwarded. Never carries the link.
#[derive(Debug)]
pub enum RouteError {
    Refused(LinkError),
    ClaudeNotFound,
    Launch(io::Error),
}

fn app_data(local_app_data: &Path, file: &str) -> PathBuf {
    local_app_data.join(APP_DATA_DIR).join(file)
}

/// The registry snapshot taken before clawshell took `claude://` over, kept
/// on disk so turning routing off after a restart still restores it.
mod backup {
    use super::{BACKUP_FILE, app_data};
    use std::io;
    use std::path::Path;

    pub fn load(local_app_data: &Path) -> Option<String> {
        let text = std::fs::read_to_string(app_data(local_app_data, BACKUP_FILE)).ok()?;
        let value: serde_json::Value = serde_json::from_str(&text).ok()?;
        value.get("snapshot")?.as_str().map(str::to_string)
    }

    pub fn save(local_app_data: &Path, snapshot: &str) -> io::Result<()> {
        let path = app_data(local_app_data, BACKUP_FILE);
        std::fs::create_dir_all(path.parent().expect("file sits in a folder"))?;
        let tmp = path.with_extension("json.tmp");
        std::fs::write(
            &tmp,
            serde_json::json!({ "snapshot": snapshot }).to_string(),
        )?;
        std::fs::rename(tmp, path)
    }

    pub fn remove(local_app_data: &Path) {
        let _ = std::fs::remove_file(app_data(local_app_data, BACKUP_FILE));
    }
}

impl Core {
    /// Turns sign-in routing on or off.
    ///
    /// On: backs up what is registered for `claude://`, registers clawshell
    /// (per user), and on the Store build opens Default Apps for the user's
    /// one choice. Off: puts the backup back exactly. A refusal changes
    /// nothing: routing keeps its old state.
    pub fn set_sign_in_routing(&self, enabled: bool) -> Result<RoutingChange, RoutingError> {
        let _gate = self.routing_gate.lock().unwrap();
        let local = &self.ports.roots.local_app_data;
        let os = &self.ports.os;
        let mut needs_default_apps_choice = false;

        if enabled {
            // The first backup is the original; asking again never replaces it.
            let fresh = backup::load(local).is_none();
            if fresh {
                let snapshot = os.snapshot_handler().map_err(RoutingError::Registry)?;
                backup::save(local, &snapshot).map_err(RoutingError::Io)?;
            }
            if let Err(e) = os.register_handler() {
                if fresh {
                    backup::remove(local);
                }
                return Err(RoutingError::Registry(e));
            }
            // The Store build always leaves the choice to the user; any build
            // does once the user has picked another app for the links.
            if self.claude_install().is_some_and(|i| is_store_install(&i)) || os.chosen_elsewhere()
            {
                // Best effort: the settings screen tells the user where to go.
                let _ = os.open_default_apps();
                needs_default_apps_choice = true;
            }
        } else if let Some(snapshot) = backup::load(local) {
            os.restore_handler(&snapshot)
                .map_err(RoutingError::Registry)?;
            backup::remove(local);
        }

        let mut settings = Settings::load(local);
        settings.sign_in_routing = enabled;
        settings.save(local).map_err(RoutingError::Io)?;
        Ok(RoutingChange {
            settings,
            needs_default_apps_choice,
        })
    }

    /// Takes `claude://` back when Claude (or an update) has rewritten it.
    /// What Claude wrote becomes the backup, so turning routing off later
    /// restores that and not something stale. Cheap when nothing changed:
    /// runs on startup and on every watcher tick.
    pub fn keep_link_handler(&self) {
        let _gate = self.routing_gate.lock().unwrap();
        let local = &self.ports.roots.local_app_data;
        let os = &self.ports.os;
        if !Settings::load(local).sign_in_routing || os.is_registered() {
            return;
        }
        let Ok(snapshot) = os.snapshot_handler() else {
            return;
        };
        if backup::save(local, &snapshot).is_ok() {
            let _ = os.register_handler();
        }
    }

    /// Routing is on, but the user's Default Apps choice sends `claude://`
    /// links to another app (Claude itself, on the Store build), so none
    /// reach clawshell. Settings warns until clawshell is picked.
    pub fn routing_needs_choice(&self) -> bool {
        self.settings().sign_in_routing && self.ports.os.chosen_elsewhere()
    }

    /// Opens the Default Apps page, where the user picks clawshell for links.
    pub fn open_default_apps(&self) -> io::Result<()> {
        self.ports.os.open_default_apps()
    }

    /// True the first time it is asked, and only if routing is still off:
    /// the moment to offer turning routing on (the first Add). Asked again,
    /// even after a restart, it is false.
    pub fn take_routing_offer(&self) -> bool {
        let _gate = self.routing_gate.lock().unwrap();
        let local = &self.ports.roots.local_app_data;
        let marker = app_data(local, OFFERED_FILE);
        if marker.exists() {
            return false;
        }
        // If the marker can't be written the offer is dropped, not repeated.
        let marked = marker
            .parent()
            .is_some_and(|dir| std::fs::create_dir_all(dir).is_ok())
            && std::fs::write(&marker, b"").is_ok();
        marked && !Settings::load(local).sign_in_routing
    }

    /// Forwards a `claude://` link to the account that asked for it, or to
    /// the original Claude. Refuses anything that is not a well-formed
    /// `claude://` link. The link is never logged or put in an error.
    pub fn route_link(&self, raw: &str) -> Result<Target, RouteError> {
        let link = ClaudeLink::parse(raw).map_err(RouteError::Refused)?;
        let install = self.claude_install().ok_or(RouteError::ClaudeNotFound)?;
        let accounts = self.known_accounts().unwrap_or_default();

        let target = if self.settings().sign_in_routing {
            let launches = self.launches.lock().unwrap();
            let candidates: Vec<Candidate> = accounts
                .iter()
                .filter(|a| a.status != Status::Missing)
                .map(|a| Candidate {
                    id: a.id.clone(),
                    signed_out: a.status == Status::SignedOut,
                    running: a.running,
                    launched_signed_out: launches.signed_out.get(&a.id).copied(),
                    last_used: launches.used.get(&a.id).copied(),
                })
                .collect();
            pick_target(&candidates)
        } else {
            Target::OriginalClaude
        };

        let account = match &target {
            Target::Account(id) => accounts.iter().find(|a| a.id == *id),
            Target::OriginalClaude => None,
        };
        let (args, env) = account.map_or_else(Default::default, |a| {
            (
                launch_args(&a.path, &a.flags),
                a.env
                    .iter()
                    .map(|v| (v.name.clone(), v.value.clone()))
                    .collect(),
            )
        });
        self.ports
            .os
            .forward_link(&LinkForward {
                exe: install.exe,
                args,
                env,
                link,
            })
            .map_err(RouteError::Launch)?;
        if let Some(account) = account {
            self.launches.lock().unwrap().note(&account.id, false);
            self.identify_new_window(account);
        }
        Ok(target)
    }
}
