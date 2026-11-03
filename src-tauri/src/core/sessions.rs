//! An account's Code sessions: the records Claude Desktop keeps at
//! `claude-code-sessions/{accountUuid}/{orgUuid}/local_{id}.json`.

use super::Core;
use super::account_error::{AccountError, wrap_account_error};
use super::copy_target::{CopyBlocker, target_folder};
use super::cost::SESSIONS_DIR;
use super::paths::same_path;
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};

/// One Code session of an account, as its record says.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeSession {
    /// Claude Code's session id. Claude sometimes clears it, keeping the
    /// old one in `priorCliSessionIds`; such a record goes by its own
    /// `sessionId`, which a copy keeps too.
    pub cli_session_id: String,
    pub title: String,
    /// The folder the session works in.
    pub cwd: String,
    /// Unix milliseconds.
    pub last_activity_at: i64,
    pub is_archived: bool,
}

/// What a copy did.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CopyOutcome {
    /// Sessions written into the target.
    pub copied: usize,
    /// The target's Claude is running, so it lists the copies only after a
    /// restart.
    pub target_running: bool,
}

#[derive(Debug)]
pub enum SessionsError {
    Account(AccountError),
    /// A session record doesn't read the way Claude Desktop writes it, so
    /// Claude has probably changed its layout. Nothing is listed rather than
    /// a part of the sessions.
    LayoutChanged,
    /// The target can't receive copies yet; nothing was written.
    TargetBlocked(CopyBlocker),
    /// The target is the source account itself.
    SameAccount,
}

wrap_account_error!(SessionsError);

impl Core {
    /// Every Code session of the account, from every organisation, newest
    /// activity first. A session kept under two organisations is listed once.
    pub fn list_sessions(&self, id: &str) -> Result<Vec<CodeSession>, SessionsError> {
        let account = self.find_account(id)?.ok_or(AccountError::Unknown)?;
        let mut sessions: Vec<CodeSession> = newest(records(&account.path)?)
            .into_values()
            .map(|(_, session)| session)
            .collect();
        sessions.sort_by(|a, b| {
            (b.last_activity_at, &a.cli_session_id).cmp(&(a.last_activity_at, &b.cli_session_id))
        });
        Ok(sessions)
    }

    /// Copies the source's sessions named by `cli_session_ids` into the
    /// target's current account and organisation folder, under the same file
    /// names. A record the target already keeps for the same session, in any
    /// of its folders, is replaced. Records are copied unchanged, and the
    /// source keeps its own. Every record of both profiles is read before
    /// anything is written, so a layout clawshell doesn't know leaves no
    /// half-written copy.
    pub fn copy_sessions(
        &self,
        source_id: &str,
        cli_session_ids: &[String],
        target_id: &str,
    ) -> Result<CopyOutcome, SessionsError> {
        let source = self.find_account(source_id)?.ok_or(AccountError::Unknown)?;
        let target = self.find_account(target_id)?.ok_or(AccountError::Unknown)?;
        if source.id == target.id {
            return Err(SessionsError::SameAccount);
        }
        let ids = {
            let usage = self.usage.lock().unwrap();
            usage.accounts.get(&target.id).and_then(|u| u.ids.clone())
        };
        let folder = target_folder(target.status, target.disabled, &target.path, ids.as_ref())
            .map_err(SessionsError::TargetBlocked)?;

        let mut wanted = newest(records(&source.path)?);
        wanted.retain(|id, _| cli_session_ids.contains(id));
        let existing = records(&target.path)?;

        for (id, (file, _)) in &wanted {
            let name = file_name(file);
            let copy = folder.join(name);
            // Not a `local_*.json` name, so Claude never reads it half-written.
            let partial = folder.join(format!("{name}.clawshell-partial"));
            std::fs::copy(file, &partial)?;
            std::fs::rename(&partial, &copy)?;
            for (old, session) in &existing {
                if &session.cli_session_id == id && !same_path(old, &copy) {
                    std::fs::remove_file(old)?;
                }
            }
        }
        Ok(CopyOutcome {
            copied: wanted.len(),
            target_running: target.running,
        })
    }
}

/// Every live session record of the profile, from every account and
/// organisation folder, with its file.
fn records(profile: &Path) -> Result<Vec<(PathBuf, CodeSession)>, SessionsError> {
    let mut found = Vec::new();
    for account_dir in subdirs(&profile.join(SESSIONS_DIR))? {
        for org_dir in subdirs(&account_dir)? {
            for file in records_in(&org_dir)? {
                let session = parse(&file).ok_or(SessionsError::LayoutChanged)?;
                found.push((file, session));
            }
        }
    }
    Ok(found)
}

/// The newest record of each session, so one kept under two organisations
/// counts once.
fn newest(records: Vec<(PathBuf, CodeSession)>) -> HashMap<String, (PathBuf, CodeSession)> {
    let mut found: HashMap<String, (PathBuf, CodeSession)> = HashMap::new();
    for (file, session) in records {
        let newer = found
            .get(&session.cli_session_id)
            .is_none_or(|(_, s)| s.last_activity_at < session.last_activity_at);
        if newer {
            found.insert(session.cli_session_id.clone(), (file, session));
        }
    }
    found
}

/// The live folders directly in `dir`; none when `dir` doesn't exist.
fn subdirs(dir: &Path) -> io::Result<Vec<PathBuf>> {
    Ok(entries(dir)?
        .into_iter()
        .filter(|p| p.is_dir() && !is_removed(p))
        .collect())
}

/// Claude moves removed sessions into `deleted_*` folders.
fn is_removed(dir: &Path) -> bool {
    file_name(dir).starts_with("deleted_")
}

fn records_in(dir: &Path) -> io::Result<Vec<PathBuf>> {
    Ok(entries(dir)?
        .into_iter()
        .filter(|p| {
            let name = file_name(p);
            name.starts_with("local_") && name.ends_with(".json")
        })
        .collect())
}

fn entries(dir: &Path) -> io::Result<Vec<PathBuf>> {
    match std::fs::read_dir(dir) {
        Ok(entries) => entries.map(|e| e.map(|e| e.path())).collect(),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(e) => Err(e),
    }
}

fn file_name(path: &Path) -> &str {
    path.file_name().and_then(|n| n.to_str()).unwrap_or("")
}

fn parse(file: &Path) -> Option<CodeSession> {
    let json: Value = serde_json::from_str(&std::fs::read_to_string(file).ok()?).ok()?;
    let text = |key: &str| json.get(key).and_then(Value::as_str).map(str::to_string);
    Some(CodeSession {
        cli_session_id: text("cliSessionId").or_else(|| text("sessionId"))?,
        title: text("title").unwrap_or_default(),
        cwd: text("cwd").unwrap_or_default(),
        last_activity_at: json.get("lastActivityAt")?.as_i64()?,
        is_archived: json
            .get("isArchived")
            .and_then(Value::as_bool)
            .unwrap_or(false),
    })
}
