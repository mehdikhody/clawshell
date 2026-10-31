//! The Windows half of shortcuts: the Desktop and Start-menu folders, and
//! writing a `.lnk` through IShellLink. Checked by hand; the names, targets
//! and update decisions are in `core::shortcuts`.

use crate::core::ports::ShortcutLink;
use std::io;
use std::path::{Path, PathBuf};
use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx,
    CoTaskMemFree, CoUninitialize, IPersistFile,
};
use windows::Win32::UI::Shell::{
    FOLDERID_Desktop, FOLDERID_Programs, IShellLinkW, KF_FLAG_DEFAULT, SHGetKnownFolderPath,
    ShellLink,
};
use windows::core::{GUID, HSTRING};

/// The user's Desktop folder.
pub fn desktop_folder() -> io::Result<PathBuf> {
    known_folder(&FOLDERID_Desktop)
}

/// The user's Start menu `Programs` folder, which Start search lists.
pub fn start_menu_folder() -> io::Result<PathBuf> {
    known_folder(&FOLDERID_Programs)
}

fn known_folder(id: &GUID) -> io::Result<PathBuf> {
    // SAFETY: the returned string is copied out and freed once.
    unsafe {
        let path = SHGetKnownFolderPath(id, KF_FLAG_DEFAULT, None).map_err(io::Error::other)?;
        let folder = path.to_string();
        CoTaskMemFree(Some(path.as_ptr().cast()));
        folder.map(PathBuf::from).map_err(io::Error::other)
    }
}

/// Writes (or replaces) the shortcut file `link`.
pub fn write_link(link: &Path, spec: &ShortcutLink) -> io::Result<()> {
    if let Some(folder) = link.parent() {
        std::fs::create_dir_all(folder)?;
    }
    // SAFETY: balanced with CoUninitialize below when it succeeded. A thread
    // already in another COM mode (RPC_E_CHANGED_MODE) is usable as it is.
    let com = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) }.is_ok();
    let result = save(link, spec);
    if com {
        // SAFETY: CoInitializeEx succeeded on this thread above.
        unsafe { CoUninitialize() };
    }
    result.map_err(io::Error::other)
}

fn save(link: &Path, spec: &ShortcutLink) -> windows::core::Result<()> {
    // SAFETY: COM is initialised by the caller; every string outlives its call.
    unsafe {
        let shell_link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)?;
        shell_link.SetPath(&HSTRING::from(spec.target.as_os_str()))?;
        shell_link.SetArguments(&HSTRING::from(spec.args.as_str()))?;
        shell_link.SetDescription(&HSTRING::from(spec.description.as_str()))?;
        shell_link.SetIconLocation(&HSTRING::from(spec.icon.as_os_str()), 0)?;
        if let Some(folder) = spec.target.parent() {
            shell_link.SetWorkingDirectory(&HSTRING::from(folder.as_os_str()))?;
        }
        let file: IPersistFile = windows::core::Interface::cast(&shell_link)?;
        let path = HSTRING::from(link.as_os_str());
        file.Save(&path, true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::Win32::Storage::FileSystem::WIN32_FIND_DATAW;
    use windows::Win32::UI::Shell::SLGP_RAWPATH;
    use windows::core::Interface;

    /// Writes into a temp folder, never the real Desktop. Ignored by default:
    /// the shell-link adapter is checked by hand (run with `--ignored`).
    #[test]
    #[ignore = "creates a real .lnk through the shell, in a temp folder"]
    fn a_written_link_reads_back_with_its_target_arguments_and_icon() {
        let dir = tempfile::tempdir().unwrap();
        let link = dir.path().join("Claude - Test.lnk");
        let spec = ShortcutLink {
            target: PathBuf::from(r"C:\Windows\System32\notepad.exe"),
            args: "--account 3f2b8c1e-5a4d-4e7b-9c10-2d6f8a9b0c11".into(),
            icon: dir.path().join("badge.ico"),
            description: "Open Test in Claude".into(),
        };

        write_link(&link, &spec).unwrap();

        assert!(link.is_file());
        // SAFETY: COM is initialised for the read-back and torn down after.
        unsafe {
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            let shell_link: IShellLinkW =
                CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER).unwrap();
            let file: IPersistFile = shell_link.cast().unwrap();
            file.Load(
                &HSTRING::from(link.as_os_str()),
                windows::Win32::System::Com::STGM_READ,
            )
            .unwrap();
            let mut text = [0u16; 512];
            let mut find = WIN32_FIND_DATAW::default();
            shell_link
                .GetPath(&mut text, &mut find, SLGP_RAWPATH.0 as u32)
                .unwrap();
            assert_eq!(wide(&text), spec.target.to_string_lossy());
            shell_link.GetArguments(&mut text).unwrap();
            assert_eq!(wide(&text), spec.args);
            let mut index = 0;
            shell_link.GetIconLocation(&mut text, &mut index).unwrap();
            assert_eq!(wide(&text), spec.icon.to_string_lossy());
            CoUninitialize();
        }
    }

    fn wide(text: &[u16]) -> String {
        let end = text.iter().position(|c| *c == 0).unwrap_or(text.len());
        String::from_utf16_lossy(&text[..end])
    }

    #[test]
    fn the_desktop_and_start_menu_folders_resolve() {
        assert!(desktop_folder().unwrap().is_absolute());
        assert!(start_menu_folder().unwrap().ends_with("Programs"));
    }
}
