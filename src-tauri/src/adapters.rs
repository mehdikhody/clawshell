//! Windows adapters behind the core's ports. Thin on purpose: the decisions
//! live in the core; these only touch the OS and are checked by hand.

use crate::core::desktop_token::{AccessToken, read_access_token};
use crate::core::install::detect_install;
use crate::core::ports::*;
use crate::core::process::DesktopProcesses;
use crate::link_registry;
use crate::shell_link;
use crate::taskbar::{badge_ico, badge_windows};
use std::collections::HashMap;
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

    fn utc_offset_secs(&self, at: SystemTime) -> i32 {
        use chrono::{DateTime, Local, Offset};
        DateTime::<Local>::from(at).offset().fix().local_minus_utc()
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
/// taskbar identity, shortcuts and link routing.
pub struct WindowsOs {
    /// Detection plus the saved Claude.exe override.
    locator: Arc<dyn ClaudeLocator>,
    lister: Arc<dyn ProcessLister>,
    /// Shows a Windows toast; supplied by the app, which owns the Tauri handle.
    toast: Toast,
    /// Per profile, the newest taskbar identity request. A watcher waiting for
    /// a window gives up when a newer request has replaced its own.
    watching: Arc<Mutex<HashMap<PathBuf, u64>>>,
}

/// Title and body in, a Windows notification out.
pub type Toast = Box<dyn Fn(&str, &str) + Send + Sync>;

impl WindowsOs {
    pub fn new(
        locator: Arc<dyn ClaudeLocator>,
        lister: Arc<dyn ProcessLister>,
        toast: Toast,
    ) -> Self {
        WindowsOs {
            locator,
            lister,
            toast,
            watching: Arc::default(),
        }
    }
}

impl ClaudeLocator for WindowsOs {
    fn locate(&self) -> Option<ClaudeInstall> {
        self.locator.locate()
    }
}

/// Starts Claude detached, without waiting. `last_arg` goes after `args`.
fn spawn_claude(
    exe: &Path,
    args: &[String],
    last_arg: Option<&str>,
    env: &[(String, String)],
) -> io::Result<()> {
    let mut command = Command::new(exe);
    command
        .args(args)
        .args(last_arg)
        // The account's variables reach this child only, not clawshell.
        .envs(env.iter().map(|(name, value)| (name, value)))
        // Must not leak into Electron: it would run Claude as plain Node,
        // so not even an account's own setting may bring it back.
        .env_remove("ELECTRON_RUN_AS_NODE")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP);
    command.spawn().map(drop)
}

impl Launcher for WindowsOs {
    fn launch(&self, spec: &LaunchSpec) -> io::Result<()> {
        spawn_claude(&spec.exe, &spec.args, None, &spec.env)
    }
}

impl FolderOpener for WindowsOs {
    fn open_folder(&self, folder: &Path) -> io::Result<()> {
        Command::new("explorer.exe")
            .arg(folder)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map(drop)
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

/// How often, and for how long, a window that does not exist yet is waited
/// for. Short polls so the id lands before Claude first shows the window.
const WINDOW_POLL: Duration = Duration::from_millis(100);
const WINDOW_WAIT: Duration = Duration::from_secs(60);

impl TaskbarIdentity for WindowsOs {
    fn apply(&self, profile: &Path, aumid: &str, badge: char, colour: &str) -> io::Result<()> {
        // Any watcher still waiting on this profile is out of date now.
        let generation = {
            let mut watching = self.watching.lock().unwrap();
            let next = watching.get(profile).copied().unwrap_or(0) + 1;
            watching.insert(profile.to_path_buf(), next);
            next
        };
        let inspector = DesktopProcesses::new(self.lister.clone(), self.locator.clone());
        if badge_windows(&inspector.pids_for(profile), aumid, badge, colour) > 0 {
            return Ok(());
        }

        // No window yet (Claude is still starting): badge it the moment it
        // appears, which is before it first shows.
        let watching = self.watching.clone();
        let (profile, aumid, colour) =
            (profile.to_path_buf(), aumid.to_string(), colour.to_string());
        std::thread::spawn(move || {
            let start = Instant::now();
            while start.elapsed() < WINDOW_WAIT {
                std::thread::sleep(WINDOW_POLL);
                if watching.lock().unwrap().get(&profile) != Some(&generation) {
                    return;
                }
                let pids = inspector.pids_for(&profile);
                if badge_windows(&pids, &aumid, badge, &colour) > 0 {
                    return;
                }
            }
        });
        Ok(())
    }
}
impl Shortcuts for WindowsOs {
    fn places(&self) -> io::Result<ShortcutPlaces> {
        Ok(ShortcutPlaces {
            desktop: shell_link::desktop_folder()?,
            start_menu: shell_link::start_menu_folder()?,
            clawshell_exe: std::env::current_exe()?,
        })
    }
    fn write_icon(&self, path: &Path, badge: char, colour: &str) -> io::Result<()> {
        let bytes = badge_ico(badge, colour)
            .ok_or_else(|| io::Error::other("the badge icon could not be drawn"))?;
        if let Some(folder) = path.parent() {
            std::fs::create_dir_all(folder)?;
        }
        std::fs::write(path, bytes)
    }
    fn write_link(&self, link: &Path, spec: &ShortcutLink) -> io::Result<()> {
        shell_link::write_link(link, spec)
    }
    fn delete(&self, path: &Path) -> io::Result<()> {
        match std::fs::remove_file(path) {
            Err(e) if e.kind() != io::ErrorKind::NotFound => Err(e),
            _ => Ok(()),
        }
    }
}
impl WindowsOs {
    /// What Windows runs for a `claude://` link: this very exe.
    fn handler_command() -> io::Result<String> {
        Ok(link_registry::handler_command(&std::env::current_exe()?))
    }
}

impl LinkHandlerRegistry for WindowsOs {
    fn is_registered(&self) -> bool {
        // An exe that can't be named can't be compared: leave the registry alone.
        Self::handler_command().map_or(true, |command| link_registry::is_registered(&command))
    }
    fn snapshot_handler(&self) -> io::Result<String> {
        Ok(link_registry::snapshot()?.to_text())
    }
    fn register_handler(&self) -> io::Result<()> {
        link_registry::register(&Self::handler_command()?)
    }
    fn restore_handler(&self, snapshot: &str) -> io::Result<()> {
        link_registry::restore(&link_registry::Snapshot::from_text(snapshot)?)
    }
    fn open_default_apps(&self) -> io::Result<()> {
        Command::new("explorer.exe")
            .arg(link_registry::DEFAULT_APPS_URI)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map(drop)
    }
    fn forward_link(&self, forward: &LinkForward) -> io::Result<()> {
        // The link is one argument, never a shell string, and is validated.
        spawn_claude(
            &forward.exe,
            &forward.args,
            Some(forward.link.as_str()),
            &forward.env,
        )
    }
}
impl Autostart for WindowsOs {
    fn set_enabled(&self, enabled: bool) -> io::Result<()> {
        set_run_key(enabled, &std::env::current_exe()?)
    }
}

/// Marks a launch made by Windows at sign-in, so "start hidden" can apply.
pub const AUTOSTART_ARG: &str = "--autostart";
const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const RUN_VALUE: &str = "clawshell";

/// Adds or removes clawshell's value under the per-user Run key. Removing a
/// value that is not there counts as done.
fn set_run_key(enabled: bool, exe: &Path) -> io::Result<()> {
    use windows::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS, WIN32_ERROR};
    use windows::Win32::System::Registry::*;
    use windows::core::HSTRING;

    let check = |status: WIN32_ERROR| -> io::Result<()> {
        if status == ERROR_SUCCESS {
            Ok(())
        } else {
            Err(io::Error::from_raw_os_error(status.0 as i32))
        }
    };

    let mut key = HKEY::default();
    // SAFETY: `key` is a valid out-pointer; it is closed below.
    check(unsafe {
        RegCreateKeyExW(
            HKEY_CURRENT_USER,
            &HSTRING::from(RUN_KEY),
            None,
            None,
            REG_OPTION_NON_VOLATILE,
            KEY_SET_VALUE,
            None,
            &mut key,
            None,
        )
    })?;

    let name = HSTRING::from(RUN_VALUE);
    let result = if enabled {
        let command = format!("\"{}\" {AUTOSTART_ARG}", exe.display());
        let bytes: Vec<u8> = command
            .encode_utf16()
            .chain(Some(0))
            .flat_map(u16::to_le_bytes)
            .collect();
        // SAFETY: `key` is open; `bytes` holds a NUL-terminated UTF-16 string.
        check(unsafe { RegSetValueExW(key, &name, None, REG_SZ, Some(&bytes)) })
    } else {
        // SAFETY: `key` is open.
        let status = unsafe { RegDeleteValueW(key, &name) };
        if status == ERROR_FILE_NOT_FOUND {
            Ok(())
        } else {
            check(status)
        }
    };
    // SAFETY: `key` came from RegCreateKeyExW above and is closed once.
    let _ = unsafe { RegCloseKey(key) };
    result
}
impl Notifier for WindowsOs {
    fn notify(&self, title: &str, body: &str) {
        (self.toast)(title, body);
    }
}
impl OsAdapters for WindowsOs {}

/// The only place an OAuth bearer token may be sent.
const OAUTH_API_PREFIX: &str = "https://api.anthropic.com/api/oauth/";
const HTTP_TIMEOUT: Duration = Duration::from_secs(15);

/// A request may go out only to the OAuth API, or to the public price list
/// without credentials.
fn allowed(request: &HttpRequest) -> bool {
    request.url.starts_with(OAUTH_API_PREFIX)
        || (request.url == crate::core::cost::PRICES_URL
            && request
                .headers
                .iter()
                .all(|(name, _)| !name.eq_ignore_ascii_case("authorization")))
}

/// Blocking HTTPS client for the profile and usage endpoints and the public
/// price list. It refuses any other address, follows no redirects, and never puts a URL, header or body
/// into an error.
pub struct UreqHttp {
    agent: ureq::Agent,
}

impl UreqHttp {
    pub fn new() -> Self {
        UreqHttp {
            agent: ureq::AgentBuilder::new()
                .redirects(0)
                .timeout(HTTP_TIMEOUT)
                .user_agent("clawshell")
                .build(),
        }
    }
}

impl HttpClient for UreqHttp {
    fn send(&self, request: HttpRequest) -> Result<HttpResponse, HttpError> {
        if !allowed(&request) {
            return Err(HttpError::Network("address not allowed".into()));
        }
        let mut call = self.agent.get(&request.url);
        for (name, value) in &request.headers {
            call = call.set(name, value);
        }
        let response = match call.call() {
            Ok(response) => response,
            Err(ureq::Error::Status(_, response)) => response,
            Err(_) => return Err(HttpError::Network("request failed".into())),
        };
        let headers = response
            .headers_names()
            .into_iter()
            .filter_map(|name| {
                let value = response.header(&name)?.to_string();
                Some((name, value))
            })
            .collect();
        let status = response.status();
        let body = response
            .into_string()
            .map_err(|_| HttpError::Network("unreadable response".into()))?;
        Ok(HttpResponse {
            status,
            headers,
            body,
        })
    }
}

/// Reads each profile's token cache and unwraps its AES key with DPAPI as the
/// current user. Plaintext stays in memory; nothing is logged or stored.
pub struct DesktopTokens;

impl TokenReader for DesktopTokens {
    fn read_token(&self, profile: &Path) -> Option<AccessToken> {
        read_access_token(profile, dpapi_unprotect)
    }
}

fn dpapi_unprotect(blob: &[u8]) -> Option<Vec<u8>> {
    use windows::Win32::Foundation::{HLOCAL, LocalFree};
    use windows::Win32::Security::Cryptography::{CRYPT_INTEGER_BLOB, CryptUnprotectData};

    let input = CRYPT_INTEGER_BLOB {
        cbData: u32::try_from(blob.len()).ok()?,
        pbData: blob.as_ptr() as *mut u8,
    };
    let mut output = CRYPT_INTEGER_BLOB::default();
    // SAFETY: `input` points at `blob`, alive for the call; on success
    // `output` is a buffer the OS allocated, copied out and freed below.
    unsafe {
        CryptUnprotectData(&input, None, None, None, None, 0, &mut output).ok()?;
        let key = std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec();
        let _ = LocalFree(Some(HLOCAL(output.pbData.cast())));
        Some(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(url: &str, headers: &[(&str, &str)]) -> HttpRequest {
        HttpRequest {
            url: url.to_string(),
            headers: headers
                .iter()
                .map(|(n, v)| (n.to_string(), v.to_string()))
                .collect(),
        }
    }

    #[test]
    fn sends_requests_only_to_the_oauth_api_or_the_price_list() {
        let oauth = "https://api.anthropic.com/api/oauth/usage";
        assert!(allowed(&request(oauth, &[("Authorization", "Bearer x")])));
        assert!(allowed(&request(crate::core::cost::PRICES_URL, &[])));
        assert!(!allowed(&request("https://example.test/prices.json", &[])));
    }

    #[test]
    fn never_sends_credentials_to_the_price_list() {
        let url = crate::core::cost::PRICES_URL;
        assert!(!allowed(&request(url, &[("authorization", "Bearer x")])));
    }
}
