//! Fatal errors: a panic or a failed start is appended to `fatal.log` under
//! clawshell's app-data folder and shown in a native dialog.
//!
//! The log must never hold a token, an email, an environment value or the
//! contents of a sign-in link, and a panic message can carry any of them. So a
//! line holds only the time, the panic's source location, a fixed message and
//! the payload's type (never its text).

use std::path::{Path, PathBuf};

const APP_DATA_DIR: &str = "clawshell";
const LOG_FILE: &str = "fatal.log";

/// What a panic carried, by type only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PayloadKind {
    Str,
    String,
    Other,
}

impl PayloadKind {
    pub fn of(payload: &(dyn std::any::Any + Send)) -> Self {
        if payload.is::<&'static str>() {
            PayloadKind::Str
        } else if payload.is::<String>() {
            PayloadKind::String
        } else {
            PayloadKind::Other
        }
    }

    fn name(self) -> &'static str {
        match self {
            PayloadKind::Str => "&str",
            PayloadKind::String => "String",
            PayloadKind::Other => "other",
        }
    }
}

/// One log line (no trailing newline). `location` is the panic's source file
/// and line, which are compile-time paths and hold nothing of the user's.
pub fn log_line(timestamp: &str, location: Option<(&str, u32)>, payload: PayloadKind) -> String {
    let at = match location {
        Some((file, line)) => format!("{file}:{line}"),
        None => "unknown location".to_string(),
    };
    format!(
        "{timestamp} fatal: clawshell panicked at {at} (payload: {})",
        payload.name()
    )
}

/// The line for a start that failed without a panic. The error itself is
/// left out: it can name paths or values.
pub fn startup_line(timestamp: &str) -> String {
    format!("{timestamp} fatal: clawshell could not run the app")
}

/// Where the log lives, under local app data.
pub fn log_path(local_app_data: &Path) -> PathBuf {
    local_app_data.join(APP_DATA_DIR).join(LOG_FILE)
}

/// Logs panics from now on and shows each in a dialog. Install it first
/// thing, before anything can panic.
pub fn install_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        let location = info.location().map(|l| (l.file(), l.line()));
        report(&log_line(&now(), location, PayloadKind::of(info.payload())));
    }));
}

/// Reports a start that failed without a panic, then ends the process.
pub fn exit_after_startup_failure() -> ! {
    report(&startup_line(&now()));
    std::process::exit(1)
}

fn now() -> String {
    chrono::Local::now().to_rfc3339()
}

/// Appends the line to the log and tells the user. Best effort throughout: a
/// failing log must not hide the dialog, nor the dialog the exit.
fn report(line: &str) {
    let log = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .map(|dir| log_path(&dir));
    if let Some(log) = &log {
        let _ = append(log, line);
    }
    let mut text = String::from("clawshell hit a fatal error and has to close.");
    if let Some(log) = &log {
        text.push_str(&format!("\n\nDetails were written to:\n{}", log.display()));
    }
    show_dialog(&text);
}

fn append(log: &Path, line: &str) -> std::io::Result<()> {
    use std::io::Write;
    if let Some(dir) = log.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log)?;
    writeln!(file, "{line}")
}

#[cfg(windows)]
fn show_dialog(text: &str) {
    use windows::Win32::UI::WindowsAndMessaging::{MB_ICONERROR, MB_OK, MessageBoxW};
    use windows::core::PCWSTR;
    let wide = |s: &str| s.encode_utf16().chain(Some(0)).collect::<Vec<u16>>();
    let (title, text) = (wide("clawshell"), wide(text));
    // SAFETY: both buffers are NUL-terminated and outlive the call.
    unsafe {
        MessageBoxW(
            None,
            PCWSTR(text.as_ptr()),
            PCWSTR(title.as_ptr()),
            MB_OK | MB_ICONERROR,
        );
    }
}

#[cfg(not(windows))]
fn show_dialog(_text: &str) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_line_holds_the_time_location_fixed_message_and_payload_type() {
        let line = log_line(
            "2026-06-15T09:30:00+00:00",
            Some(("src/core/usage.rs", 42)),
            PayloadKind::String,
        );
        assert_eq!(
            line,
            "2026-06-15T09:30:00+00:00 fatal: clawshell panicked at src/core/usage.rs:42 (payload: String)"
        );
    }

    #[test]
    fn a_missing_location_is_said_so() {
        let line = log_line("t", None, PayloadKind::Other);
        assert_eq!(
            line,
            "t fatal: clawshell panicked at unknown location (payload: other)"
        );
    }

    #[test]
    fn the_payload_is_told_by_type_and_its_text_never_reaches_the_line() {
        let secret: Box<dyn std::any::Any + Send> = Box::new(String::from("sk-ant-secret"));
        let kind = PayloadKind::of(&*secret);
        assert_eq!(kind, PayloadKind::String);
        assert!(!log_line("t", None, kind).contains("sk-ant"));
        assert_eq!(PayloadKind::of(&*Box::new("x")), PayloadKind::Str);
        assert_eq!(PayloadKind::of(&*Box::new(7_u8)), PayloadKind::Other);
    }

    #[test]
    fn a_failed_start_is_logged_without_the_error_text() {
        assert_eq!(
            startup_line("t"),
            "t fatal: clawshell could not run the app"
        );
    }

    #[test]
    fn the_log_sits_in_the_app_data_folder() {
        assert_eq!(
            log_path(Path::new("C:/Local")),
            Path::new("C:/Local").join("clawshell").join("fatal.log")
        );
    }
}
