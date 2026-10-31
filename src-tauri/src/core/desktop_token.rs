//! Reading the OAuth token cache Claude Desktop keeps in a profile.
//!
//! `config.json` holds `oauth:tokenCacheV2` as base64 of `v10` + 12-byte nonce
//! + AES-256-GCM ciphertext (tag last). The AES key lives in `Local State`
//! under `os_crypt.encrypted_key`, DPAPI-wrapped. Unwrapping with DPAPI is the
//! Windows adapter's job; this module is the pure part. Plaintext stays in
//! memory and never reaches a log or an error message.

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;

const V10: &[u8] = b"v10";
const DPAPI: &[u8] = b"DPAPI";
const NONCE_LEN: usize = 12;
const TAG_LEN: usize = 16;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenError {
    Base64,
    NotV10,
    NotDpapi,
    BadKey,
    Decrypt,
}

/// Decrypts a base64 `v10` value with the profile's AES-256 key.
pub fn decrypt_v10(value_b64: &str, key: &[u8]) -> Result<Vec<u8>, TokenError> {
    let raw = STANDARD.decode(value_b64.trim()).map_err(|_| TokenError::Base64)?;
    let body = raw.strip_prefix(V10).ok_or(TokenError::NotV10)?;
    if body.len() < NONCE_LEN + TAG_LEN {
        return Err(TokenError::Decrypt);
    }
    let (nonce, sealed) = body.split_at(NONCE_LEN);
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|_| TokenError::BadKey)?;
    cipher
        .decrypt(Nonce::from_slice(nonce), sealed)
        .map_err(|_| TokenError::Decrypt)
}

/// `os_crypt.encrypted_key` -> the raw DPAPI blob to hand to CryptUnprotectData.
pub fn dpapi_blob_from_encrypted_key(encrypted_key_b64: &str) -> Result<Vec<u8>, TokenError> {
    let raw = STANDARD
        .decode(encrypted_key_b64.trim())
        .map_err(|_| TokenError::Base64)?;
    raw.strip_prefix(DPAPI)
        .map(<[u8]>::to_vec)
        .ok_or(TokenError::NotDpapi)
}

/// A decrypted access token and its expiry (epoch milliseconds, when known).
pub struct AccessToken {
    pub access_token: String,
    pub expires_at_ms: Option<i64>,
}

impl std::fmt::Debug for AccessToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AccessToken")
            .field("access_token", &"<redacted>")
            .field("expires_at_ms", &self.expires_at_ms)
            .finish()
    }
}

/// Picks the access token out of the decrypted token cache: a JSON object
/// keyed `acct:{id}:{id}:{base url}:{scopes}`, each value holding `token`
/// and `expiresAt`. Entries carrying the `user:profile` scope are preferred
/// (both endpoints need it); among candidates the latest expiry wins.
pub fn extract_token(cache: &serde_json::Value) -> Option<AccessToken> {
    cache
        .as_object()?
        .iter()
        .filter_map(|(key, entry)| {
            let token = entry.get("token")?.as_str().filter(|t| !t.is_empty())?;
            let expires_at_ms = entry.get("expiresAt").and_then(serde_json::Value::as_i64);
            let has_profile_scope = key.contains("user:profile");
            Some((has_profile_scope, expires_at_ms, token, expires_at_ms))
        })
        .max_by_key(|(profile, expiry, ..)| (*profile, *expiry))
        .map(|(_, _, token, expires_at_ms)| AccessToken {
            access_token: token.to_string(),
            expires_at_ms,
        })
}
