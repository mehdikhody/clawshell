//! clawshell's own account records: labels live in `clawshell/accounts.json`
//! under local app data, kept apart from the profile folders themselves.

use super::badge::{is_valid_colour, next_colour};
use super::default_profile::{DEFAULT_COLOUR, DEFAULT_ID, DEFAULT_LABEL};
use super::launch_options::{EnvVar, LaunchOptions};
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
    /// `#rrggbb`; none in records saved before colours existed.
    #[serde(default)]
    pub colour: Option<String>,
    /// Environment variables for this account's Claude process only.
    #[serde(default)]
    pub env: Vec<EnvVar>,
    /// Extra launch arguments for this account's Claude.
    #[serde(default)]
    pub flags: Vec<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Records {
    /// The accounts in the order the user chose; the order they are listed in.
    #[serde(default)]
    pub accounts: Vec<Record>,
    /// Set once the order is saved. Older files lack it: their accounts were
    /// listed by id, and keep that order until the first sync.
    #[serde(default)]
    pub ordered: bool,
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

    /// Gives an older file's accounts the id order they were listed in, the
    /// first time; true when that saved a new order.
    pub fn adopt_id_order(&mut self) -> bool {
        if self.ordered {
            return false;
        }
        self.accounts.sort_by(|a, b| a.id.cmp(&b.id));
        self.ordered = true;
        true
    }

    /// Where `id` sits in the saved order; an unknown id sorts last.
    pub fn position_of(&self, id: &str) -> usize {
        self.accounts
            .iter()
            .position(|r| r.id == id)
            .unwrap_or(usize::MAX)
    }

    /// Puts the records in the order of `ids`. Unknown ids are ignored, and a
    /// record `ids` does not name keeps its place after the named ones.
    pub fn reorder(&mut self, ids: &[String]) {
        let mut rest = std::mem::take(&mut self.accounts);
        let mut ordered = Vec::with_capacity(rest.len());
        for id in ids {
            if let Some(at) = rest.iter().position(|r| &r.id == id) {
                ordered.push(rest.remove(at));
            }
        }
        ordered.extend(rest);
        self.accounts = ordered;
        self.ordered = true;
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

    pub fn colour_of(&self, id: &str) -> Option<&str> {
        self.accounts
            .iter()
            .find(|r| r.id == id)
            .and_then(|r| r.colour.as_deref())
    }

    /// Recolours `id`; false when no record has that id.
    pub fn set_colour(&mut self, id: &str, colour: &str) -> bool {
        match self.accounts.iter_mut().find(|r| r.id == id) {
            Some(record) => {
                record.colour = Some(colour.to_string());
                true
            }
            None => false,
        }
    }

    /// Gives every record that has no usable colour yet (saved before colours
    /// existed) the next unused one; true when any was given.
    pub fn assign_missing_colours(&mut self) -> bool {
        let mut changed = false;
        for i in 0..self.accounts.len() {
            // A hand-edited colour that is not `#rrggbb` is replaced too: it
            // ends up in a style attribute.
            if !self.accounts[i]
                .colour
                .as_deref()
                .is_some_and(is_valid_colour)
            {
                let colour = next_colour(self.accounts.iter().filter_map(|r| r.colour.as_deref()));
                self.accounts[i].colour = Some(colour.to_string());
                changed = true;
            }
        }
        changed
    }

    /// The account's env vars and flags; empty for an unknown id.
    pub fn options_of(&self, id: &str) -> LaunchOptions {
        self.accounts
            .iter()
            .find(|r| r.id == id)
            .map(|r| LaunchOptions {
                env: r.env.clone(),
                flags: r.flags.clone(),
            })
            .unwrap_or_default()
    }

    /// Replaces the account's env vars and flags; false when no record has
    /// that id.
    pub fn set_options(&mut self, id: &str, options: LaunchOptions) -> bool {
        match self.accounts.iter_mut().find(|r| r.id == id) {
            Some(record) => {
                record.env = options.env;
                record.flags = options.flags;
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

    /// The colour a new account would get: the least used palette colour.
    pub fn fresh_colour(&self) -> &'static str {
        next_colour(self.accounts.iter().filter_map(|r| r.colour.as_deref()))
    }

    /// Records the default account under its label and colour the first
    /// time; true when that added it.
    pub fn ensure_default(&mut self) -> bool {
        if self.label_of(DEFAULT_ID).is_some() {
            return false;
        }
        self.insert(DEFAULT_ID, DEFAULT_LABEL, DEFAULT_COLOUR);
        true
    }

    /// Records `id` under a fresh default label and returns that label.
    pub fn add(&mut self, id: &str) -> String {
        let label = self.next_label();
        let colour = self.fresh_colour();
        self.insert(id, &label, colour);
        label
    }

    /// Records `id` under `label` and `colour`, at the end of the order.
    pub fn insert(&mut self, id: &str, label: &str, colour: &str) {
        self.accounts.push(Record {
            id: id.to_string(),
            label: label.to_string(),
            colour: Some(colour.to_string()),
            env: Vec::new(),
            flags: Vec::new(),
        });
    }
}
