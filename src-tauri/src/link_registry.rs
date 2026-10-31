//! The per-user (HKCU) registry side of sign-in routing: registering clawshell
//! as the `claude://` handler, taking an exact copy of what was there, and
//! putting that copy back. Thin Windows code, checked by hand; the decisions
//! (when to back up, restore or re-take) live in the core.
//!
//! Nothing here reads, writes or prints a link: it only handles the handler.

use serde::{Deserialize, Serialize};
use std::io;
use std::path::Path;
use windows::Win32::Foundation::{
    ERROR_FILE_NOT_FOUND, ERROR_NO_MORE_ITEMS, ERROR_PATH_NOT_FOUND, ERROR_SUCCESS, WIN32_ERROR,
};
use windows::Win32::System::Registry::*;
use windows::core::{HSTRING, PWSTR};

/// Claude's registration: the `claude` URL protocol under the user's classes.
const CLAUDE_KEY: &str = r"Software\Classes\claude";
const CLAUDE_COMMAND_KEY: &str = r"Software\Classes\claude\shell\open\command";
/// clawshell's own registration, for the Windows Default Apps page.
const PROG_ID: &str = "clawshell.claude";
const PROG_ID_KEY: &str = r"Software\Classes\clawshell.claude";
const APP_KEY: &str = r"Software\clawshell";
const CAPABILITIES_KEY: &str = r"Software\clawshell\Capabilities";
const REGISTERED_APPS_KEY: &str = r"Software\RegisteredApplications";
const APP_NAME: &str = "clawshell";
/// Where the Default Apps page for this app opens.
pub const DEFAULT_APPS_URI: &str = "ms-settings:defaultapps?registeredAppUser=clawshell";

/// One registry value: name, type and raw bytes (hex in the saved copy).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct ValueImage {
    name: String,
    kind: u32,
    data: String,
}

/// A registry key with everything under it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
struct KeyImage {
    values: Vec<ValueImage>,
    keys: Vec<(String, KeyImage)>,
}

/// What was registered for `claude://` before clawshell: the whole
/// `Software\Classes\claude` tree, or nothing.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Snapshot {
    claude: Option<KeyImage>,
}

impl Snapshot {
    pub fn to_text(&self) -> String {
        serde_json::to_string(self).expect("a snapshot is plain data")
    }

    pub fn from_text(text: &str) -> io::Result<Self> {
        serde_json::from_str(text)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "unreadable handler backup"))
    }
}

fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn from_hex(text: &str) -> Option<Vec<u8>> {
    if !text.len().is_multiple_of(2) {
        return None;
    }
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(text.get(i..i + 2)?, 16).ok())
        .collect()
}

/// The command Windows runs for a link: clawshell with the link as its one
/// argument. The quotes keep a path with spaces in one piece.
pub fn handler_command(exe: &Path) -> String {
    format!("\"{}\" \"%1\"", exe.display())
}

fn check(status: WIN32_ERROR) -> io::Result<()> {
    if status == ERROR_SUCCESS {
        Ok(())
    } else {
        Err(io::Error::from_raw_os_error(status.0 as i32))
    }
}

fn is_missing(status: WIN32_ERROR) -> bool {
    status == ERROR_FILE_NOT_FOUND || status == ERROR_PATH_NOT_FOUND
}

/// An open key, closed on drop.
struct Key(HKEY);

impl Drop for Key {
    fn drop(&mut self) {
        // SAFETY: the handle came from a successful open or create call and
        // is closed exactly once, here.
        let _ = unsafe { RegCloseKey(self.0) };
    }
}

/// Opens `path` under `parent`; `Ok(None)` when it does not exist.
fn open(parent: HKEY, path: &str, access: REG_SAM_FLAGS) -> io::Result<Option<Key>> {
    let mut handle = HKEY::default();
    // SAFETY: `handle` is a valid out-pointer.
    let status = unsafe { RegOpenKeyExW(parent, &HSTRING::from(path), None, access, &mut handle) };
    if is_missing(status) {
        return Ok(None);
    }
    check(status)?;
    Ok(Some(Key(handle)))
}

fn create(parent: HKEY, path: &str) -> io::Result<Key> {
    let mut handle = HKEY::default();
    // SAFETY: `handle` is a valid out-pointer.
    check(unsafe {
        RegCreateKeyExW(
            parent,
            &HSTRING::from(path),
            None,
            None,
            REG_OPTION_NON_VOLATILE,
            KEY_WRITE | KEY_READ,
            None,
            &mut handle,
            None,
        )
    })?;
    Ok(Key(handle))
}

fn utf16_bytes(text: &str) -> Vec<u8> {
    text.encode_utf16()
        .chain(Some(0))
        .flat_map(u16::to_le_bytes)
        .collect()
}

fn set_value(key: &Key, name: &str, kind: REG_VALUE_TYPE, data: &[u8]) -> io::Result<()> {
    // SAFETY: the key is open for writing and `data` outlives the call.
    check(unsafe { RegSetValueExW(key.0, &HSTRING::from(name), None, kind, Some(data)) })
}

fn set_string(key: &Key, name: &str, text: &str) -> io::Result<()> {
    set_value(key, name, REG_SZ, &utf16_bytes(text))
}

fn has_value(key: &Key, name: &str) -> bool {
    // SAFETY: only the existence of the value is asked; no buffers are given.
    let status = unsafe { RegQueryValueExW(key.0, &HSTRING::from(name), None, None, None, None) };
    status == ERROR_SUCCESS
}

/// Reads a string value; `None` when it is missing or not text.
fn read_string(key: &Key, name: &str) -> Option<String> {
    let name = HSTRING::from(name);
    let mut kind = REG_VALUE_TYPE(0);
    let mut size = 0u32;
    // SAFETY: the first call only asks for the size.
    let status =
        unsafe { RegQueryValueExW(key.0, &name, None, Some(&mut kind), None, Some(&mut size)) };
    if status != ERROR_SUCCESS || (kind != REG_SZ && kind != REG_EXPAND_SZ) {
        return None;
    }
    let mut buffer = vec![0u8; size as usize];
    // SAFETY: `buffer` holds `size` bytes, as announced.
    let status = unsafe {
        RegQueryValueExW(
            key.0,
            &name,
            None,
            Some(&mut kind),
            Some(buffer.as_mut_ptr()),
            Some(&mut size),
        )
    };
    if status != ERROR_SUCCESS {
        return None;
    }
    buffer.truncate(size as usize);
    let units: Vec<u16> = buffer
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u16::from_le_bytes(*pair))
        .collect();
    Some(
        String::from_utf16_lossy(&units)
            .trim_end_matches('\0')
            .to_string(),
    )
}

/// Copies a key and everything below it.
fn read_image(key: &Key) -> io::Result<KeyImage> {
    let (mut sub_count, mut max_sub, mut value_count, mut max_name, mut max_data) =
        (0u32, 0u32, 0u32, 0u32, 0u32);
    // SAFETY: every out-pointer is valid for the call.
    check(unsafe {
        RegQueryInfoKeyW(
            key.0,
            None,
            None,
            None,
            Some(&mut sub_count),
            Some(&mut max_sub),
            None,
            Some(&mut value_count),
            Some(&mut max_name),
            Some(&mut max_data),
            None,
            None,
        )
    })?;

    let mut image = KeyImage::default();
    for index in 0..value_count {
        let mut name = vec![0u16; max_name as usize + 1];
        let mut data = vec![0u8; (max_data as usize).max(1)];
        let (mut name_len, mut data_len, mut kind) = (name.len() as u32, data.len() as u32, 0u32);
        // SAFETY: the buffers have the lengths passed alongside them.
        let status = unsafe {
            RegEnumValueW(
                key.0,
                index,
                Some(PWSTR(name.as_mut_ptr())),
                &mut name_len,
                None,
                Some(&mut kind),
                Some(data.as_mut_ptr()),
                Some(&mut data_len),
            )
        };
        if status == ERROR_NO_MORE_ITEMS {
            break;
        }
        check(status)?;
        image.values.push(ValueImage {
            name: String::from_utf16_lossy(&name[..name_len as usize]),
            kind,
            data: to_hex(&data[..data_len as usize]),
        });
    }

    for index in 0..sub_count {
        let mut name = vec![0u16; max_sub as usize + 1];
        let mut name_len = name.len() as u32;
        // SAFETY: the buffer has the length passed alongside it.
        let status = unsafe {
            RegEnumKeyExW(
                key.0,
                index,
                Some(PWSTR(name.as_mut_ptr())),
                &mut name_len,
                None,
                None,
                None,
                None,
            )
        };
        if status == ERROR_NO_MORE_ITEMS {
            break;
        }
        check(status)?;
        let name = String::from_utf16_lossy(&name[..name_len as usize]);
        if let Some(child) = open(key.0, &name, KEY_READ)? {
            image.keys.push((name, read_image(&child)?));
        }
    }
    Ok(image)
}

/// Writes a copy back under `key`, creating the keys it names.
fn write_image(key: &Key, image: &KeyImage) -> io::Result<()> {
    for value in &image.values {
        let data = from_hex(&value.data).ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidData, "unreadable handler backup")
        })?;
        set_value(key, &value.name, REG_VALUE_TYPE(value.kind), &data)?;
    }
    for (name, child) in &image.keys {
        write_image(&create(key.0, name)?, child)?;
    }
    Ok(())
}

/// Deletes a key and everything below it; a key that is not there is done.
fn delete_tree(path: &str) -> io::Result<()> {
    // SAFETY: plain call with a valid root and path.
    let status = unsafe { RegDeleteTreeW(HKEY_CURRENT_USER, &HSTRING::from(path)) };
    if is_missing(status) {
        Ok(())
    } else {
        check(status)
    }
}

/// Copies what is registered for `claude://` right now.
pub fn snapshot() -> io::Result<Snapshot> {
    let claude = match open(HKEY_CURRENT_USER, CLAUDE_KEY, KEY_READ)? {
        Some(key) => Some(read_image(&key)?),
        None => None,
    };
    Ok(Snapshot { claude })
}

/// Whether the registered command for `claude://` is `command`. A registry
/// that can't be read counts as registered, so the caller leaves it alone.
pub fn is_registered(command: &str) -> bool {
    let key = match open(HKEY_CURRENT_USER, CLAUDE_COMMAND_KEY, KEY_READ) {
        Ok(Some(key)) => key,
        Ok(None) => return false,
        Err(_) => return true,
    };
    read_string(&key, "").is_some_and(|current| current.trim().eq_ignore_ascii_case(command))
}

/// Registers `command` for `claude://` under the current user, and clawshell
/// as an app of its own so the Default Apps page lists it.
pub fn register(command: &str) -> io::Result<()> {
    let claude = create(HKEY_CURRENT_USER, CLAUDE_KEY)?;
    // An existing registration already says what it is; only fill gaps.
    if !has_value(&claude, "") {
        set_string(&claude, "", "URL:Claude")?;
    }
    if !has_value(&claude, "URL Protocol") {
        set_string(&claude, "URL Protocol", "")?;
    }
    set_string(&create(HKEY_CURRENT_USER, CLAUDE_COMMAND_KEY)?, "", command)?;

    let prog = create(HKEY_CURRENT_USER, PROG_ID_KEY)?;
    set_string(&prog, "", "clawshell sign-in link")?;
    set_string(
        &create(
            HKEY_CURRENT_USER,
            &format!(r"{PROG_ID_KEY}\shell\open\command"),
        )?,
        "",
        command,
    )?;

    let capabilities = create(HKEY_CURRENT_USER, CAPABILITIES_KEY)?;
    set_string(&capabilities, "ApplicationName", APP_NAME)?;
    set_string(
        &capabilities,
        "ApplicationDescription",
        "Routes Claude sign-in links to the account that asked for them",
    )?;
    set_string(
        &create(
            HKEY_CURRENT_USER,
            &format!(r"{CAPABILITIES_KEY}\URLAssociations"),
        )?,
        "claude",
        PROG_ID,
    )?;
    set_string(
        &create(HKEY_CURRENT_USER, REGISTERED_APPS_KEY)?,
        APP_NAME,
        CAPABILITIES_KEY,
    )
}

/// Puts `snapshot` back exactly and removes what [`register`] added.
pub fn restore(snapshot: &Snapshot) -> io::Result<()> {
    delete_tree(CLAUDE_KEY)?;
    if let Some(image) = &snapshot.claude {
        write_image(&create(HKEY_CURRENT_USER, CLAUDE_KEY)?, image)?;
    }
    delete_tree(PROG_ID_KEY)?;
    delete_tree(CAPABILITIES_KEY)?;
    // SAFETY: plain calls with a valid root; the app key is only removed once
    // empty, and a refusal just means something else lives there.
    unsafe {
        let _ = RegDeleteKeyW(HKEY_CURRENT_USER, &HSTRING::from(APP_KEY));
    }
    if let Some(apps) = open(HKEY_CURRENT_USER, REGISTERED_APPS_KEY, KEY_SET_VALUE)? {
        // SAFETY: the key is open for writing.
        let status = unsafe { RegDeleteValueW(apps.0, &HSTRING::from(APP_NAME)) };
        if !is_missing(status) {
            check(status)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_snapshot_survives_its_text_form() {
        let snapshot = Snapshot {
            claude: Some(KeyImage {
                values: vec![ValueImage {
                    name: String::new(),
                    kind: 1,
                    data: to_hex(&utf16_bytes("URL:Claude")),
                }],
                keys: vec![(
                    "shell".into(),
                    KeyImage {
                        values: vec![],
                        keys: vec![],
                    },
                )],
            }),
        };
        assert_eq!(Snapshot::from_text(&snapshot.to_text()).unwrap(), snapshot);
        let absent = Snapshot { claude: None };
        assert_eq!(Snapshot::from_text(&absent.to_text()).unwrap(), absent);
    }

    #[test]
    fn a_damaged_snapshot_is_refused_not_guessed_at() {
        assert!(Snapshot::from_text("not json").is_err());
        assert_eq!(from_hex("0g"), None);
        assert_eq!(from_hex("abc"), None);
        assert_eq!(from_hex("00ff"), Some(vec![0, 255]));
    }

    #[test]
    fn the_command_quotes_the_path_and_takes_the_link_as_one_argument() {
        assert_eq!(
            handler_command(Path::new(r"C:\Program Files\clawshell\clawshell.exe")),
            r#""C:\Program Files\clawshell\clawshell.exe" "%1""#
        );
    }
}
