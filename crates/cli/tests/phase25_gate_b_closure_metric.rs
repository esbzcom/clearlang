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
fn gate_b_closure_metric_lock_matches_release_boundary_requirements() {
    let root = repo_root();
    let lock = read_json(
        root.join("docs")
            .join("design")
            .join("phase-25.1.21-gate-b-closure-metric.lock.json"),
    );
    assert_eq!(lock["schema_version"], Value::from(1));
    assert_eq!(lock["policy_version"], Value::String("25.1.21".to_string()));
    let zero_boundaries: Vec<&str> = lock["zero_assumption_boundaries_for_release"]
        .as_array()
        .expect("zero_assumption_boundaries_for_release[]")
        .iter()
        .map(|entry| entry.as_str().expect("boundary should be string"))
        .collect();
    assert_eq!(
        zero_boundaries,
        vec![
            "unsigned.int_model",
            "bitwise.uninterpreted",
            "crypto.uninterpreted"
        ],
        "gate-b closure boundaries should match release zero-boundary rule"
    );
    let gate_lock = read_json(
        root.join("docs")
            .join("evidence")
            .join("milestone_3-proof-gate.lock.json"),
    );
    let prohibited: Vec<&str> = gate_lock["prohibited_release_assumption_boundaries"]
        .as_array()
        .expect("prohibited_release_assumption_boundaries[]")
        .iter()
        .map(|entry| entry.as_str().expect("prohibited boundary string"))
        .collect();
    assert_eq!(
        prohibited, zero_boundaries,
        "milestone_3 gate lock prohibited boundaries must align with Gate-B closure metric"
    );
}

#[test]
fn gate_b_closure_metric_requires_deterministic_evidence_tests_and_release_flags() {
    let root = repo_root();
    let lock = read_json(
        root.join("docs")
            .join("design")
            .join("phase-25.1.21-gate-b-closure-metric.lock.json"),
    );
    let deterministic_tests: Vec<&str> = lock["deterministic_evidence_tests"]
        .as_array()
        .expect("deterministic_evidence_tests[]")
        .iter()
        .map(|entry| entry.as_str().expect("deterministic test command"))
        .collect();
    let gate_lock = read_json(
        root.join("docs")
            .join("evidence")
            .join("milestone_3-proof-gate.lock.json"),
    );
    let required_ci_tests: Vec<&str> = gate_lock["required_ci_tests"]
        .as_array()
        .expect("required_ci_tests[]")
        .iter()
        .map(|entry| entry.as_str().expect("required ci test command"))
        .collect();
    for command in deterministic_tests {
        assert!(
            required_ci_tests.contains(&command),
            "milestone_3 gate lock should include deterministic evidence test `{command}`"
        );
    }

    let required_release_flags: Vec<&str> = lock["required_release_commands"]
        .as_array()
        .expect("required_release_commands[]")
        .iter()
        .map(|entry| entry.as_str().expect("release command fragment"))
        .collect();
    let release_process =
        fs::read_to_string(root.join("docs").join("release-process.md")).expect("release process");
    for flag in required_release_flags {
        assert!(
            release_process.contains(flag),
            "release-process.md should include required closure flag `{flag}`"
        );
    }
}
