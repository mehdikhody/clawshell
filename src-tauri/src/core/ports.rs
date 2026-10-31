//! Injected ports: everything the core needs from the outside world.
//!
//! Tests supply fakes (see `tests/support`); the app supplies Windows adapters.

use super::desktop_token::AccessToken;
use super::link::ClaudeLink;
use std::collections::HashSet;
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Filesystem roots the core works under.
#[derive(Debug, Clone)]
pub struct Roots {
    /// `%LOCALAPPDATA%`: holds the `clawshell-{uuid}` profile folders and
    /// clawshell's own `clawshell` app-data folder.
    pub local_app_data: PathBuf,
    /// `%APPDATA%`: where older Claude Desktop versions kept the default
    /// profile.
    pub roaming_app_data: PathBuf,
    /// The user's profile home (parent of `.claude/projects`).
    pub home: PathBuf,
}

#[derive(Clone, PartialEq, Eq)]
pub struct HttpRequest {
    pub url: String,
    pub headers: Vec<(String, String)>,
}

/// Header values are never shown: they carry the bearer token.
impl std::fmt::Debug for HttpRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let names: Vec<_> = self.headers.iter().map(|(name, _)| name).collect();
        f.debug_struct("HttpRequest")
            .field("url", &self.url)
            .field("header_names", &names)
            .finish()
    }
}

impl HttpRequest {
    pub fn get(url: &str) -> Self {
        HttpRequest {
            url: url.to_string(),
            headers: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HttpError {
    Network(String),
}

/// Reaches the profile, usage and pricing endpoints.
pub trait HttpClient: Send + Sync {
    fn send(&self, request: HttpRequest) -> Result<HttpResponse, HttpError>;
}

pub trait Clock: Send + Sync {
    fn now(&self) -> SystemTime;

    /// Seconds the local time zone is ahead of UTC at `at` (daylight saving
    /// included). A day boundary is local midnight, so cost buckets by this.
    fn utc_offset_secs(&self, _at: SystemTime) -> i32 {
        0
    }
}

/// Profile folder -> access token and its expiry, or none when signed out.
pub trait TokenReader: Send + Sync {
    fn read_token(&self, profile: &Path) -> Option<AccessToken>;
}

/// Which profile folders currently have a running Claude Desktop.
pub trait ProcessInspector: Send + Sync {
    fn running_profiles(&self) -> HashSet<PathBuf>;
}

/// One running OS process, as the process lister sees it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessInfo {
    pub pid: u32,
    pub exe: Option<PathBuf>,
    pub args: Vec<String>,
}

/// Raw process listing; the Windows adapter wraps the OS process table.
pub trait ProcessLister: Send + Sync {
    fn processes(&self) -> Vec<ProcessInfo>;
}

// OS adapters: thin Windows wrappers, verified by hand rather than automated.

/// A located Claude Desktop build.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaudeInstall {
    /// The `Claude.exe` to start.
    pub exe: PathBuf,
    /// Folder holding the build. A process counts as Claude Desktop only when
    /// its executable lives under here (Code CLI children live elsewhere).
    pub root: PathBuf,
}

pub trait ClaudeLocator: Send + Sync {
    /// Finds Claude Desktop (Store/MSIX or installer build); `None` when absent.
    fn locate(&self) -> Option<ClaudeInstall>;
}

/// One start of Claude, fully assembled by the core.
#[derive(Clone, PartialEq, Eq)]
pub struct LaunchSpec {
    pub exe: PathBuf,
    /// The command-line arguments, in order: the user-data-dir switch first,
    /// then the account's extra flags.
    pub args: Vec<String>,
    /// Environment variables set on this child process only. Values can be
    /// secrets, so they never appear in `Debug` output.
    pub env: Vec<(String, String)>,
}

impl std::fmt::Debug for LaunchSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let names: Vec<_> = self.env.iter().map(|(name, _)| name).collect();
        f.debug_struct("LaunchSpec")
            .field("exe", &self.exe)
            .field("args", &self.args)
            .field("env_names", &names)
            .finish()
    }
}

pub trait Launcher: Send + Sync {
    /// Starts `spec.exe` with `spec.args`, adding `spec.env` to the child
    /// process's environment (and nobody else's), without waiting for it.
    fn launch(&self, spec: &LaunchSpec) -> io::Result<()>;
}

pub trait FolderOpener: Send + Sync {
    /// Shows `folder` in the file manager (Explorer), without waiting for it.
    fn open_folder(&self, folder: &Path) -> io::Result<()>;
}

pub trait WindowFocus: Send + Sync {
    /// Brings the profile's window forward; false when it has none.
    fn focus(&self, profile: &Path) -> bool;
}

pub trait TaskbarIdentity: Send + Sync {
    /// Gives the profile's Claude window the AppUserModelID `aumid` and a
    /// taskbar icon badged with `badge` on `colour` (`#rrggbb`). The core
    /// derives the id, so a new letter or colour is a new id. Called again
    /// with the same arguments, it changes nothing. A window that does not
    /// exist yet is badged as soon as it is created, before it first shows.
    fn apply(&self, profile: &Path, aumid: &str, badge: char, colour: &str) -> io::Result<()>;
}

/// Where shortcuts go, and what they point at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShortcutPlaces {
    /// The user's Desktop folder.
    pub desktop: PathBuf,
    /// The user's Start menu `Programs` folder (what Start search lists).
    pub start_menu: PathBuf,
    /// clawshell's own executable: the one thing a shortcut may launch.
    pub clawshell_exe: PathBuf,
}

/// What a shortcut file holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShortcutLink {
    pub target: PathBuf,
    /// Command-line arguments for `target`.
    pub args: String,
    /// The `.ico` file the shortcut shows.
    pub icon: PathBuf,
    pub description: String,
}

/// Every filesystem and shell-link write behind account shortcuts. The core
/// decides names, targets and what to update; this only writes.
pub trait Shortcuts: Send + Sync {
    fn places(&self) -> io::Result<ShortcutPlaces>;
    /// Writes the badge icon (`badge` on `colour`, `#rrggbb`) to `path` as an
    /// `.ico`, creating its folder and replacing any file already there.
    fn write_icon(&self, path: &Path, badge: char, colour: &str) -> io::Result<()>;
    /// Writes the shortcut file `link`, creating its folder and replacing any
    /// shortcut already there.
    fn write_link(&self, link: &Path, spec: &ShortcutLink) -> io::Result<()>;
    /// Deletes the file at `path`; a file that is not there counts as done.
    fn delete(&self, path: &Path) -> io::Result<()>;
}

/// One `claude://` link to hand to a Claude process. The link is never shown
/// by `Debug`, and neither are the environment values.
#[derive(Clone, PartialEq, Eq)]
pub struct LinkForward {
    pub exe: PathBuf,
    /// The launch arguments, in order, without the link: the user-data-dir
    /// switch of the target account (none for the original Claude) and its
    /// extra flags. The adapter appends the link.
    pub args: Vec<String>,
    /// Environment variables set on this child process only.
    pub env: Vec<(String, String)>,
    pub link: ClaudeLink,
}

impl std::fmt::Debug for LinkForward {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let names: Vec<_> = self.env.iter().map(|(name, _)| name).collect();
        f.debug_struct("LinkForward")
            .field("exe", &self.exe)
            .field("args", &self.args)
            .field("env_names", &names)
            .finish_non_exhaustive()
    }
}

/// The user's `claude://` link handler, and the links clawshell forwards.
/// Everything here is per-user (HKCU); nothing needs administrator rights.
pub trait LinkHandlerRegistry: Send + Sync {
    /// Whether clawshell is the registered `claude://` handler right now.
    fn is_registered(&self) -> bool;
    /// An opaque copy of whatever the user has registered for `claude://`
    /// (including nothing at all), good for a later exact restore.
    fn snapshot_handler(&self) -> io::Result<String>;
    /// Registers clawshell as the `claude://` handler.
    fn register_handler(&self) -> io::Result<()>;
    /// Puts back exactly what `snapshot` recorded and removes everything
    /// `register_handler` added.
    fn restore_handler(&self, snapshot: &str) -> io::Result<()>;
    /// Whether the user's Default Apps choice for `claude://` names another
    /// app. Windows follows that choice over the registered handler, so links
    /// skip clawshell until the user picks it there.
    fn chosen_elsewhere(&self) -> bool;
    /// Opens the Windows Default Apps page for clawshell, where the user makes
    /// the one choice Windows will not let a program make for them.
    fn open_default_apps(&self) -> io::Result<()>;
    /// Starts Claude with the link, without waiting for it.
    fn forward_link(&self, forward: &LinkForward) -> io::Result<()>;
}

pub trait Autostart: Send + Sync {
    fn set_enabled(&self, enabled: bool) -> io::Result<()>;
}

pub trait Notifier: Send + Sync {
    fn notify(&self, title: &str, body: &str);
}

/// All OS adapters behind one injectable handle.
pub trait OsAdapters:
    ClaudeLocator
    + Launcher
    + FolderOpener
    + WindowFocus
    + TaskbarIdentity
    + Shortcuts
    + LinkHandlerRegistry
    + Autostart
    + Notifier
{
}
