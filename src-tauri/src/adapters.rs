//! Windows adapters behind the core's ports. Thin on purpose: the decisions
//! live in the core; these only touch the OS and are checked by hand.

use crate::core::install::detect_install;
use crate::core::ports::*;
use crate::core::process::DesktopProcesses;
use std::io;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};

const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const DETACHED_PROCESS: u32 = 0x0000_0008;
const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
/// How long a "Claude not found" answer is trusted before looking again.
const NOT_FOUND_RECHECK: Duration = Duration::from_secs(5);

pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> SystemTime {
        SystemTime::now()
    }
}

/// Reads the OS process table (executable and command line of each process).
pub struct SysinfoLister;

impl ProcessLister for SysinfoLister {
    fn processes(&self) -> Vec<ProcessInfo> {
        use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};
        let Ok(mut system) = System::new() else {
            return Vec::new();
        };
        system.refresh_processes_specifics(
            ProcessesToUpdate::All,
            true,
            ProcessRefreshKind::nothing()
                .with_exe(UpdateKind::Always)
                .with_cmd(UpdateKind::Always),
        );
        system
            .processes()
            .iter()
            .map(|(pid, process)| ProcessInfo {
                pid: pid.as_u32(),
                exe: process.exe().map(Path::to_path_buf),
                args: process
                    .cmd()
                    .iter()
                    .map(|a| a.to_string_lossy().into_owned())
                    .collect(),
            })
            .collect()
    }
}

/// Finds Claude Desktop: the Store/MSIX build through its package, the
/// installer build through its install folder. Answers are cached because the
/// package lookup shells out to PowerShell.
pub struct WindowsLocator {
    local_app_data: PathBuf,
    cache: Mutex<Cache>,
}

enum Cache {
    Empty,
    Found(ClaudeInstall),
    Missing(Instant),
}

impl WindowsLocator {
    pub fn new(local_app_data: PathBuf) -> Self {
        WindowsLocator {
            local_app_data,
            cache: Mutex::new(Cache::Empty),
        }
    }

    fn store_package_folder() -> Option<PathBuf> {
        let output = Command::new("powershell.exe")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "(Get-AppxPackage -Name Claude | Select-Object -First 1).InstallLocation",
            ])
            .stdin(Stdio::null())
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .ok()?;
        let text = String::from_utf8_lossy(&output.stdout);
        let folder = text.trim();
        (!folder.is_empty()).then(|| PathBuf::from(folder))
    }
}

impl ClaudeLocator for WindowsLocator {
    fn locate(&self) -> Option<ClaudeInstall> {
        let mut cache = self.cache.lock().unwrap();
        match &*cache {
            Cache::Found(install) if install.exe.is_file() => return Some(install.clone()),
            Cache::Missing(at) if at.elapsed() < NOT_FOUND_RECHECK => return None,
            _ => {}
        }
        let found = detect_install(
            &self.local_app_data,
            Self::store_package_folder().as_deref(),
        );
        *cache = match &found {
            Some(install) => Cache::Found(install.clone()),
            None => Cache::Missing(Instant::now()),
        };
        found
    }
}

/// Everything the app does to Windows. Tickets after this one fill in the
/// taskbar identity, shortcuts, link routing, autostart and notifications.
pub struct WindowsOs {
    locator: Arc<WindowsLocator>,
    lister: Arc<dyn ProcessLister>,
}

impl WindowsOs {
    pub fn new(locator: Arc<WindowsLocator>, lister: Arc<dyn ProcessLister>) -> Self {
        WindowsOs { locator, lister }
    }
}

impl ClaudeLocator for WindowsOs {
    fn locate(&self) -> Option<ClaudeInstall> {
        self.locator.locate()
    }
}

impl Launcher for WindowsOs {
    fn launch(&self, exe: &Path, profile: &Path) -> io::Result<()> {
        let mut command = Command::new(exe);
        command
            .arg(format!("--user-data-dir={}", profile.display()))
            // Must not leak into Electron: it would run Claude as plain Node.
            .env_remove("ELECTRON_RUN_AS_NODE")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP);
        command.spawn().map(drop)
    }
}

impl WindowFocus for WindowsOs {
    fn focus(&self, profile: &Path) -> bool {
        let inspector = DesktopProcesses::new(self.lister.clone(), self.locator.clone());
        let pids = inspector.pids_for(profile);
        !pids.is_empty() && focus_window_of(&pids)
    }
}

/// Brings the first visible, unowned, titled top-level window of any of
/// `pids` forward, restoring it if minimized.
fn focus_window_of(pids: &[u32]) -> bool {
    use windows::Win32::Foundation::{HWND, LPARAM};
    use windows::Win32::UI::WindowsAndMessaging::*;
    use windows::core::BOOL;

    struct Search<'a> {
        pids: &'a [u32],
        found: Option<HWND>,
    }

    unsafe extern "system" fn visit(hwnd: HWND, lparam: LPARAM) -> BOOL {
        // SAFETY: lparam is the `&mut Search` passed to EnumWindows below,
        // alive for the whole (synchronous) enumeration.
        let search = unsafe { &mut *(lparam.0 as *mut Search) };
        let mut pid = 0u32;
        unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
        let is_main_window = search.pids.contains(&pid)
            && unsafe { IsWindowVisible(hwnd) }.as_bool()
            && unsafe { GetWindow(hwnd, GW_OWNER) }.map_or(true, |owner| owner.is_invalid())
            && unsafe { GetWindowTextLengthW(hwnd) } > 0;
        if is_main_window {
            search.found = Some(hwnd);
            return BOOL(0);
        }
        BOOL(1)
    }

    let mut search = Search { pids, found: None };
    // EnumWindows reports an error when the callback stops it early.
    let _ = unsafe { EnumWindows(Some(visit), LPARAM(&mut search as *mut Search as isize)) };
    let Some(hwnd) = search.found else {
        return false;
    };
    unsafe {
        if IsIconic(hwnd).as_bool() {
            let _ = ShowWindow(hwnd, SW_RESTORE);
        }
        SetForegroundWindow(hwnd).as_bool()
    }
}

impl TaskbarIdentity for WindowsOs {
    fn apply(&self, _profile: &Path, _badge: char, _colour: &str) -> io::Result<()> {
        Ok(())
    }
}
impl Shortcuts for WindowsOs {
    fn create(&self, _profile: &Path, _label: &str) -> io::Result<()> {
        Ok(())
    }
    fn remove(&self, _profile: &Path) -> io::Result<()> {
        Ok(())
    }
}
impl LinkHandlerRegistry for WindowsOs {
    fn current_handler(&self) -> Option<String> {
        None
    }
    fn set_handler(&self, _command: Option<&str>) -> io::Result<()> {
        Ok(())
    }
}
impl Autostart for WindowsOs {
    fn set_enabled(&self, _enabled: bool) -> io::Result<()> {
        Ok(())
    }
}
impl Notifier for WindowsOs {
    fn notify(&self, _title: &str, _body: &str) {}
}
impl OsAdapters for WindowsOs {}

/// Placeholder until usage and identity land: no network access yet.
pub struct OfflineHttp;

impl HttpClient for OfflineHttp {
    fn send(&self, _request: HttpRequest) -> Result<HttpResponse, HttpError> {
        Err(HttpError::Network(
            "network access is not wired up yet".into(),
        ))
    }
}

/// Placeholder until usage lands: signed-in state comes from `config.json`.
pub struct NoTokens;

impl TokenReader for NoTokens {
    fn read_token(&self, _profile: &Path) -> Option<String> {
        None
    }
}
