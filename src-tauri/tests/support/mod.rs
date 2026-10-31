//! Core test harness: temp fixture folders plus scripted fakes for every port.
#![allow(dead_code)]

use clawshell_lib::core::desktop_token::AccessToken;
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

    /// The fake Desktop folder.
    pub fn desktop(&self) -> PathBuf {
        self.dir.path().join("Desktop")
    }

    /// The fake Start menu `Programs` folder.
    pub fn start_menu(&self) -> PathBuf {
        self.dir.path().join("StartMenu/Programs")
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

    /// Overwrites a profile's `plan-usage-history.json` verbatim.
    pub fn write_usage_history(&self, id: &str, json: &str) {
        let path = self
            .local_app_data()
            .join(format!("clawshell-{id}/plan-usage-history.json"));
        fs::write(path, json).unwrap();
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

    /// Writes a Claude Code session file the way Desktop keeps them under
    /// `claude-code-sessions/{account}/{org}/`.
    pub fn add_session(&self, profile_id: &str, cli_session_id: &str, prior: &[&str]) {
        let dir = self.local_app_data().join(format!(
            "clawshell-{profile_id}/claude-code-sessions/acct/org"
        ));
        fs::create_dir_all(&dir).unwrap();
        let body = serde_json::json!({
            "sessionId": format!("local_{cli_session_id}"),
            "cliSessionId": cli_session_id,
            "priorCliSessionIds": prior,
        });
        fs::write(
            dir.join(format!("local_{cli_session_id}.json")),
            body.to_string(),
        )
        .unwrap();
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
    failing: Mutex<Vec<String>>,
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

    /// Like `respond`, with response headers (e.g. `Retry-After`).
    pub fn respond_with_headers(
        &self,
        url: &str,
        status: u16,
        headers: &[(&str, &str)],
        body: &str,
    ) {
        self.scripted
            .lock()
            .unwrap()
            .entry(url.to_string())
            .or_default()
            .push_back(HttpResponse {
                status,
                headers: headers
                    .iter()
                    .map(|(n, v)| (n.to_string(), v.to_string()))
                    .collect(),
                body: body.to_string(),
            });
    }

    /// Scripts a network failure for the next request to `url`.
    pub fn fail(&self, url: &str) {
        self.failing.lock().unwrap().push(url.to_string());
    }

    pub fn requests(&self) -> Vec<HttpRequest> {
        self.requests.lock().unwrap().clone()
    }
}

impl HttpClient for FakeHttp {
    fn send(&self, request: HttpRequest) -> Result<HttpResponse, HttpError> {
        let url = request.url.clone();
        self.requests.lock().unwrap().push(request);
        {
            let mut failing = self.failing.lock().unwrap();
            if let Some(at) = failing.iter().position(|u| *u == url) {
                failing.remove(at);
                return Err(HttpError::Network("connection reset".into()));
            }
        }
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
    utc_offset_secs: Mutex<i32>,
}

impl FakeClock {
    pub fn at_unix_secs(secs: u64) -> Arc<Self> {
        Arc::new(FakeClock {
            now: Mutex::new(UNIX_EPOCH + Duration::from_secs(secs)),
            utc_offset_secs: Mutex::new(0),
        })
    }

    /// Sets the local time zone as an offset from UTC, in hours.
    pub fn set_utc_offset_hours(&self, hours: i32) {
        *self.utc_offset_secs.lock().unwrap() = hours * 3_600;
    }

    pub fn advance(&self, by: Duration) {
        *self.now.lock().unwrap() += by;
    }
}

impl Clock for FakeClock {
    fn now(&self) -> SystemTime {
        *self.now.lock().unwrap()
    }

    fn utc_offset_secs(&self, _at: SystemTime) -> i32 {
        *self.utc_offset_secs.lock().unwrap()
    }
}

/// Token reader faked at its port: profile folder -> access token.
#[derive(Default)]
pub struct FakeTokens(pub Mutex<HashMap<PathBuf, (String, Option<i64>)>>);

impl FakeTokens {
    /// `expires_at_ms` is epoch milliseconds, as Claude Desktop stores it.
    pub fn set(&self, profile: &Path, token: &str, expires_at_ms: Option<i64>) {
        self.0
            .lock()
            .unwrap()
            .insert(profile.to_path_buf(), (token.to_string(), expires_at_ms));
    }
}

impl TokenReader for FakeTokens {
    fn read_token(&self, profile: &Path) -> Option<AccessToken> {
        self.0
            .lock()
            .unwrap()
            .get(profile)
            .map(|(token, expires)| AccessToken {
                access_token: token.clone(),
                expires_at_ms: *expires,
            })
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
    /// The whole spec (arguments and environment) of every launch.
    pub launch_specs: Mutex<Vec<LaunchSpec>>,
    pub focused: Mutex<Vec<PathBuf>>,
    /// Every folder shown in the file manager.
    pub opened_folders: Mutex<Vec<PathBuf>>,
    /// Where the fake Desktop, Start menu and clawshell exe are.
    pub places: Mutex<ShortcutPlaces>,
    /// Every `(path, badge, colour)` handed to `write_icon`, in order.
    pub icons_written: Mutex<Vec<(PathBuf, char, String)>>,
    /// Every shortcut file written, in order.
    pub links_written: Mutex<Vec<(PathBuf, ShortcutLink)>>,
    /// Every path handed to `delete`, in order.
    pub deleted: Mutex<Vec<PathBuf>>,
    /// Whether `focus` finds a window to bring forward.
    pub has_window: Mutex<bool>,
    /// Every value passed to the autostart port, in order.
    pub autostart: Mutex<Vec<bool>>,
    /// Makes the autostart port fail, as a locked-down registry would.
    pub autostart_fails: Mutex<bool>,
    /// Every `(title, body)` handed to the notifier, in order.
    pub notifications: Mutex<Vec<(String, String)>>,
    /// Every taskbar identity handed to the port, in order.
    pub taskbar: Mutex<Vec<AppliedIdentity>>,
    /// Makes the taskbar port fail, as when the window is not up yet.
    pub taskbar_fails: Mutex<bool>,
}

impl Default for FakeOs {
    fn default() -> Self {
        FakeOs {
            install: Mutex::new(Some(fake_install())),
            launched: Mutex::default(),
            launch_specs: Mutex::default(),
            focused: Mutex::default(),
            opened_folders: Mutex::default(),
            places: Mutex::new(ShortcutPlaces {
                desktop: PathBuf::from("C:/Users/test/Desktop"),
                start_menu: PathBuf::from("C:/Users/test/StartMenu/Programs"),
                clawshell_exe: PathBuf::from("C:/Apps/clawshell/clawshell.exe"),
            }),
            icons_written: Mutex::default(),
            links_written: Mutex::default(),
            deleted: Mutex::default(),
            has_window: Mutex::new(true),
            autostart: Mutex::default(),
            autostart_fails: Mutex::new(false),
            notifications: Mutex::default(),
            taskbar: Mutex::default(),
            taskbar_fails: Mutex::new(false),
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
    fn launch(&self, spec: &LaunchSpec) -> io::Result<()> {
        let profile = spec
            .args
            .iter()
            .find_map(|a| a.strip_prefix("--user-data-dir="))
            .expect("every launch names its profile folder");
        self.launched
            .lock()
            .unwrap()
            .push((spec.exe.clone(), PathBuf::from(profile)));
        self.launch_specs.lock().unwrap().push(spec.clone());
        Ok(())
    }
}
impl FolderOpener for FakeOs {
    fn open_folder(&self, folder: &Path) -> io::Result<()> {
        self.opened_folders
            .lock()
            .unwrap()
            .push(folder.to_path_buf());
        Ok(())
    }
}
impl WindowFocus for FakeOs {
    fn focus(&self, profile: &Path) -> bool {
        self.focused.lock().unwrap().push(profile.to_path_buf());
        *self.has_window.lock().unwrap()
    }
}
/// One call to the taskbar identity port.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppliedIdentity {
    pub profile: PathBuf,
    pub aumid: String,
    pub badge: char,
    pub colour: String,
}

impl TaskbarIdentity for FakeOs {
    fn apply(&self, profile: &Path, aumid: &str, badge: char, colour: &str) -> io::Result<()> {
        if *self.taskbar_fails.lock().unwrap() {
            return Err(io::Error::other("no window yet"));
        }
        self.taskbar.lock().unwrap().push(AppliedIdentity {
            profile: profile.to_path_buf(),
            aumid: aumid.to_string(),
            badge,
            colour: colour.to_string(),
        });
        Ok(())
    }
}
impl Shortcuts for FakeOs {
    fn places(&self) -> io::Result<ShortcutPlaces> {
        Ok(self.places.lock().unwrap().clone())
    }
    // The fake leaves a stub file in the (temp) folders, so the core's "is
    // there a shortcut to update" check has something real to look at.
    fn write_icon(&self, path: &Path, badge: char, colour: &str) -> io::Result<()> {
        fs::create_dir_all(path.parent().unwrap())?;
        fs::write(path, "ico")?;
        self.icons_written
            .lock()
            .unwrap()
            .push((path.to_path_buf(), badge, colour.to_string()));
        Ok(())
    }
    fn write_link(&self, link: &Path, spec: &ShortcutLink) -> io::Result<()> {
        fs::create_dir_all(link.parent().unwrap())?;
        fs::write(link, "lnk")?;
        self.links_written
            .lock()
            .unwrap()
            .push((link.to_path_buf(), spec.clone()));
        Ok(())
    }
    fn delete(&self, path: &Path) -> io::Result<()> {
        self.deleted.lock().unwrap().push(path.to_path_buf());
        match fs::remove_file(path) {
            Err(e) if e.kind() != io::ErrorKind::NotFound => Err(e),
            _ => Ok(()),
        }
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
    fn set_enabled(&self, enabled: bool) -> io::Result<()> {
        if *self.autostart_fails.lock().unwrap() {
            return Err(io::Error::other("registry is locked"));
        }
        self.autostart.lock().unwrap().push(enabled);
        Ok(())
    }
}
impl Notifier for FakeOs {
    fn notify(&self, title: &str, body: &str) {
        self.notifications
            .lock()
            .unwrap()
            .push((title.to_string(), body.to_string()));
    }
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
        let fixture = Fixture::new();
        let os = FakeOs::default();
        // Shortcuts land in the fixture's temp folders, never the real ones.
        *os.places.lock().unwrap() = ShortcutPlaces {
            desktop: fixture.desktop(),
            start_menu: fixture.start_menu(),
            clawshell_exe: PathBuf::from("C:/Apps/clawshell/clawshell.exe"),
        };
        Harness {
            fixture,
            http: FakeHttp::new(),
            clock: FakeClock::at_unix_secs(1_781_524_800),
            tokens: Arc::new(FakeTokens::default()),
            processes: Arc::new(FakeProcesses::default()),
            os: Arc::new(os),
        }
    }

    /// The fake clock as epoch milliseconds.
    pub fn now_ms(&self) -> i64 {
        self.clock
            .now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64
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
