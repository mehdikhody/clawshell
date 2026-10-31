//! Desktop token cache: decrypting `oauth:tokenCacheV2` and unwrapping the
//! AES key blob. Fixtures are synthetic; nothing here touches a real profile.

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use clawshell_lib::core::desktop_token::{TokenError, decrypt_v10, dpapi_blob_from_encrypted_key};

fn hex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

// AES-256-GCM test case 15 from the GCM specification (McGrew & Viega).
const KEY: &str = "feffe9928665731c6d6a8f9467308308feffe9928665731c6d6a8f9467308308";
const IV: &str = "cafebabefacedbaddecaf888";
const PLAIN: &str = "d9313225f88406e5a55909c5aff5269a86a7a9531534f7da2e4c303d8a318a721c3c0c95956809532fcf0e2449a6b525b16aedf5aa0de657ba637b391aafd255";
const CIPHER: &str = "522dc1f099567d07f47f37a32a84427d643a8cdcbfe5c0c97598a2bd2555d1aa8cb08e48590dbb3da7b08b1056828838c5f61e6393ba7a0abcc9f662898015ad";
const TAG: &str = "b094dac5d93471bdec1a502270e3cc6c";

fn v10_value() -> String {
    let mut raw = b"v10".to_vec();
    raw.extend(hex(IV));
    raw.extend(hex(CIPHER));
    raw.extend(hex(TAG));
    STANDARD.encode(raw)
}

#[test]
fn decrypts_a_v10_value_with_the_aes_key() {
    let plain = decrypt_v10(&v10_value(), &hex(KEY)).unwrap();
    assert_eq!(plain, hex(PLAIN));
}

#[test]
fn rejects_a_wrong_key_without_leaking_anything() {
    let err = decrypt_v10(&v10_value(), &[0u8; 32]).unwrap_err();
    assert_eq!(err, TokenError::Decrypt);
}

#[test]
fn rejects_a_value_without_the_v10_prefix() {
    let raw = STANDARD.encode(b"v11-not-this-one-at-all-padding-padding");
    assert_eq!(decrypt_v10(&raw, &hex(KEY)).unwrap_err(), TokenError::NotV10);
}

#[test]
fn encrypted_key_loses_its_dpapi_marker() {
    let blob = STANDARD.encode([b"DPAPI".as_slice(), &[1, 2, 3, 4]].concat());
    assert_eq!(dpapi_blob_from_encrypted_key(&blob).unwrap(), vec![1, 2, 3, 4]);
}

#[test]
fn encrypted_key_without_the_marker_is_rejected() {
    let blob = STANDARD.encode([9, 9, 9, 9, 9, 9]);
    assert_eq!(
        dpapi_blob_from_encrypted_key(&blob).unwrap_err(),
        TokenError::NotDpapi
    );
}

// ---- token cache structure -------------------------------------------------
// The decrypted cache is a JSON object keyed by
// `acct:{id}:{id}:{auth base url}:{scopes}`; each value has `token`,
// `refreshToken`, `expiresAt` (epoch ms) and a few flags.

use clawshell_lib::core::desktop_token::extract_token;
use serde_json::json;

const KEY_PROFILE_ONLY: &str = "acct:a:b:https://api.anthropic.com:user:profile";
const KEY_WIDE: &str = "acct:a:b:https://api.anthropic.com:user:inference user:file_upload user:profile";
const KEY_NO_PROFILE: &str = "acct:a:b:https://api.anthropic.com:user:inference";

#[test]
fn extracts_the_token_and_expiry_from_a_single_entry() {
    let cache = json!({ KEY_PROFILE_ONLY: { "token": "tok-1", "refreshToken": "r", "expiresAt": 1_900_000_000_000_i64 } });
    let t = extract_token(&cache).unwrap();
    assert_eq!(t.access_token, "tok-1");
    assert_eq!(t.expires_at_ms, Some(1_900_000_000_000));
}

#[test]
fn prefers_the_entry_that_expires_last_among_those_with_profile_scope() {
    let cache = json!({
        KEY_PROFILE_ONLY: { "token": "older", "expiresAt": 1_900_000_000_000_i64 },
        KEY_WIDE: { "token": "newer", "expiresAt": 1_900_000_500_000_i64 },
        KEY_NO_PROFILE: { "token": "unscoped", "expiresAt": 1_999_999_999_999_i64 },
    });
    assert_eq!(extract_token(&cache).unwrap().access_token, "newer");
}

#[test]
fn falls_back_to_any_entry_when_none_has_profile_scope() {
    let cache = json!({ KEY_NO_PROFILE: { "token": "only", "expiresAt": 1_900_000_000_000_i64 } });
    assert_eq!(extract_token(&cache).unwrap().access_token, "only");
}

#[test]
fn a_missing_expiry_is_unknown_not_expired() {
    let cache = json!({ KEY_PROFILE_ONLY: { "token": "tok" } });
    assert_eq!(extract_token(&cache).unwrap().expires_at_ms, None);
}

#[test]
fn entries_without_a_token_string_are_skipped() {
    let cache = json!({
        KEY_PROFILE_ONLY: { "token": "", "expiresAt": 1_i64 },
        KEY_WIDE: { "refreshToken": "r" },
    });
    assert!(extract_token(&cache).is_none());
}

#[test]
fn does_not_leak_the_token_through_debug_output() {
    let cache = json!({ KEY_PROFILE_ONLY: { "token": "super-secret", "expiresAt": 1_i64 } });
    let shown = format!("{:?}", extract_token(&cache).unwrap());
    assert!(!shown.contains("super-secret"));
}
