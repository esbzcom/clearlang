use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

fn read_json(path: &Path) -> Value {
    serde_json::from_slice(
        fs::read(path)
            .unwrap_or_else(|err| panic!("read `{}`: {err}", path.display()))
            .as_slice(),
    )
    .unwrap_or_else(|err| panic!("parse `{}`: {err}", path.display()))
}

#[test]
fn solver_supply_chain_lock_pins_version_integrity_and_docs() {
    let root = repo_root();
    let lock_path = root
        .join("docs")
        .join("design")
        .join("phase-25.1.16-solver-supply-chain.lock.json");
    let lock = read_json(lock_path.as_path());
    assert_eq!(lock["schema_version"], Value::from(1));
    assert_eq!(lock["policy_version"], Value::String("25.1.16".to_string()));
    assert_eq!(
        lock["solver_identity"]["family"],
        Value::String("z3".to_string())
    );
    assert_eq!(
        lock["bundle_integrity"]["checksum_algorithm"],
        Value::String("sha256".to_string())
    );
    assert_eq!(
        lock["bundle_integrity"]["checksum_required"],
        Value::Bool(true)
    );
    assert_eq!(
        lock["bundle_integrity"]["signature_required"],
        Value::Bool(true)
    );
    assert_eq!(
        lock["bundle_integrity"]["signature_mode"],
        Value::String("publisher-auth-ed25519-v1".to_string())
    );
    assert_eq!(
        lock["bundle_integrity"]["signature_schema_version"],
        Value::from(1),
        "solver bundle signature sidecar schema must be pinned"
    );
    let trusted_signers = lock["bundle_integrity"]["trusted_signers"]
        .as_array()
        .expect("bundle_integrity.trusted_signers[]");
    assert!(
        trusted_signers.iter().any(|entry| {
            entry["key_id"] == Value::String("z3-vendor-k7-2026q2".to_string())
                && entry["scheme"] == Value::String("ed25519".to_string())
                && entry["status"] == Value::String("active".to_string())
        }),
        "trusted signer set must include active pinned vendor key"
    );
    assert!(
        trusted_signers.iter().any(|entry| {
            entry["key_id"] == Value::String("z3-vendor-k8-2026q3".to_string())
                && entry["scheme"] == Value::String("ed25519".to_string())
                && entry["status"] == Value::String("next".to_string())
        }),
        "trusted signer set must include next rotation key"
    );
    let allowed_statuses = lock["bundle_integrity"]["rotation_policy"]["allowed_statuses"]
        .as_array()
        .expect("bundle_integrity.rotation_policy.allowed_statuses[]");
    assert!(
        allowed_statuses
            .iter()
            .filter_map(Value::as_str)
            .any(|status| status == "active"),
        "rotation policy must permit active signer status"
    );

    let profile_lock = read_json(
        root.join("docs")
            .join("design")
            .join("phase-25.1.4-solver-profile.lock.json")
            .as_path(),
    );
    assert_eq!(
        lock["solver_identity"]["version"], profile_lock["solver_profile"]["solver_version"],
        "supply-chain lock solver version must match deterministic solver-profile lock"
    );

    let license_files = lock["license_notice"]["required_files"]
        .as_array()
        .expect("license_notice.required_files[]");
    assert!(
        !license_files.is_empty(),
        "license_notice.required_files should not be empty"
    );
    for entry in license_files {
        let rel = entry
            .as_str()
            .expect("license_notice.required_files[] should be string paths");
        assert!(
            root.join(rel).exists(),
            "required license/notice file should exist: `{rel}`"
        );
    }

    let cve_doc = lock["cve_update_policy"]["policy_doc"]
        .as_str()
        .expect("cve_update_policy.policy_doc");
    assert!(
        root.join(cve_doc).exists(),
        "CVE policy doc should exist: `{cve_doc}`"
    );
    let rollback_doc = lock["rollback_procedure"]["doc"]
        .as_str()
        .expect("rollback_procedure.doc");
    assert!(
        root.join(rollback_doc).exists(),
        "rollback procedure doc should exist: `{rollback_doc}`"
    );
}

#[test]
fn solver_supply_chain_gate_is_wired_into_ci_and_release_evidence() {
    let root = repo_root();
    let lock = read_json(
        root.join("docs")
            .join("design")
            .join("phase-25.1.16-solver-supply-chain.lock.json")
            .as_path(),
    );
    let required_step = lock["ci_validation_strategy"]["required_step"]
        .as_str()
        .expect("ci_validation_strategy.required_step");
    let required_command = lock["ci_validation_strategy"]["required_command"]
        .as_str()
        .expect("ci_validation_strategy.required_command");
    let workflow_rel = lock["ci_validation_strategy"]["workflow"]
        .as_str()
        .expect("ci_validation_strategy.workflow");
    let workflow = fs::read_to_string(root.join(workflow_rel))
        .unwrap_or_else(|err| panic!("read `{workflow_rel}`: {err}"));
    assert!(
        workflow.contains(required_step),
        "workflow should include required step `{required_step}`"
    );
    assert!(
        workflow.contains(required_command),
        "workflow should include required command `{required_command}`"
    );

    let evidence_lock = read_json(
        root.join("docs")
            .join("evidence")
            .join("milestone_3-proof-gate.lock.json")
            .as_path(),
    );
    let required_ci_tests = evidence_lock["required_ci_tests"]
        .as_array()
        .expect("required_ci_tests[]");
    assert!(
        required_ci_tests
            .iter()
            .filter_map(Value::as_str)
            .any(|cmd| cmd == required_command),
        "milestone_3 proof gate lock should include solver supply-chain gate command"
    );
}
