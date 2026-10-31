//! Core test harness: temp fixture folders plus scripted fakes for every port.
#![allow(dead_code)]

use clawshell_lib::core::ports::*;
use clawshell_lib::core::{Core, Ports};
use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// A temp "machine": local app data, a user home with the shared Claude Code
/// transcripts folder, and helpers to lay down profile folders.
pub struct Fixture {
    dir: tempfile::TempDir,
}

impl Fixture {
    pub fn new() -> Self {
        let dir = tempfile::tempdir().expect("temp dir");
        fs::create_dir_all(dir.path().join("LocalAppData")).unwrap();
        fs::create_dir_all(dir.path().join("Home/.claude/projects")).unwrap();
        Fixture { dir }
    }

    pub fn local_app_data(&self) -> PathBuf {
        self.dir.path().join("LocalAppData")
    }

    pub fn home(&self) -> PathBuf {
        self.dir.path().join("Home")
    }

    pub fn projects_dir(&self) -> PathBuf {
        self.home().join(".claude/projects")
    }

    pub fn roots(&self) -> Roots {
        Roots {
            local_app_data: self.local_app_data(),
            home: self.home(),
        }
    }

    /// Creates `clawshell-{id}` with the files Claude Desktop keeps in a profile.
    pub fn add_profile(&self, id: &str) -> PathBuf {
        let path = self.local_app_data().join(format!("clawshell-{id}"));
        fs::create_dir_all(path.join("claude-code-sessions")).unwrap();
        fs::write(path.join("config.json"), "{}").unwrap();
        fs::write(path.join("Local State"), "{}").unwrap();
        fs::write(path.join("plan-usage-history.json"), "[]").unwrap();
        path
    }

    /// Marks a profile as signed in the way Claude Desktop records it.
    pub fn sign_in(&self, id: &str) {
        let path = self
            .local_app_data()
            .join(format!("clawshell-{id}/config.json"));
        fs::write(path, r#"{"windowSizeWasSignedIn": true}"#).unwrap();
    }

    /// Overwrites a profile's `config.json` verbatim.
    pub fn write_config(&self, id: &str, json: &str) {
        let path = self
            .local_app_data()
            .join(format!("clawshell-{id}/config.json"));
        fs::write(path, json).unwrap();
    }

    /// Path of a profile folder, as the core reports it.
    pub fn profile_path(&self, id: &str) -> PathBuf {
        self.local_app_data().join(format!("clawshell-{id}"))
    }

    /// Creates a plain folder under local app data (e.g. the old switcher's).
    pub fn add_folder(&self, name: &str) -> PathBuf {
        let path = self.local_app_data().join(name);
        fs::create_dir_all(&path).unwrap();
        path
    }

    /// Writes a JSONL transcript under the shared `projects` folder.
    pub fn add_transcript(&self, relative: &str, lines: &[&str]) {
        let path = self.projects_dir().join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, lines.join("\n")).unwrap();
    }
}

/// Scripted HTTP: queue responses per URL, inspect the requests made.
#[derive(Default)]
pub struct FakeHttp {
    scripted: Mutex<HashMap<String, VecDeque<HttpResponse>>>,
    requests: Mutex<Vec<HttpRequest>>,
}

impl FakeHttp {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    pub fn respond(&self, url: &str, status: u16, body: &str) {
        self.scripted
            .lock()
            .unwrap()
            .entry(url.to_string())
            .or_default()
            .push_back(HttpResponse {
                status,
                headers: Vec::new(),
                body: body.to_string(),
            });
    }

    pub fn requests(&self) -> Vec<HttpRequest> {
        self.requests.lock().unwrap().clone()
    }
}

impl HttpClient for FakeHttp {
    fn send(&self, request: HttpRequest) -> Result<HttpResponse, HttpError> {
        let url = request.url.clone();
        self.requests.lock().unwrap().push(request);
        self.scripted
            .lock()
            .unwrap()
            .get_mut(&url)
            .and_then(|queue| queue.pop_front())
            .ok_or_else(|| HttpError::Network(format!("no scripted response for {url}")))
    }
}

/// A clock that only moves when the test says so.
pub struct FakeClock {
    now: Mutex<SystemTime>,
}

impl FakeClock {
    pub fn at_unix_secs(secs: u64) -> Arc<Self> {
        Arc::new(FakeClock {
            now: Mutex::new(UNIX_EPOCH + Duration::from_secs(secs)),
        })
    }

    pub fn advance(&self, by: Duration) {
        *self.now.lock().unwrap() += by;
    }
}

impl Clock for FakeClock {
    fn now(&self) -> SystemTime {
        *self.now.lock().unwrap()
    }
}

/// Token reader faked at its port: profile folder -> access token.
#[derive(Default)]
pub struct FakeTokens(pub Mutex<HashMap<PathBuf, String>>);

impl TokenReader for FakeTokens {
    fn read_token(&self, profile: &Path) -> Option<String> {
        self.0.lock().unwrap().get(profile).cloned()
    }
}

/// Process inspector faked at its port: the profile folders "running" Claude.
#[derive(Default)]
pub struct FakeProcesses(pub Mutex<HashSet<PathBuf>>);

impl ProcessInspector for FakeProcesses {
    fn running_profiles(&self) -> HashSet<PathBuf> {
        self.0.lock().unwrap().clone()
    }
}

/// Raw process table faked at the lister port.
#[derive(Default)]
pub struct FakeProcessLister(pub Mutex<Vec<ProcessInfo>>);

impl ProcessLister for FakeProcessLister {
    fn processes(&self) -> Vec<ProcessInfo> {
        self.0.lock().unwrap().clone()
    }
}

/// Records OS calls instead of touching Windows.
pub struct FakeOs {
    /// What the locator finds; `None` simulates Claude not being installed.
    pub install: Mutex<Option<ClaudeInstall>>,
    /// `(exe, profile)` of every launch.
    pub launched: Mutex<Vec<(PathBuf, PathBuf)>>,
    pub focused: Mutex<Vec<PathBuf>>,
    /// Whether `focus` finds a window to bring forward.
    pub has_window: Mutex<bool>,
}

impl Default for FakeOs {
    fn default() -> Self {
        FakeOs {
            install: Mutex::new(Some(fake_install())),
            launched: Mutex::default(),
            focused: Mutex::default(),
            has_window: Mutex::new(true),
        }
    }
}

pub fn fake_install() -> ClaudeInstall {
    ClaudeInstall {
        exe: PathBuf::from("C:/Claude/app/Claude.exe"),
        root: PathBuf::from("C:/Claude"),
    }
}

impl ClaudeLocator for FakeOs {
    fn locate(&self) -> Option<ClaudeInstall> {
        self.install.lock().unwrap().clone()
    }
}
impl Launcher for FakeOs {
    fn launch(&self, exe: &Path, profile: &Path) -> io::Result<()> {
        self.launched
            .lock()
            .unwrap()
            .push((exe.to_path_buf(), profile.to_path_buf()));
        Ok(())
    }
}
impl WindowFocus for FakeOs {
    fn focus(&self, profile: &Path) -> bool {
        self.focused.lock().unwrap().push(profile.to_path_buf());
        *self.has_window.lock().unwrap()
    }
}
impl TaskbarIdentity for FakeOs {
    fn apply(&self, _profile: &Path, _badge: char, _colour: &str) -> io::Result<()> {
        Ok(())
    }
}
impl Shortcuts for FakeOs {
    fn create(&self, _profile: &Path, _label: &str) -> io::Result<()> {
        Ok(())
    }
    fn remove(&self, _profile: &Path) -> io::Result<()> {
        Ok(())
    }
}
impl LinkHandlerRegistry for FakeOs {
    fn current_handler(&self) -> Option<String> {
        None
    }
    fn set_handler(&self, _command: Option<&str>) -> io::Result<()> {
        Ok(())
    }
}
impl Autostart for FakeOs {
    fn set_enabled(&self, _enabled: bool) -> io::Result<()> {
        Ok(())
    }
}
impl Notifier for FakeOs {
    fn notify(&self, _title: &str, _body: &str) {}
}
impl OsAdapters for FakeOs {}

/// Everything a core test needs: the fixture machine plus handles on each fake.
pub struct Harness {
    pub fixture: Fixture,
    pub http: Arc<FakeHttp>,
    pub clock: Arc<FakeClock>,
    pub tokens: Arc<FakeTokens>,
    pub processes: Arc<FakeProcesses>,
    pub os: Arc<FakeOs>,
}

impl Harness {
    /// Starts at 2026-06-15 12:00:00 UTC.
    pub fn new() -> Self {
        Harness {
            fixture: Fixture::new(),
            http: FakeHttp::new(),
            clock: FakeClock::at_unix_secs(1_781_524_800),
            tokens: Arc::new(FakeTokens::default()),
            processes: Arc::new(FakeProcesses::default()),
            os: Arc::new(FakeOs::default()),
        }
    }

    pub fn core(&self) -> Core {
        Core::new(Ports {
            roots: self.fixture.roots(),
            http: self.http.clone(),
            clock: self.clock.clone(),
            tokens: self.tokens.clone(),
            processes: self.processes.clone(),
            os: self.os.clone(),
        })
    }
}
