use super::*;
use ed25519_dalek::{Signer, SigningKey};
use serde_json::json;
use sha2::{Digest, Sha256};

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn extract_first_sha256_from_stdout(stdout: &[u8]) -> String {
    let text = String::from_utf8(stdout.to_vec()).expect("stdout utf8");
    let marker = "[sha256:";
    let start = text.find(marker).expect("stdout contains sha256 marker") + marker.len();
    let rest = &text[start..];
    let end = rest.find(']').expect("sha256 marker closed");
    rest[..end].to_string()
}

fn canonicalize_json_value(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut keys: Vec<_> = map.keys().cloned().collect();
            keys.sort();
            let mut out = serde_json::Map::new();
            for key in keys {
                out.insert(
                    key.clone(),
                    canonicalize_json_value(map.get(key.as_str()).expect("key exists")),
                );
            }
            Value::Object(out)
        }
        Value::Array(items) => Value::Array(items.iter().map(canonicalize_json_value).collect()),
        _ => value.clone(),
    }
}

fn canonical_json_bytes(value: &Value) -> Vec<u8> {
    serde_json::to_vec(&canonicalize_json_value(value)).expect("serialize canonical json")
}

fn current_clg_version_requirement() -> String {
    let current_full = env!("CARGO_PKG_VERSION");
    let current_core = current_full
        .split(['-', '+'])
        .next()
        .unwrap_or(current_full);
    let mut parts = current_core.split('.');
    let major = parts
        .next()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(0);
    let minor = parts
        .next()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(1);
    format!("^{}.{}.0", major, minor)
}

fn with_current_clg_version_requirement(template: &str) -> String {
    template.replace(
        "\"clg_version\": \"^0.1.0\"",
        format!(
            "\"clg_version\": \"{}\"",
            current_clg_version_requirement()
        )
        .as_str(),
    )
}


include!("core_resolution.rs");
include!("core_lockfile.rs");
include!("core_migrate.rs");
