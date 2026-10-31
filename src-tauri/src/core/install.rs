//! Locating Claude Desktop on disk.

use super::ports::ClaudeInstall;
use std::path::Path;

const INSTALLER_DIR: &str = "AnthropicClaude";

/// Store/MSIX build: `store_package` is the package's install folder (from its
/// package registration); the exe sits at `app\Claude.exe`. Installer build:
/// `%LOCALAPPDATA%\AnthropicClaude\app-{version}\claude.exe`, newest version.
/// The Store build wins when both exist.
pub fn detect_install(
    local_app_data: &Path,
    store_package: Option<&Path>,
) -> Option<ClaudeInstall> {
    store_package
        .and_then(store_install)
        .or_else(|| installer_install(local_app_data))
}

fn store_install(package: &Path) -> Option<ClaudeInstall> {
    let exe = package.join("app").join("Claude.exe");
    exe.is_file().then(|| ClaudeInstall {
        exe,
        root: package.to_path_buf(),
    })
}

fn installer_install(local_app_data: &Path) -> Option<ClaudeInstall> {
    let root = local_app_data.join(INSTALLER_DIR);
    let newest = std::fs::read_dir(&root)
        .ok()?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let version = parse_version(entry.file_name().to_str()?.strip_prefix("app-")?)?;
            let exe = entry.path().join("claude.exe");
            exe.is_file().then_some((version, exe))
        })
        .max_by(|a, b| a.0.cmp(&b.0))?;
    Some(ClaudeInstall {
        exe: newest.1,
        root,
    })
}

fn parse_version(text: &str) -> Option<Vec<u64>> {
    text.split('.').map(|part| part.parse().ok()).collect()
}
