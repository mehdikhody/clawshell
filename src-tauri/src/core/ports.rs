//! Injected ports: everything the core needs from the outside world.
//!
//! Tests supply fakes (see `tests/support`); the app supplies Windows adapters.

use super::desktop_token::AccessToken;
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

pub trait Launcher: Send + Sync {
    /// Starts `exe` with `profile` as its user-data-dir, without waiting for it.
    fn launch(&self, exe: &Path, profile: &Path) -> io::Result<()>;
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
    fn apply(&self, profile: &Path, badge: char, colour: &str) -> io::Result<()>;
}

pub trait Shortcuts: Send + Sync {
    fn create(&self, profile: &Path, label: &str) -> io::Result<()>;
    fn remove(&self, profile: &Path) -> io::Result<()>;
}

pub trait LinkHandlerRegistry: Send + Sync {
    fn current_handler(&self) -> Option<String>;
    /// `None` restores the original handler.
    fn set_handler(&self, command: Option<&str>) -> io::Result<()>;
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
