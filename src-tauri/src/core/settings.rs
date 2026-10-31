//! clawshell's own settings: one JSON file in `clawshell/settings.json` under
//! local app data, read and written only by the core.

use super::Core;
use super::ports::{ClaudeInstall, ClaudeLocator};
use serde::{Deserialize, Serialize};
use std::io;
use std::path::{Path, PathBuf};

const APP_DATA_DIR: &str = "clawshell";
const FILE: &str = "settings.json";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    /// Start clawshell when Windows starts.
    pub launch_at_boot: bool,
    /// A boot-time launch stays in the tray instead of showing the window.
    pub start_hidden: bool,
    /// The close button hides the window to the tray; off, it quits.
    pub close_to_tray: bool,
    /// Where `Claude.exe` is, when detection can't find it.
    pub claude_path: Option<PathBuf>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            launch_at_boot: false,
            start_hidden: false,
            close_to_tray: true,
            claude_path: None,
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
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
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
        Ok(new)
    }
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
