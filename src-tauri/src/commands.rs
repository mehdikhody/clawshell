//! Thin Tauri wrappers over the core, plus the watcher that pushes status
//! changes to the UI.

use crate::core::{Account, AccountList, AddError, Core, LaunchError, LaunchOutcome};
use serde::Serialize;
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter, State};

pub const ACCOUNTS_CHANGED: &str = "accounts-changed";
const WATCH_INTERVAL: Duration = Duration::from_secs(2);

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

/// Polls the account list and emits `accounts-changed` whenever it differs,
/// so Running / Idle / Signed out follow Claude windows opening and closing.
pub fn watch_accounts(app: AppHandle, core: Arc<Core>) {
    std::thread::spawn(move || {
        let mut last: Option<AccountList> = None;
        loop {
            std::thread::sleep(WATCH_INTERVAL);
            let Ok(list) = core.list_accounts() else {
                continue;
            };
            if last.as_ref() != Some(&list) {
                let _ = app.emit(ACCOUNTS_CHANGED, &list);
                last = Some(list);
            }
        }
    });
}
