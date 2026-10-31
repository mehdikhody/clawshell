//! Injected ports: everything the core needs from the outside world.
//!
//! Tests supply fakes (see `tests/support`); the app supplies Windows adapters.

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpRequest {
    pub url: String,
    pub headers: Vec<(String, String)>,
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
}

/// Profile folder -> access token, or none when signed out.
pub trait TokenReader: Send + Sync {
    fn read_token(&self, profile: &Path) -> Option<String>;
}

/// Which profile folders currently have a running Claude Desktop.
pub trait ProcessInspector: Send + Sync {
    fn running_profiles(&self) -> HashSet<PathBuf>;
}

// OS adapters: thin Windows wrappers, verified by hand rather than automated.

pub trait Launcher: Send + Sync {
    fn launch(&self, profile: &Path) -> io::Result<()>;
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
    Launcher
    + WindowFocus
    + TaskbarIdentity
    + Shortcuts
    + LinkHandlerRegistry
    + Autostart
    + Notifier
{
}
