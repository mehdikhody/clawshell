//! The default account: the profile Claude Desktop uses when it is started
//! without `--user-data-dir`, the one its own Start-menu entry opens.

use super::install::is_store_install;
use super::ports::{ClaudeInstall, Roots};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// The default account's id. Not a uuid, so it never names a
/// `clawshell-{uuid}` folder, a shortcut or a taskbar identity.
pub const DEFAULT_ID: &str = "default";
/// Its label until the user renames it.
pub const DEFAULT_LABEL: &str = "Claude";
/// Its colour, for the dot beside its name: the orange of Claude's icon. It
/// has no badge.
pub const DEFAULT_COLOUR: &str = "#d97757";

/// Where current Claude Desktop keeps its data, one folder per install.
const DATA_DIR: &str = "Claude-Data";
/// Where older versions kept it, under `%APPDATA%`.
const LEGACY_DIR: &str = "Claude";
const CONFIG_FILE: &str = "config.json";

/// The default profile folder. Current Claude keeps it under
/// `%LOCALAPPDATA%\Claude-Data`, in a folder named for the install (the
/// package family name, in lower case, for the Store build). Older versions
/// used `%APPDATA%\Claude`, which the Store build sees in its package's
/// `LocalCache\Roaming`. The first of these that holds a `config.json` wins;
/// with none, the place current Claude would create.
pub fn default_profile_dir(roots: &Roots, install: Option<&ClaudeInstall>) -> PathBuf {
    let data = roots.local_app_data.join(DATA_DIR);
    let family = install
        .filter(|install| is_store_install(install))
        .and_then(package_family);

    let mut candidates = Vec::new();
    if let Some(family) = &family {
        candidates.push(data.join(family.to_lowercase()));
    }
    candidates.extend(newest_first(&data));
    candidates.push(match &family {
        Some(family) => roots
            .local_app_data
            .join("Packages")
            .join(family)
            .join("LocalCache")
            .join("Roaming")
            .join(LEGACY_DIR),
        None => roots.roaming_app_data.join(LEGACY_DIR),
    });

    candidates
        .iter()
        .find(|dir| dir.join(CONFIG_FILE).is_file())
        .unwrap_or(&candidates[0])
        .clone()
}

/// The Store package's family name, `Name_PublisherId`, from its install
/// folder `Name_Version_Arch__PublisherId`.
fn package_family(install: &ClaudeInstall) -> Option<String> {
    let folder = install.root.file_name()?.to_str()?;
    let (head, publisher) = folder.split_once("__")?;
    let name = head.split('_').next().filter(|name| !name.is_empty())?;
    (!publisher.is_empty()).then(|| format!("{name}_{publisher}"))
}

/// The folders under `data`, the one whose `config.json` changed last first.
fn newest_first(data: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(data) else {
        return Vec::new();
    };
    let mut folders: Vec<(Option<SystemTime>, PathBuf)> = entries
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_ok_and(|t| t.is_dir()))
        .map(|entry| {
            let path = entry.path();
            let changed = std::fs::metadata(path.join(CONFIG_FILE))
                .and_then(|m| m.modified())
                .ok();
            (changed, path)
        })
        .collect();
    folders.sort_by_key(|(changed, _)| std::cmp::Reverse(*changed));
    folders.into_iter().map(|(_, path)| path).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_family_name_comes_from_the_store_install_folder() {
        let install = ClaudeInstall {
            exe: PathBuf::from(
                r"C:\Program Files\WindowsApps\Claude_4.4212.0.0_x64__pzs8sxrjxfjjc\app\Claude.exe",
            ),
            root: PathBuf::from(
                r"C:\Program Files\WindowsApps\Claude_4.4212.0.0_x64__pzs8sxrjxfjjc",
            ),
        };
        assert_eq!(
            package_family(&install).as_deref(),
            Some("Claude_pzs8sxrjxfjjc")
        );
    }

    #[test]
    fn a_folder_without_a_publisher_has_no_family_name() {
        let install = ClaudeInstall {
            exe: PathBuf::from(r"C:\Apps\AnthropicClaude\app-1.0\claude.exe"),
            root: PathBuf::from(r"C:\Apps\AnthropicClaude"),
        };
        assert_eq!(package_family(&install), None);
    }
}
