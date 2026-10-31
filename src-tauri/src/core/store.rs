//! clawshell's own account records: labels live in `clawshell/accounts.json`
//! under local app data, kept apart from the profile folders themselves.

use serde::{Deserialize, Serialize};
use std::io;
use std::path::{Path, PathBuf};

const APP_DATA_DIR: &str = "clawshell";
const FILE: &str = "accounts.json";
/// The original Claude install counts as Account 1, so clawshell starts at 2.
const FIRST_LABEL_NUMBER: u32 = 2;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Record {
    pub id: String,
    pub label: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Records {
    #[serde(default)]
    pub accounts: Vec<Record>,
}

fn file(local_app_data: &Path) -> PathBuf {
    local_app_data.join(APP_DATA_DIR).join(FILE)
}

impl Records {
    /// Missing or unreadable records start empty; labels are re-derived.
    pub fn load(local_app_data: &Path) -> Self {
        std::fs::read_to_string(file(local_app_data))
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, local_app_data: &Path) -> io::Result<()> {
        let path = file(local_app_data);
        std::fs::create_dir_all(path.parent().expect("file sits in a folder"))?;
        let tmp = path.with_extension("json.tmp");
        std::fs::write(
            &tmp,
            serde_json::to_vec_pretty(self).map_err(io::Error::other)?,
        )?;
        std::fs::rename(tmp, path)
    }

    pub fn label_of(&self, id: &str) -> Option<&str> {
        self.accounts
            .iter()
            .find(|r| r.id == id)
            .map(|r| r.label.as_str())
    }

    /// Relabels `id`; false when no record has that id.
    pub fn rename(&mut self, id: &str, label: &str) -> bool {
        match self.accounts.iter_mut().find(|r| r.id == id) {
            Some(record) => {
                record.label = label.to_string();
                true
            }
            None => false,
        }
    }

    pub fn remove(&mut self, id: &str) {
        self.accounts.retain(|r| r.id != id);
    }

    /// The smallest "Account N" (N >= 2) no record already uses.
    pub fn next_label(&self) -> String {
        (FIRST_LABEL_NUMBER..)
            .map(|n| format!("Account {n}"))
            .find(|label| self.accounts.iter().all(|r| &r.label != label))
            .expect("an unused label exists")
    }

    /// Records `id` under a fresh default label and returns that label.
    pub fn add(&mut self, id: &str) -> String {
        let label = self.next_label();
        self.accounts.push(Record {
            id: id.to_string(),
            label: label.clone(),
        });
        label
    }
}
