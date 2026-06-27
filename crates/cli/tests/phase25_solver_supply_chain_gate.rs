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

#[test]
fn solver_supply_chain_gate_covers_all_staged_release_target_bundles() {
    let root = repo_root();
    let support_matrix = read_json(
        root.join("docs")
            .join("design")
            .join("phase-25.1.15-solver-support-matrix.lock.json")
            .as_path(),
    );
    let supply_chain = read_json(
        root.join("docs")
            .join("design")
            .join("phase-25.1.16-solver-supply-chain.lock.json")
            .as_path(),
    );

    let allowed_statuses = supply_chain["bundle_integrity"]["rotation_policy"]["allowed_statuses"]
        .as_array()
        .expect("allowed_statuses[]");
    let trusted_signers = supply_chain["bundle_integrity"]["trusted_signers"]
        .as_array()
        .expect("trusted_signers[]");
    let trusted_key_ids = trusted_signers
        .iter()
        .filter(|entry| {
            let status = entry["status"].as_str().expect("trusted_signers[].status");
            allowed_statuses
                .iter()
                .filter_map(Value::as_str)
                .any(|allowed| allowed == status)
        })
        .map(|entry| {
            entry["key_id"]
                .as_str()
                .expect("trusted_signers[].key_id")
                .to_string()
        })
        .collect::<Vec<_>>();

    let release_targets = support_matrix["release_target_platforms"]
        .as_array()
        .expect("release_target_platforms[]");
    for target in release_targets {
        let target = target.as_str().expect("release target");
        let bundle = match target {
            "windows-x64" => root
                .join("tools")
                .join("proof")
                .join("z3")
                .join("windows")
                .join("z3.exe"),
            "linux-x64-glibc-2.39" => root
                .join("tools")
                .join("proof")
                .join("z3")
                .join("linux")
                .join("z3"),
            "macos-x64-15.7.3" => root
                .join("tools")
                .join("proof")
                .join("z3")
                .join("macos")
                .join("z3"),
            other => panic!("unexpected release target `{other}`"),
        };
        let checksum_path = bundle.parent().expect("bundle parent").join(format!(
            "{}.sha256",
            bundle
                .file_name()
                .expect("bundle filename")
                .to_string_lossy()
        ));
        let signature_path = bundle.parent().expect("bundle parent").join(format!(
            "{}.sig",
            bundle
                .file_name()
                .expect("bundle filename")
                .to_string_lossy()
        ));

        let checksum = fs::read_to_string(&checksum_path)
            .unwrap_or_else(|err| panic!("read `{}`: {err}", checksum_path.display()));
        let checksum = checksum
            .lines()
            .next()
            .expect("checksum first line")
            .trim()
            .to_string();
        assert!(
            checksum.starts_with("sha256:"),
            "solver checksum must use sha256 prefix for `{target}`"
        );

        let signature = read_json(signature_path.as_path());
        assert_eq!(signature["schema_version"], Value::from(1));
        assert_eq!(signature["scheme"], Value::String("ed25519".to_string()));
        assert_eq!(
            signature["signed_payload"],
            Value::String(checksum),
            "signed payload must match checksum entry for `{target}`"
        );
        let key_id = signature["key_id"].as_str().expect("signature.key_id");
        assert!(
            trusted_key_ids.iter().any(|trusted| trusted == key_id),
            "signature key_id `{key_id}` must be trusted for `{target}`"
        );
    }
}
