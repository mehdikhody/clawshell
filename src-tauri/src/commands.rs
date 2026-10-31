//! Thin Tauri wrappers over the core, plus the watcher that pushes status
//! changes to the UI.

use crate::core::ports::ClaudeLocator;
use crate::core::{
    Account, AccountError, AccountList, AddError, ColourError, Core, DeleteError, Heatmap,
    LaunchError, LaunchOptions, LaunchOptionsError, LaunchOutcome, MonthCost, OpenFolderError,
    RefreshOutcome, RenameError, RoutingChange, RoutingError, Settings, SettingsError,
    ShortcutError,
};
use crate::taskbar::{badge_image, claude_base};
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use serde::Serialize;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Emitter, State};

pub const ACCOUNTS_CHANGED: &str = "accounts-changed";
/// Sent once, after the first successful Add: the UI offers sign-in routing.
pub const ROUTING_OFFER: &str = "routing-offer";
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
        // Every running window keeps its badged taskbar identity.
        self.core.apply_taskbar_identities(&list.accounts);
        let mut last = self.last.lock().unwrap();
        if last.as_ref() != Some(&list) {
            let _ = self.app.emit(ACCOUNTS_CHANGED, &list);
            *last = Some(list);
            crate::tray::refresh(&self.app, &self.core);
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

/// Maps the two shared account failures; `io_code` is the code an operation
/// gives a disk or launch failure.
fn account_error(e: AccountError, io_code: &'static str) -> CommandError {
    match e {
        AccountError::Unknown => CommandError::new("unknown-account", "No such account"),
        AccountError::Io(e) => CommandError::new(io_code, e.to_string()),
    }
}

impl From<AccountError> for CommandError {
    fn from(e: AccountError) -> Self {
        account_error(e, "io")
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
            LaunchError::Account(e) => account_error(e, "launch-failed"),
            LaunchError::ClaudeNotFound => {
                CommandError::new("claude-not-found", "Claude Desktop was not found")
            }
        }
    }
}

impl From<RenameError> for CommandError {
    fn from(e: RenameError) -> Self {
        match e {
            RenameError::Account(e) => e.into(),
            RenameError::EmptyLabel => CommandError::new("empty-label", "The name is empty"),
        }
    }
}

impl From<ColourError> for CommandError {
    fn from(e: ColourError) -> Self {
        match e {
            ColourError::Account(e) => e.into(),
            ColourError::InvalidColour => {
                CommandError::new("invalid-colour", "The colour must look like #rrggbb")
            }
        }
    }
}

impl From<LaunchOptionsError> for CommandError {
    fn from(e: LaunchOptionsError) -> Self {
        // Messages never carry a name or a value: values can be secrets.
        match e {
            LaunchOptionsError::Account(e) => e.into(),
            LaunchOptionsError::InvalidEnvName => CommandError::new(
                "invalid-env-name",
                "Names use letters, digits and _ only, and don't start with a digit",
            ),
            LaunchOptionsError::DuplicateEnvName => {
                CommandError::new("duplicate-env-name", "A variable name is used twice")
            }
            LaunchOptionsError::InvalidEnvValue => {
                CommandError::new("invalid-env-value", "A value holds a NUL character")
            }
            LaunchOptionsError::ReservedFlag => CommandError::new(
                "reserved-flag",
                "--user-data-dir is set by clawshell and can't be changed",
            ),
        }
    }
}

impl From<OpenFolderError> for CommandError {
    fn from(e: OpenFolderError) -> Self {
        match e {
            OpenFolderError::Account(e) => e.into(),
            OpenFolderError::FolderMissing => {
                CommandError::new("folder-missing", "The account's folder is missing")
            }
        }
    }
}

impl From<DeleteError> for CommandError {
    fn from(e: DeleteError) -> Self {
        match e {
            DeleteError::Account(e) => e.into(),
            DeleteError::Running => {
                CommandError::new("account-running", "Close the account's Claude first")
            }
        }
    }
}

impl From<ShortcutError> for CommandError {
    fn from(e: ShortcutError) -> Self {
        match e {
            ShortcutError::Account(e) => account_error(e, "shortcuts-failed"),
            ShortcutError::FolderMissing => {
                CommandError::new("folder-missing", "The account's folder is missing")
            }
        }
    }
}

/// Puts "Claude - <name>" shortcuts on the Desktop and in the Start menu.
#[tauri::command]
pub fn add_shortcuts(core: State<'_, Arc<Core>>, id: String) -> Result<(), CommandError> {
    Ok(core.add_shortcuts(&id)?)
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
pub fn set_account_colour(
    core: State<'_, Arc<Core>>,
    id: String,
    colour: String,
) -> Result<Account, CommandError> {
    Ok(core.set_account_colour(&id, &colour)?)
}

/// Saves the account's environment variables and extra launch flags, or
/// refuses them and saves nothing. They apply when its Claude next starts.
#[tauri::command]
pub fn set_launch_options(
    core: State<'_, Arc<Core>>,
    feed: State<'_, AccountFeed>,
    id: String,
    options: LaunchOptions,
) -> Result<Account, CommandError> {
    let account = core.set_launch_options(&id, options)?;
    feed.publish();
    Ok(account)
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

/// Pixels of the account icon shown in the account list.
const ACCOUNT_ICON_PX: i32 = 64;

/// An account's badge icon, Claude's own icon with the letter disc, as an
/// `.ico` data URL for the account list.
#[tauri::command]
pub fn account_icon(
    locator: State<'_, Arc<dyn ClaudeLocator>>,
    badge: String,
    colour: String,
) -> Result<String, CommandError> {
    let letter = badge.chars().next().unwrap_or('?');
    let base = claude_base(locator.locate().map(|install| install.exe));
    let bytes = badge_image(letter, &colour, &base, ACCOUNT_ICON_PX)
        .ok_or_else(|| CommandError::new("icon", "the badge icon could not be drawn"))?;
    Ok(format!("data:image/x-icon;base64,{}", BASE64.encode(bytes)))
}

#[tauri::command]
pub fn add_account(app: AppHandle, core: State<'_, Arc<Core>>) -> Result<Account, CommandError> {
    let account = core.add_account()?;
    offer_routing_if_due(&app, &core);
    Ok(account)
}

/// After an Add: on the very first one, asks the UI to offer sign-in routing.
/// Returns whether it did, so the caller can bring the window forward.
pub fn offer_routing_if_due(app: &AppHandle, core: &Core) -> bool {
    let due = core.take_routing_offer();
    if due {
        let _ = app.emit(ROUTING_OFFER, ());
    }
    due
}

impl From<RoutingError> for CommandError {
    fn from(e: RoutingError) -> Self {
        match e {
            RoutingError::Registry(e) => CommandError::new("routing-failed", e.to_string()),
            RoutingError::Io(e) => CommandError::new("io", e.to_string()),
        }
    }
}

/// Turns sign-in routing on or off. The answer says whether the user still
/// has to pick clawshell on the Default Apps page (the Store build).
#[tauri::command]
pub fn set_sign_in_routing(
    core: State<'_, Arc<Core>>,
    enabled: bool,
) -> Result<RoutingChange, CommandError> {
    Ok(core.set_sign_in_routing(enabled)?)
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
            // Claude rewriting its link handler is noticed on the next tick.
            feed.core.keep_link_handler();
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

impl From<SettingsError> for CommandError {
    fn from(e: SettingsError) -> Self {
        match e {
            SettingsError::ClaudePathMissing => {
                CommandError::new("claude-path-missing", "That file doesn't exist")
            }
            SettingsError::ClaudePathNotAFile => {
                CommandError::new("claude-path-not-a-file", "That path is not a file")
            }
            SettingsError::ClaudePathNotClaude => {
                CommandError::new("claude-path-not-claude", "That file is not Claude.exe")
            }
            SettingsError::InvalidThresholds => CommandError::new(
                "invalid-thresholds",
                "The warning must be below the limit, both from 1 to 100",
            ),
            SettingsError::Autostart(e) => CommandError::new("autostart-failed", e.to_string()),
            SettingsError::Io(e) => CommandError::new("io", e.to_string()),
        }
    }
}

#[tauri::command]
pub fn get_settings(core: State<'_, Arc<Core>>) -> Settings {
    core.settings()
}

/// Saves the whole settings object, or refuses it and saves nothing.
#[tauri::command]
pub fn update_settings(
    app: AppHandle,
    core: State<'_, Arc<Core>>,
    feed: State<'_, AccountFeed>,
    settings: Settings,
) -> Result<Settings, CommandError> {
    let saved = core.update_settings(settings)?;
    // A new Claude.exe path can change whether Claude counts as found.
    feed.publish();
    // The tray menu follows the saved language.
    crate::tray::refresh(&app, &core);
    Ok(saved)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    fn code_and_message(e: impl Into<CommandError>) -> (&'static str, String) {
        let e = e.into();
        (e.code, e.message)
    }

    fn unknown() -> AccountError {
        AccountError::Unknown
    }

    fn disk() -> AccountError {
        AccountError::Io(io::Error::other("disk"))
    }

    #[test]
    fn every_operation_reports_an_unknown_account_the_same_way() {
        let expected = ("unknown-account", "No such account".to_string());
        assert_eq!(code_and_message(LaunchError::Account(unknown())), expected);
        assert_eq!(code_and_message(RenameError::Account(unknown())), expected);
        assert_eq!(code_and_message(ColourError::Account(unknown())), expected);
        assert_eq!(
            code_and_message(LaunchOptionsError::Account(unknown())),
            expected
        );
        assert_eq!(
            code_and_message(OpenFolderError::Account(unknown())),
            expected
        );
        assert_eq!(code_and_message(DeleteError::Account(unknown())), expected);
        assert_eq!(
            code_and_message(ShortcutError::Account(unknown())),
            expected
        );
    }

    #[test]
    fn disk_failures_keep_each_operations_code() {
        let message = "disk".to_string();
        assert_eq!(
            code_and_message(LaunchError::Account(disk())),
            ("launch-failed", message.clone())
        );
        assert_eq!(
            code_and_message(ShortcutError::Account(disk())),
            ("shortcuts-failed", message.clone())
        );
        assert_eq!(
            code_and_message(RenameError::Account(disk())),
            ("io", message.clone())
        );
        assert_eq!(
            code_and_message(ColourError::Account(disk())),
            ("io", message.clone())
        );
        assert_eq!(
            code_and_message(LaunchOptionsError::Account(disk())),
            ("io", message.clone())
        );
        assert_eq!(
            code_and_message(OpenFolderError::Account(disk())),
            ("io", message.clone())
        );
        assert_eq!(
            code_and_message(DeleteError::Account(disk())),
            ("io", message)
        );
    }
}
