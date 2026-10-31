//! Thin Tauri wrappers over the core, plus the watcher that pushes status
//! changes to the UI.

use crate::core::{
    Account, AccountList, AddError, Core, DeleteError, Heatmap, LaunchError, LaunchOutcome,
    MonthCost, OpenFolderError, RefreshOutcome, RenameError,
};
use serde::Serialize;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Emitter, State};

pub const ACCOUNTS_CHANGED: &str = "accounts-changed";
const WATCH_INTERVAL: Duration = Duration::from_secs(2);
/// How often the scheduler looks for accounts whose usage poll is due. The
/// core decides which are; most ticks do nothing.
const USAGE_TICK: Duration = Duration::from_secs(5);

/// Pushes the account list to the UI whenever it differs from the last one
/// pushed: status changes and usage snapshot changes alike.
#[derive(Clone)]
pub struct AccountFeed {
    app: AppHandle,
    core: Arc<Core>,
    last: Arc<Mutex<Option<AccountList>>>,
}

impl AccountFeed {
    pub fn new(app: AppHandle, core: Arc<Core>) -> Self {
        AccountFeed {
            app,
            core,
            last: Arc::default(),
        }
    }

    pub fn publish(&self) {
        let Ok(list) = self.core.list_accounts() else {
            return;
        };
        let mut last = self.last.lock().unwrap();
        if last.as_ref() != Some(&list) {
            let _ = self.app.emit(ACCOUNTS_CHANGED, &list);
            *last = Some(list);
        }
    }
}

/// What the UI gets on failure: a stable code to localise, plus detail.
#[derive(Debug, Serialize)]
pub struct CommandError {
    code: &'static str,
    message: String,
}

impl CommandError {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        CommandError {
            code,
            message: message.into(),
        }
    }
}

impl From<AddError> for CommandError {
    fn from(e: AddError) -> Self {
        match e {
            AddError::ClaudeNotFound => {
                CommandError::new("claude-not-found", "Claude Desktop was not found")
            }
            AddError::Io(e) => CommandError::new("io", e.to_string()),
            AddError::Launch(e) => CommandError::new("launch-failed", e.to_string()),
        }
    }
}

impl From<LaunchError> for CommandError {
    fn from(e: LaunchError) -> Self {
        match e {
            LaunchError::UnknownAccount => CommandError::new("unknown-account", "No such account"),
            LaunchError::ClaudeNotFound => {
                CommandError::new("claude-not-found", "Claude Desktop was not found")
            }
            LaunchError::Io(e) => CommandError::new("launch-failed", e.to_string()),
        }
    }
}

impl From<RenameError> for CommandError {
    fn from(e: RenameError) -> Self {
        match e {
            RenameError::UnknownAccount => CommandError::new("unknown-account", "No such account"),
            RenameError::EmptyLabel => CommandError::new("empty-label", "The name is empty"),
            RenameError::Io(e) => CommandError::new("io", e.to_string()),
        }
    }
}

impl From<OpenFolderError> for CommandError {
    fn from(e: OpenFolderError) -> Self {
        match e {
            OpenFolderError::UnknownAccount => {
                CommandError::new("unknown-account", "No such account")
            }
            OpenFolderError::FolderMissing => {
                CommandError::new("folder-missing", "The account's folder is missing")
            }
            OpenFolderError::Io(e) => CommandError::new("io", e.to_string()),
        }
    }
}

impl From<DeleteError> for CommandError {
    fn from(e: DeleteError) -> Self {
        match e {
            DeleteError::UnknownAccount => CommandError::new("unknown-account", "No such account"),
            DeleteError::Running => {
                CommandError::new("account-running", "Close the account's Claude first")
            }
            DeleteError::Io(e) => CommandError::new("io", e.to_string()),
        }
    }
}

#[tauri::command]
pub fn rename_account(
    core: State<'_, Arc<Core>>,
    id: String,
    label: String,
) -> Result<Account, CommandError> {
    Ok(core.rename_account(&id, &label)?)
}

#[tauri::command]
pub fn open_account_folder(core: State<'_, Arc<Core>>, id: String) -> Result<(), CommandError> {
    Ok(core.open_account_folder(&id)?)
}

#[tauri::command]
pub fn delete_account(core: State<'_, Arc<Core>>, id: String) -> Result<(), CommandError> {
    Ok(core.delete_account(&id)?)
}

#[tauri::command]
pub fn list_accounts(core: State<'_, Arc<Core>>) -> Result<AccountList, CommandError> {
    core.list_accounts()
        .map_err(|e| CommandError::new("io", e.to_string()))
}

#[tauri::command]
pub fn add_account(core: State<'_, Arc<Core>>) -> Result<Account, CommandError> {
    Ok(core.add_account()?)
}

#[tauri::command]
pub fn launch_account(
    core: State<'_, Arc<Core>>,
    id: String,
) -> Result<LaunchOutcome, CommandError> {
    Ok(core.launch_account(&id)?)
}

/// Manual refresh. The core refuses it inside the minimum interval; the
/// outcome says when the next one is allowed.
#[tauri::command]
pub async fn refresh_usage(
    core: State<'_, Arc<Core>>,
    feed: State<'_, AccountFeed>,
) -> Result<RefreshOutcome, CommandError> {
    let (core, feed) = (core.inner().clone(), feed.inner().clone());
    tauri::async_runtime::spawn_blocking(move || {
        let outcome = core.refresh_usage(true);
        feed.publish();
        outcome
    })
    .await
    .map_err(|e| CommandError::new("io", e.to_string()))
}

/// This month's dollars. Fetches the price list when it has to, so it runs off
/// the UI thread.
#[tauri::command]
pub async fn month_cost(core: State<'_, Arc<Core>>) -> Result<MonthCost, CommandError> {
    let core = core.inner().clone();
    tauri::async_runtime::spawn_blocking(move || core.month_cost())
        .await
        .map_err(|e| CommandError::new("io", e.to_string()))
}

/// The 30-day heatmap in both modes. Reads transcripts and may fetch the
/// price list, so it runs off the UI thread.
#[tauri::command]
pub async fn heatmap(core: State<'_, Arc<Core>>) -> Result<Heatmap, CommandError> {
    let core = core.inner().clone();
    tauri::async_runtime::spawn_blocking(move || core.heatmap())
        .await
        .map_err(|e| CommandError::new("io", e.to_string()))
}

/// Polls the account list and emits `accounts-changed` whenever it differs,
/// so Running / Idle / Signed out follow Claude windows opening and closing.
pub fn watch_accounts(feed: AccountFeed) {
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(WATCH_INTERVAL);
            feed.publish();
        }
    });
}

/// Runs the usage polling schedule and pushes each changed snapshot at once.
pub fn poll_usage(core: Arc<Core>, feed: AccountFeed) {
    std::thread::spawn(move || {
        loop {
            if core.refresh_usage(false).refreshed > 0 {
                feed.publish();
            }
            std::thread::sleep(USAGE_TICK);
        }
    });
}
