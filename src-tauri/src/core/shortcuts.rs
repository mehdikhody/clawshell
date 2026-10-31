//! Per-account shortcuts: a "Claude - <name>" link on the Desktop and in the
//! Start menu, each with the account's badge icon, launching through clawshell.

use super::Core;
use super::account_error::{AccountError, wrap_account_error};
use super::accounts::{Account, LaunchError, LaunchOutcome, Status, is_account_id};
use super::ports::{ShortcutLink, ShortcutPlaces};
use std::io;
use std::path::PathBuf;

/// The command-line switch a shortcut passes: `--account <id>`.
pub const ACCOUNT_ARG: &str = "--account";
/// Longest label kept in a shortcut's name; keeps the path short.
const MAX_LABEL_CHARS: usize = 80;
/// clawshell's folder under local app data, and its icon sub-folder.
const APP_DATA_DIR: &str = "clawshell";
const ICON_DIR: &str = "icons";
/// Characters Windows forbids in a file name.
const FORBIDDEN: &str = r#"<>:"/\|?*"#;

/// The shortcut's file name, without the `.lnk`: "Claude - <label>", with
/// the characters Windows forbids in file names replaced.
pub fn shortcut_name(label: &str) -> String {
    let label: String = label
        .chars()
        .take(MAX_LABEL_CHARS)
        .map(|c| {
            if c.is_control() || FORBIDDEN.contains(c) {
                '_'
            } else {
                c
            }
        })
        .collect();
    format!("Claude - {}", label.trim_end_matches(['.', ' ']))
}

/// The arguments a shortcut passes to clawshell for the account `id`.
pub fn launch_args(id: &str) -> String {
    format!("{ACCOUNT_ARG} {id}")
}

/// The account a command line asks clawshell to open (`--account <id>` or
/// `--account=<id>`), when it names a well-formed account id.
pub fn account_to_open(args: &[String]) -> Option<String> {
    let prefix = format!("{ACCOUNT_ARG}=");
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        let id = if arg == ACCOUNT_ARG {
            args.next()
        } else {
            None
        };
        let id = id.map(String::as_str).or_else(|| arg.strip_prefix(&prefix));
        if let Some(id) = id.filter(|id| is_account_id(id)) {
            return Some(id.to_string());
        }
    }
    None
}

/// An `.ico` file from square images given as `(side, straight-alpha BGRA
/// pixels, top row first)`. Each image is stored as a 32-bit BMP with an
/// empty AND mask, which every Windows version reads.
pub fn encode_ico(images: &[(usize, Vec<u8>)]) -> Vec<u8> {
    const HEADER: usize = 6;
    const ENTRY: usize = 16;
    const BMP_HEADER: usize = 40;

    let le16 = |n: usize| (n as u16).to_le_bytes();
    let le32 = |n: usize| (n as u32).to_le_bytes();

    // Each image: BMP header, pixels bottom row first, then the AND mask
    // (one bit per pixel, every row padded to four bytes, all zero = opaque).
    let bodies: Vec<Vec<u8>> = images
        .iter()
        .map(|(side, pixels)| {
            assert_eq!(pixels.len(), side * side * 4, "BGRA pixels for a square");
            let mask = side.div_ceil(32) * 4 * side;
            let mut body = Vec::with_capacity(BMP_HEADER + pixels.len() + mask);
            body.extend(le32(BMP_HEADER));
            body.extend(le32(*side));
            body.extend(le32(side * 2));
            body.extend(le16(1));
            body.extend(le16(32));
            body.extend([0; 24]);
            for row in pixels.chunks(side * 4).rev() {
                body.extend_from_slice(row);
            }
            body.resize(body.len() + mask, 0);
            body
        })
        .collect();

    let mut ico = Vec::new();
    ico.extend([0, 0, 1, 0]);
    ico.extend(le16(images.len()));
    let mut offset = HEADER + ENTRY * images.len();
    for ((side, _), body) in images.iter().zip(&bodies) {
        // A side of 256 is stored as 0.
        let side_byte = if *side >= 256 { 0 } else { *side as u8 };
        ico.extend([side_byte, side_byte, 0, 0]);
        ico.extend(le16(1));
        ico.extend(le16(32));
        ico.extend(le32(body.len()));
        ico.extend(le32(offset));
        offset += body.len();
    }
    for body in bodies {
        ico.extend(body);
    }
    ico
}

/// Why shortcuts could not be added.
#[derive(Debug)]
pub enum ShortcutError {
    Account(AccountError),
    /// The account's profile folder is gone; there is nothing to launch.
    FolderMissing,
    /// Claude's own Start-menu entry already opens the default account.
    DefaultAccount,
}

wrap_account_error!(ShortcutError);

impl Core {
    /// Handles a shortcut click: the account named on the command line is
    /// started, or brought forward when it is already running. `None` when the
    /// command line names no account (a plain launch of clawshell).
    pub fn open_requested(&self, args: &[String]) -> Option<Result<LaunchOutcome, LaunchError>> {
        account_to_open(args).map(|id| self.launch_account(&id))
    }

    /// Creates (or replaces) the account's Desktop and Start-menu shortcuts.
    /// They launch clawshell itself, never a Claude exe, so a Claude update
    /// cannot break them.
    pub fn add_shortcuts(&self, id: &str) -> Result<(), ShortcutError> {
        let account = self
            .known_accounts()?
            .into_iter()
            .find(|a| a.id == id)
            .ok_or(AccountError::Unknown)?;
        if account.is_default {
            return Err(ShortcutError::DefaultAccount);
        }
        if account.status == Status::Missing {
            return Err(ShortcutError::FolderMissing);
        }
        let places = self.ports.os.places()?;
        let links = link_paths(&places, &account.label);
        self.write_shortcuts(&account, &places, &links)?;
        Ok(())
    }

    /// Keeps the shortcuts that exist in step with a rename or a new colour:
    /// the link takes the new name and the icon the new badge. A shortcut the
    /// user never added, or deleted by hand, stays absent. Best effort: the
    /// change itself is already saved.
    pub(super) fn update_shortcuts(&self, before: &Account, after: &Account) {
        // Claude's own Start-menu entry is the default account's shortcut.
        if after.is_default {
            return;
        }
        let Ok(places) = self.ports.os.places() else {
            return;
        };
        let old = link_paths(&places, &before.label);
        let new = link_paths(&places, &after.label);
        let present: Vec<bool> = old.iter().map(|link| link.exists()).collect();
        let wanted: Vec<PathBuf> = new
            .iter()
            .zip(&present)
            .filter(|(_, present)| **present)
            .map(|(link, _)| link.clone())
            .collect();
        if wanted.is_empty() {
            return;
        }
        if self.write_shortcuts(after, &places, &wanted).is_err() {
            return;
        }
        // The old link has another name; the old icon another badge or colour.
        for (old, new) in old.iter().zip(&new) {
            if old != new {
                let _ = self.ports.os.delete(old);
            }
        }
        if self.icon_path(before) != self.icon_path(after) {
            let _ = self.ports.os.delete(&self.icon_path(before));
        }
    }

    /// Removes the account's shortcuts and icon. Best effort: the account is
    /// already gone, so stale shortcuts are not fatal.
    pub(super) fn remove_shortcuts(&self, account: &Account) {
        if let Ok(places) = self.ports.os.places() {
            for link in link_paths(&places, &account.label) {
                let _ = self.ports.os.delete(&link);
            }
        }
        let _ = self.ports.os.delete(&self.icon_path(account));
    }

    /// Writes the account's icon, then a shortcut at each of `links`.
    fn write_shortcuts(
        &self,
        account: &Account,
        places: &ShortcutPlaces,
        links: &[PathBuf],
    ) -> io::Result<()> {
        let icon = self.icon_path(account);
        self.ports
            .os
            .write_icon(&icon, account.badge, &account.colour)?;
        let spec = ShortcutLink {
            target: places.clawshell_exe.clone(),
            args: launch_args(&account.id),
            icon,
            description: format!("Open {} in Claude", account.label),
        };
        for link in links {
            self.ports.os.write_link(link, &spec)?;
        }
        Ok(())
    }

    /// The account's badge icon file. The badge is in the name: Explorer
    /// caches an icon by path, so a new badge needs a new path.
    fn icon_path(&self, account: &Account) -> PathBuf {
        let colour = account.colour.trim_start_matches('#').to_ascii_lowercase();
        self.ports
            .roots
            .local_app_data
            .join(APP_DATA_DIR)
            .join(ICON_DIR)
            .join(format!(
                "{}-{:x}-{colour}.ico",
                account.id,
                u32::from(account.badge)
            ))
    }
}

/// Where the account's shortcuts live: the Desktop, then the Start menu.
fn link_paths(places: &ShortcutPlaces, label: &str) -> [PathBuf; 2] {
    let file = format!("{}.lnk", shortcut_name(label));
    [places.desktop.join(&file), places.start_menu.join(file)]
}
