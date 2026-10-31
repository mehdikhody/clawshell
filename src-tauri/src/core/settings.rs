//! clawshell's own settings: one JSON file in `clawshell/settings.json` under
//! local app data, read and written only by the core.

use super::Core;
use super::ports::{ClaudeInstall, ClaudeLocator};
use serde::{Deserialize, Serialize};
use std::io;
use std::path::{Path, PathBuf};

const APP_DATA_DIR: &str = "clawshell";
const FILE: &str = "settings.json";
const DEFAULT_WARNING_THRESHOLD: u8 = 80;
const DEFAULT_LIMIT_THRESHOLD: u8 = 100;

/// The interface language, saved as `en` or `fa`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    #[default]
    En,
    Fa,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    /// Start clawshell when Windows starts.
    pub launch_at_boot: bool,
    /// A boot-time launch stays in the tray instead of showing the window.
    pub start_hidden: bool,
    /// The close button hides the window to the tray; off, it quits.
    pub close_to_tray: bool,
    /// The language of the window and the tray menu.
    pub language: Language,
    /// Where `Claude.exe` is, when detection can't find it.
    pub claude_path: Option<PathBuf>,
    /// Notify when a window reaches the warning threshold.
    pub notify_warning: bool,
    /// Notify when a window reaches the limit threshold.
    pub notify_limited: bool,
    /// Notify when a window that was Limited resets.
    pub notify_reset: bool,
    /// Percent of a window at which the warning fires.
    pub warning_threshold: u8,
    /// Percent of a window at which it counts as Limited.
    pub limit_threshold: u8,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            launch_at_boot: false,
            start_hidden: false,
            close_to_tray: true,
            language: Language::En,
            claude_path: None,
            notify_warning: true,
            notify_limited: true,
            notify_reset: true,
            warning_threshold: DEFAULT_WARNING_THRESHOLD,
            limit_threshold: DEFAULT_LIMIT_THRESHOLD,
        }
    }
}

fn file(local_app_data: &Path) -> PathBuf {
    local_app_data.join(APP_DATA_DIR).join(FILE)
}

impl Settings {
    /// Missing or unreadable settings fall back to the defaults.
    pub fn load(local_app_data: &Path) -> Self {
        std::fs::read_to_string(file(local_app_data))
            .ok()
            .and_then(|text| serde_json::from_str::<Settings>(&text).ok())
            .map(Settings::with_usable_thresholds)
            .unwrap_or_default()
    }

    /// A saved file whose thresholds make no sense gets the default pair back.
    fn with_usable_thresholds(mut self) -> Self {
        if !thresholds_valid(&self) {
            self.warning_threshold = DEFAULT_WARNING_THRESHOLD;
            self.limit_threshold = DEFAULT_LIMIT_THRESHOLD;
        }
        self
    }

    fn save(&self, local_app_data: &Path) -> io::Result<()> {
        let path = file(local_app_data);
        std::fs::create_dir_all(path.parent().expect("file sits in a folder"))?;
        let tmp = path.with_extension("json.tmp");
        std::fs::write(
            &tmp,
            serde_json::to_vec_pretty(self).map_err(io::Error::other)?,
        )?;
        std::fs::rename(tmp, path)
    }
}

#[derive(Debug)]
pub enum SettingsError {
    /// The Claude.exe override points at nothing.
    ClaudePathMissing,
    /// The override is a folder, not a file.
    ClaudePathNotAFile,
    /// The override is a file, but not `Claude.exe`.
    ClaudePathNotClaude,
    /// The warning threshold is not below the limit threshold, or one of them
    /// is outside 1 to 100.
    InvalidThresholds,
    /// Windows refused to register or unregister autostart.
    Autostart(io::Error),
    Io(io::Error),
}

impl Core {
    pub fn settings(&self) -> Settings {
        Settings::load(&self.ports.roots.local_app_data)
    }

    /// Replaces the settings. Nothing is saved unless every part is accepted.
    pub fn update_settings(&self, new: Settings) -> Result<Settings, SettingsError> {
        let local = &self.ports.roots.local_app_data;
        let old = Settings::load(local);
        // Validate before touching Windows, so a refusal changes nothing.
        if !thresholds_valid(&new) {
            return Err(SettingsError::InvalidThresholds);
        }
        if new.claude_path != old.claude_path
            && let Some(path) = &new.claude_path
        {
            validate_claude_path(path)?;
        }
        if new.launch_at_boot != old.launch_at_boot {
            self.ports
                .os
                .set_enabled(new.launch_at_boot)
                .map_err(SettingsError::Autostart)?;
        }
        new.save(local).map_err(SettingsError::Io)?;
        // New thresholds and switches apply to the readings already held.
        self.notify_held_usage();
        Ok(new)
    }
}

fn thresholds_valid(s: &Settings) -> bool {
    (1..=100).contains(&s.warning_threshold)
        && (1..=100).contains(&s.limit_threshold)
        && s.warning_threshold < s.limit_threshold
}

const CLAUDE_EXE: &str = "claude.exe";

/// An override must be an existing file named `Claude.exe`.
fn validate_claude_path(path: &Path) -> Result<(), SettingsError> {
    if !path.exists() {
        return Err(SettingsError::ClaudePathMissing);
    }
    if !path.is_file() {
        return Err(SettingsError::ClaudePathNotAFile);
    }
    let is_claude = path
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.eq_ignore_ascii_case(CLAUDE_EXE));
    if is_claude {
        Ok(())
    } else {
        Err(SettingsError::ClaudePathNotClaude)
    }
}

/// Detection first; the saved override only when detection finds nothing and
/// the file is still there. The override's folder stands in for the install
/// root, so processes started from it count as Claude Desktop.
pub fn locate_with_override(
    detected: &dyn ClaudeLocator,
    local_app_data: &Path,
) -> Option<ClaudeInstall> {
    detected.locate().or_else(|| {
        let exe = Settings::load(local_app_data).claude_path?;
        let root = exe.parent()?.to_path_buf();
        exe.is_file().then_some(ClaudeInstall { exe, root })
    })
}

/// A locator that falls back to the saved Claude.exe override. The running
/// check uses it too, so an overridden install shows as running.
pub struct OverrideAware {
    detected: std::sync::Arc<dyn ClaudeLocator>,
    local_app_data: PathBuf,
}

impl OverrideAware {
    pub fn new(detected: std::sync::Arc<dyn ClaudeLocator>, local_app_data: PathBuf) -> Self {
        OverrideAware {
            detected,
            local_app_data,
        }
    }
}

impl ClaudeLocator for OverrideAware {
    fn locate(&self) -> Option<ClaudeInstall> {
        locate_with_override(self.detected.as_ref(), &self.local_app_data)
    }
}

impl Core {
    /// The Claude to start: detected, else the override.
    pub(super) fn claude_install(&self) -> Option<ClaudeInstall> {
        locate_with_override(self.ports.os.as_ref(), &self.ports.roots.local_app_data)
    }
}
