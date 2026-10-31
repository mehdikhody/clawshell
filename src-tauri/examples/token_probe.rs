//! Developer-only feasibility probe (ticket 01). Run once by hand:
//!
//!   cargo run --example token_probe -- "<profile folder>"
//!
//! Decrypts the profile's token cache in memory, then calls the OAuth profile
//! and usage endpoints once each. Prints only HTTP status codes, response key
//! names, whether the token had expired, and token-cache key names. Never
//! prints, logs or writes a token, an email or any other value. No refresh.

#[cfg(windows)]
fn main() {
    use clawshell_lib::core::desktop_token as dt;
    use std::path::Path;

    let profile = std::env::args()
        .nth(1)
        .expect("usage: token_probe <profile folder>");
    let profile = Path::new(&profile);

    let read_json = |name: &str| -> serde_json::Value {
        let text = std::fs::read_to_string(profile.join(name)).unwrap_or_else(|_| {
            println!("cannot read {name}");
            std::process::exit(1)
        });
        serde_json::from_str(&text).unwrap_or_else(|_| {
            println!("{name} is not json");
            std::process::exit(1)
        })
    };

    let config = read_json("config.json");
    let local_state = read_json("Local State");
    let value = config["oauth:tokenCacheV2"].as_str().unwrap_or_else(|| {
        println!("no oauth:tokenCacheV2 in config.json");
        std::process::exit(1)
    });
    let wrapped = local_state["os_crypt"]["encrypted_key"]
        .as_str()
        .unwrap_or_else(|| {
            println!("no os_crypt.encrypted_key in Local State");
            std::process::exit(1)
        });

    let blob = dt::dpapi_blob_from_encrypted_key(wrapped).expect("encrypted_key shape");
    let key = dpapi_unprotect(&blob).unwrap_or_else(|| {
        println!("DPAPI unwrap failed");
        std::process::exit(1)
    });
    println!("aes key length: {}", key.len());
    let plain = dt::decrypt_v10(value, &key).unwrap_or_else(|e| {
        println!("decrypt failed: {e:?}");
        std::process::exit(1)
    });
    println!("decrypted token cache: {} bytes", plain.len());

    let json: serde_json::Value = match serde_json::from_slice(&plain) {
        Ok(v) => v,
        Err(_) => {
            println!("plaintext is not json");
            std::process::exit(1)
        }
    };
    println!("token cache structure (names only):");
    describe(&json, 0);

    let Some(token) = dt::extract_token(&json) else {
        println!("no access token found in the cache");
        std::process::exit(1)
    };
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;
    match token.expires_at_ms {
        Some(exp) => println!(
            "expiresAt digits: {} (13 = epoch ms); token expired: {}",
            exp.to_string().len(),
            exp <= now_ms
        ),
        None => println!("token expiry: not found"),
    }

    for (label, url, beta) in [
        (
            "profile",
            "https://api.anthropic.com/api/oauth/profile",
            false,
        ),
        ("usage", "https://api.anthropic.com/api/oauth/usage", true),
    ] {
        let mut req = ureq::get(url)
            .set("Authorization", &format!("Bearer {}", token.access_token))
            .set("Accept", "application/json");
        if beta {
            req = req.set("anthropic-beta", "oauth-2025-04-20");
        }
        let (status, body) = match req.call() {
            Ok(r) => (r.status(), r.into_string().unwrap_or_default()),
            Err(ureq::Error::Status(code, r)) => (code, r.into_string().unwrap_or_default()),
            Err(_) => {
                println!("{label}: network error");
                continue;
            }
        };
        println!("{label}: HTTP {status}");
        match serde_json::from_str::<serde_json::Value>(&body) {
            Ok(v) => {
                println!("{label} response keys (names only):");
                describe(&v, 0);
            }
            Err(_) => println!("{label}: body is not json"),
        }
    }
}

/// Prints key names, nesting and value types. Keys that look like ids or
/// emails are masked, and values are never shown.
#[cfg(windows)]
fn describe(v: &serde_json::Value, depth: usize) {
    use serde_json::Value;
    let pad = "  ".repeat(depth + 1);
    match v {
        Value::Object(map) => {
            for (k, child) in map {
                println!("{pad}{}: {}", mask(k), kind(child));
                if matches!(child, Value::Object(_) | Value::Array(_)) && depth < 4 {
                    describe(child, depth + 1);
                }
            }
        }
        Value::Array(items) => {
            if let Some(first) = items.first() {
                if matches!(first, Value::Object(_)) && depth < 4 {
                    describe(first, depth + 1);
                }
            }
        }
        _ => {}
    }
}

#[cfg(windows)]
fn kind(v: &serde_json::Value) -> &'static str {
    use serde_json::Value;
    match v {
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

#[cfg(windows)]
fn mask(key: &str) -> String {
    let plain_name = key.len() <= 32
        && key
            .chars()
            .all(|c| c.is_ascii_alphabetic() || c == '_' || c == '-' || c == ':')
        && !key.contains('@');
    if plain_name {
        key.to_string()
    } else {
        masked_key(key)
    }
}

#[cfg(windows)]
fn dpapi_unprotect(blob: &[u8]) -> Option<Vec<u8>> {
    use windows::Win32::Security::Cryptography::{CRYPT_INTEGER_BLOB, CryptUnprotectData};
    let input = CRYPT_INTEGER_BLOB {
        cbData: blob.len() as u32,
        pbData: blob.as_ptr() as *mut u8,
    };
    let mut output = CRYPT_INTEGER_BLOB::default();
    unsafe {
        CryptUnprotectData(&input, None, None, None, None, 0, &mut output).ok()?;
        Some(std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec())
    }
}

#[cfg(not(windows))]
fn main() {
    println!("token_probe is Windows-only (DPAPI)");
}

/// Shows the ":"-separated parts of a cache key, masking any part that has a digit or "@" (ids, emails).
#[cfg(windows)]
fn masked_key(key: &str) -> String {
    key.split(':')
        .map(|part| {
            if part.chars().any(|c| c.is_ascii_digit()) || part.contains('@') {
                "<id>".to_string()
            } else {
                part.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(":")
}
