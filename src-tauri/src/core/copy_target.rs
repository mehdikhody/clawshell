//! Whether an account can receive copied Code sessions, and why not.

use super::accounts::Status;
use super::cost::SESSIONS_DIR;
use serde::Serialize;
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// Why an account can't receive session copies yet. Each is something the
/// user can fix, so the copy dialog shows it beside the account.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CopyBlocker {
    /// "Sign in to this account first": signed out, or the profile endpoint
    /// hasn't yet told us which account and organisation it is signed into.
    SignIn,
    /// "Open the Code tab in this account once first": Claude hasn't created
    /// the account's session folder yet.
    OpenCodeTab,
}

/// The account and organisation a profile is signed into now, as the OAuth
/// profile endpoint reports them. They name the folder Claude keeps the
/// profile's Code sessions in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ProfileIdentity {
    account: String,
    organization: String,
}

impl ProfileIdentity {
    /// Only well-formed uuids count: both become part of a path.
    pub(super) fn parse(account: &str, organization: &str) -> Option<Self> {
        let canonical = |id: &str| Uuid::parse_str(id).ok().map(|u| u.to_string());
        Some(ProfileIdentity {
            account: canonical(account)?,
            organization: canonical(organization)?,
        })
    }
}

/// The folder a copied session record goes in, or what stops the account
/// receiving one.
pub(super) fn target_folder(
    status: Status,
    profile: &Path,
    ids: Option<&ProfileIdentity>,
) -> Result<PathBuf, CopyBlocker> {
    if matches!(status, Status::SignedOut | Status::Missing) {
        return Err(CopyBlocker::SignIn);
    }
    let ids = ids.ok_or(CopyBlocker::SignIn)?;
    let folder = profile
        .join(SESSIONS_DIR)
        .join(&ids.account)
        .join(&ids.organization);
    if folder.is_dir() {
        Ok(folder)
    } else {
        Err(CopyBlocker::OpenCodeTab)
    }
}

/// The reason the account can't receive copies; `None` when it can.
pub(super) fn copy_blocker(
    status: Status,
    profile: &Path,
    ids: Option<&ProfileIdentity>,
) -> Option<CopyBlocker> {
    target_folder(status, profile, ids).err()
}
