use serde_json::Value;
use std::fs;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

fn read_json(path: PathBuf) -> Value {
    let raw = fs::read(path).expect("read json file");
    serde_json::from_slice(raw.as_slice()).expect("parse json file")
}

#[test]
fn crypto_release_closure_lock_blocks_crypto_surfaces_for_release() {
    let root = repo_root();
    let lock = read_json(
        root.join("docs")
            .join("design")
            .join("phase-25.1.19-crypto-release-closure.lock.json"),
    );
    assert_eq!(lock["schema_version"], Value::from(1));
    assert_eq!(lock["policy_version"], Value::String("25.1.19".to_string()));
    assert_eq!(
        lock["blocked_assumption_boundary"],
        Value::String("crypto.uninterpreted".to_string())
    );
    let release_enabled = lock["release_enabled_crypto_surfaces"]
        .as_array()
        .expect("release_enabled_crypto_surfaces[]");
    assert!(
        release_enabled.is_empty(),
        "current milestone scope should keep release-enabled crypto surfaces empty"
    );
}

#[test]
fn coverage_matrix_keeps_crypto_surfaces_non_proved_and_l3_blocked() {
    let root = repo_root();
    let lock = read_json(
        root.join("docs")
            .join("design")
            .join("phase-25.1.19-crypto-release-closure.lock.json"),
    );
    let matrix_rel = lock["coverage_matrix_path"]
        .as_str()
        .expect("coverage_matrix_path");
    let matrix = read_json(root.join(matrix_rel));
    let entries = matrix["entries"].as_array().expect("entries[]");
    let blocked_boundary = lock["blocked_assumption_boundary"]
        .as_str()
        .expect("blocked_assumption_boundary");
    let mut found_crypto = false;
    for entry in entries {
        let surface = entry["surface"].as_str().unwrap_or_default();
        let is_crypto_surface =
            surface == "std::bytes::eq_ct" || surface.starts_with("std::crypto::");
        if !is_crypto_surface {
            continue;
        }
        found_crypto = true;
        assert_ne!(
            entry["status"],
            Value::String("proved".to_string()),
            "crypto surfaces must not be marked proved for current release scope"
        );
        assert_eq!(
            entry["assumption_boundary"],
            Value::String(blocked_boundary.to_string()),
            "crypto surfaces must stay on blocked assumption boundary"
        );
        assert_eq!(
            entry["tiers"]["L3"],
            Value::String("blocked".to_string()),
            "crypto surfaces must stay L3 blocked"
        );
    }
    assert!(
        found_crypto,
        "coverage matrix should include crypto surfaces for closure checks"
    );
}

#[test]
fn crypto_release_boundary_gate_command_remains_in_ci_lock() {
    let root = repo_root();
    let lock = read_json(
        root.join("docs")
            .join("design")
            .join("phase-25.1.19-crypto-release-closure.lock.json"),
    );
    let required_command = lock["required_ci_command"]
        .as_str()
        .expect("required_ci_command");
    let gate_lock = read_json(
        root.join("docs")
            .join("evidence")
            .join("milestone_3-proof-gate.lock.json"),
    );
    let required_ci_tests = gate_lock["required_ci_tests"]
        .as_array()
        .expect("required_ci_tests[]");
    assert!(
        required_ci_tests
            .iter()
            .filter_map(Value::as_str)
            .any(|entry| entry == required_command),
        "milestone_3 gate lock should include required crypto closure command"
    );
}
