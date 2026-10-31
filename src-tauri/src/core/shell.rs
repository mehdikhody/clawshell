//! Decisions about the window and the tray, kept out of the Tauri layer so
//! they can be tested: what Close does, whether a launch shows the window,
//! and what the tray menu offers.

use super::{Core, Status};
use std::io;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloseAction {
    HideToTray,
    Quit,
}

/// One row of the tray menu, in display order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrayEntry {
    Open,
    Separator,
    /// A ticked row is an account with a running Claude.
    Account {
        id: String,
        label: String,
        running: bool,
    },
    AddAccount,
    Quit,
}

impl Core {
    /// What the window's close button does, per "close to tray".
    pub fn close_action(&self) -> CloseAction {
        if self.settings().close_to_tray {
            CloseAction::HideToTray
        } else {
            CloseAction::Quit
        }
    }

    /// Whether the window opens at launch. A launch made by Windows at
    /// sign-in (`boot`) stays in the tray when "start hidden" is on.
    pub fn shows_window_at_launch(&self, boot: bool) -> bool {
        !(boot && self.settings().start_hidden)
    }

    pub fn tray_entries(&self) -> io::Result<Vec<TrayEntry>> {
        let mut entries = vec![TrayEntry::Open, TrayEntry::Separator];
        let accounts: Vec<_> = self
            .list_accounts()?
            .accounts
            .into_iter()
            .filter(|a| a.status != Status::Missing)
            .map(|a| TrayEntry::Account {
                id: a.id,
                label: a.label,
                running: a.running,
            })
            .collect();
        if !accounts.is_empty() {
            entries.extend(accounts);
            entries.push(TrayEntry::Separator);
        }
        entries.push(TrayEntry::AddAccount);
        entries.push(TrayEntry::Quit);
        Ok(entries)
    }
}
