//! Signed-in check that needs no decryption: the heuristic the reference
//! switcher uses, read straight from the profile's `config.json`.

use std::path::Path;

const TOKEN_CACHE: &str = "oauth:tokenCacheV2";
const SIGNED_IN_FLAG: &str = "windowSizeWasSignedIn";
/// A token cache shorter than this holds no usable sign-in.
const MIN_TOKEN_CACHE_LEN: usize = 100;

/// `windowSizeWasSignedIn` decides when Claude recorded it; otherwise a token
/// cache longer than 100 characters means signed in. A missing or unreadable
/// config counts as signed out.
pub fn is_signed_in(profile: &Path) -> bool {
    let Ok(text) = std::fs::read_to_string(profile.join("config.json")) else {
        return false;
    };
    let Ok(config) = serde_json::from_str::<serde_json::Value>(&text) else {
        return false;
    };
    if let Some(flag) = config
        .get(SIGNED_IN_FLAG)
        .and_then(serde_json::Value::as_bool)
    {
        return flag;
    }
    config
        .get(TOKEN_CACHE)
        .and_then(serde_json::Value::as_str)
        .is_some_and(|cache| cache.len() > MIN_TOKEN_CACHE_LEN)
}
