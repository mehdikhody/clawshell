//! Per-account environment variables and extra launch flags: validation and
//! the assembly of a launch. Pure functions; the port does the starting.

use serde::{Deserialize, Serialize};
use std::path::Path;

/// The arguments for starting Claude on `profile`: the user-data-dir switch
/// first, then `flags`, each one argument as given.
pub fn launch_args(profile: &Path, flags: &[String]) -> Vec<String> {
    let mut args = vec![format!("--user-data-dir={}", profile.display())];
    args.extend(flags.iter().cloned());
    args
}

/// One environment variable. The value can be a secret, so `Debug` hides it.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnvVar {
    pub name: String,
    pub value: String,
}

impl std::fmt::Debug for EnvVar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EnvVar")
            .field("name", &self.name)
            .finish_non_exhaustive()
    }
}

/// What an account adds to every start of its Claude.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LaunchOptions {
    /// Set on that account's Claude process only, in this order.
    pub env: Vec<EnvVar>,
    /// Extra command-line arguments, one argument per entry.
    pub flags: Vec<String>,
}

#[derive(Debug)]
pub enum LaunchOptionsError {
    UnknownAccount,
    /// A name is empty, holds anything but letters, digits and `_`, or starts
    /// with a digit.
    InvalidEnvName,
    /// A name appears twice (Windows ignores case in names).
    DuplicateEnvName,
    /// A value holds a NUL character, which no process environment can carry.
    InvalidEnvValue,
    /// A flag would move Claude off the account's profile folder.
    ReservedFlag,
    Io(std::io::Error),
}

impl From<std::io::Error> for LaunchOptionsError {
    fn from(e: std::io::Error) -> Self {
        LaunchOptionsError::Io(e)
    }
}

/// The switch that names the profile folder; only clawshell may set it.
const PROFILE_SWITCH: &str = "--user-data-dir";

fn is_valid_env_name(name: &str) -> bool {
    !name.starts_with(|c: char| c.is_ascii_digit())
        && !name.is_empty()
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Checks the options and tidies them: flags are trimmed and blank ones
/// dropped. Errors never carry a name or a value, since values can be secrets.
pub fn validate(options: LaunchOptions) -> Result<LaunchOptions, LaunchOptionsError> {
    let mut seen: Vec<String> = Vec::new();
    for var in &options.env {
        if !is_valid_env_name(&var.name) {
            return Err(LaunchOptionsError::InvalidEnvName);
        }
        let key = var.name.to_ascii_uppercase();
        if seen.contains(&key) {
            return Err(LaunchOptionsError::DuplicateEnvName);
        }
        seen.push(key);
        if var.value.contains('\0') {
            return Err(LaunchOptionsError::InvalidEnvValue);
        }
    }
    let flags: Vec<String> = options
        .flags
        .iter()
        .map(|f| f.trim())
        .filter(|f| !f.is_empty())
        .map(str::to_string)
        .collect();
    let reserved = |flag: &String| {
        let flag = flag.to_ascii_lowercase();
        flag == PROFILE_SWITCH || flag.starts_with(&format!("{PROFILE_SWITCH}="))
    };
    if flags.iter().any(reserved) {
        return Err(LaunchOptionsError::ReservedFlag);
    }
    Ok(LaunchOptions {
        env: options.env,
        flags,
    })
}
