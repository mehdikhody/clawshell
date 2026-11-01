//! Decisions about the window and the tray, kept out of the Tauri layer so
//! they can be tested: what Close does, whether a launch shows the window,
//! and what the tray menu offers.

use super::{Core, Language, Status};
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

/// The fixed words of the tray menu, in the saved language.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrayLabels {
    pub open: &'static str,
    pub add_account: &'static str,
    pub quit: &'static str,
}

impl Language {
    fn tray_labels(self) -> TrayLabels {
        match self {
            Language::En => TrayLabels {
                open: "Open clawshell",
                add_account: "Add account",
                quit: "Quit",
            },
            Language::Ar => TrayLabels {
                open: "فتح clawshell",
                add_account: "إضافة حساب",
                quit: "خروج",
            },
            Language::Ru => TrayLabels {
                open: "Открыть clawshell",
                add_account: "Добавить аккаунт",
                quit: "Выход",
            },
            Language::Zh => TrayLabels {
                open: "打开 clawshell",
                add_account: "添加账户",
                quit: "退出",
            },
            Language::Fa => TrayLabels {
                open: "باز کردن clawshell",
                add_account: "افزودن حساب",
                quit: "خروج",
            },
        }
    }
}

impl Core {
    pub fn tray_labels(&self) -> TrayLabels {
        self.settings().language.tray_labels()
    }

    /// What the window's close button does, per "close to tray". Until the
    /// disclaimer is accepted it quits: there is no tray to hide to.
    pub fn close_action(&self) -> CloseAction {
        let settings = self.settings();
        if settings.close_to_tray && self.disclaimer_status().accepted {
            CloseAction::HideToTray
        } else {
            CloseAction::Quit
        }
    }

    /// Whether the window opens at launch. A launch made by Windows at
    /// sign-in (`boot`) stays in the tray when "start hidden" is on, but not
    /// before the disclaimer is accepted.
    pub fn shows_window_at_launch(&self, boot: bool) -> bool {
        !(boot && self.settings().start_hidden && self.disclaimer_status().accepted)
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
