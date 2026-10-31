//! An account's Code sessions: the records Claude Desktop keeps at
//! `claude-code-sessions/{accountUuid}/{orgUuid}/local_{id}.json`.

use super::Core;
use super::account_error::{AccountError, wrap_account_error};
use super::cost::SESSIONS_DIR;
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};

/// One Code session of an account, as its record says.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeSession {
    pub cli_session_id: String,
    pub title: String,
    /// The folder the session works in.
    pub cwd: String,
    /// Unix milliseconds.
    pub last_activity_at: i64,
    pub is_archived: bool,
}

#[derive(Debug)]
pub enum SessionsError {
    Account(AccountError),
    /// A session record doesn't read the way Claude Desktop writes it, so
    /// Claude has probably changed its layout. Nothing is listed rather than
    /// a part of the sessions.
    LayoutChanged,
}

wrap_account_error!(SessionsError);

impl Core {
    /// Every Code session of the account, from every organisation, newest
    /// activity first. A session kept under two organisations is listed once.
    pub fn list_sessions(&self, id: &str) -> Result<Vec<CodeSession>, SessionsError> {
        let account = self.find_account(id)?.ok_or(AccountError::Unknown)?;
        let root = account.path.join(SESSIONS_DIR);
        let mut found: HashMap<String, CodeSession> = HashMap::new();
        for account_dir in subdirs(&root)? {
            for org_dir in subdirs(&account_dir)? {
                for file in records_in(&org_dir)? {
                    let session = parse(&file).ok_or(SessionsError::LayoutChanged)?;
                    let newer = found
                        .get(&session.cli_session_id)
                        .is_none_or(|s| s.last_activity_at < session.last_activity_at);
                    if newer {
                        found.insert(session.cli_session_id.clone(), session);
                    }
                }
            }
        }
        let mut sessions: Vec<CodeSession> = found.into_values().collect();
        sessions.sort_by(|a, b| {
            (b.last_activity_at, &a.cli_session_id).cmp(&(a.last_activity_at, &b.cli_session_id))
        });
        Ok(sessions)
    }
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
        cli_session_id: text("cliSessionId")?,
        title: text("title").unwrap_or_default(),
        cwd: text("cwd").unwrap_or_default(),
        last_activity_at: json.get("lastActivityAt")?.as_i64()?,
        is_archived: json
            .get("isArchived")
            .and_then(Value::as_bool)
            .unwrap_or(false),
    })
}
