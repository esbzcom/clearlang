#[test]
fn sign_and_verify_roundtrip() {
    let tmp = tempdir().unwrap();
    let (wasm_path, sig_path, pub_path) = build_signed_module(tmp.path());
    let sig_value: serde_json::Value =
        serde_json::from_slice(&fs::read(&sig_path).expect("read signature")).expect("json");
    let assurance = sig_value
        .get("payload")
        .and_then(|v| v.get("assurance"))
        .and_then(|v| v.as_object())
        .expect("payload assurance");
    assert_eq!(assurance.get("tier").and_then(|v| v.as_str()), Some("L1"));
    assert_eq!(
        assurance.get("label").and_then(|v| v.as_str()),
        Some("checked core")
    );
    assert_eq!(
        assurance
            .get("levels")
            .and_then(|v| v.get("L2"))
            .and_then(|v| v.as_str()),
        Some("verified module")
    );
    let assurance_claim = sig_value
        .get("payload")
        .and_then(|v| v.get("assurance_claim"))
        .and_then(|v| v.as_object())
        .expect("payload assurance_claim");
    assert_eq!(
        assurance_claim
            .get("compiler_mode")
            .and_then(|v| v.as_str()),
        Some("standard")
    );
    assert_eq!(
        assurance_claim
            .get("non_strict_evidence_only")
            .and_then(|v| v.as_bool()),
        Some(true)
    );
    assert_eq!(
        assurance_claim
            .get("release_grade_trust")
            .and_then(|v| v.as_bool()),
        Some(false)
    );
    assert_eq!(
        sig_value
            .get("payload")
            .and_then(|v| v.get("proof_status"))
            .and_then(|v| v.as_str()),
        Some("not_proved_all")
    );

    let mut verify = Command::cargo_bin("clg").expect("bin");
    verify
        .args(["verify"])
        .arg("--module")
        .arg(&wasm_path)
        .arg("--sig")
        .arg(&sig_path)
        .arg("--pubkey")
        .arg(&pub_path);
    verify.assert().success();
}

#[test]
fn verify_explain_emits_checked_core_summary() {
    let tmp = tempdir().unwrap();
    let (wasm_path, sig_path, pub_path) = build_signed_module(tmp.path());

    let mut verify = Command::cargo_bin("clg").expect("bin");
    verify
        .args(["verify", "--explain"])
        .arg("--module")
        .arg(&wasm_path)
        .arg("--sig")
        .arg(&sig_path)
        .arg("--pubkey")
        .arg(&pub_path);
    let stdout = verify.assert().success().get_output().stdout.clone();
    let text = String::from_utf8(stdout).expect("utf8 stdout");
    assert!(text.contains("Verification explanation"));
    assert!(text.contains("proof_status: not_proved_all"));
    assert!(text.contains("assurance: L1 (checked core)"));
    assert!(text.contains("assurance_claim: non-strict evidence only"));
    assert!(text.contains("assumed_boundaries: none"));
}

#[test]
fn verify_explain_includes_assumed_boundary_reasoning() {
    let tmp = tempdir().unwrap();
    let (wasm_path, sig_path, pub_path) =
        build_signed_module_from_source_with_manifest_and_trust_anchors(
            tmp.path(),
            sample_source_with_assumptions(),
            None,
            None,
            None,
        );

    let mut verify = Command::cargo_bin("clg").expect("bin");
    verify
        .args(["verify", "--explain"])
        .arg("--module")
        .arg(&wasm_path)
        .arg("--sig")
        .arg(&sig_path)
        .arg("--pubkey")
        .arg(&pub_path);
    let stdout = verify.assert().success().get_output().stdout.clone();
    let text = String::from_utf8(stdout).expect("utf8 stdout");
    assert!(text.contains("vcs_with_assumptions:"));
    assert!(text.contains("assumed_boundaries:"));
    assert!(text.contains("crypto.uninterpreted"));
    assert!(text.contains("why="));
}

#[test]
fn build_emits_signed_assurance_manifest() {
    let tmp = tempdir().unwrap();
    let manifest = tmp.path().join("out.assurance.json");
    let (_wasm_path, _sig_path, pub_path) = build_signed_module_with_manifest_and_trust_anchors(
        tmp.path(),
        Some(&manifest),
        None,
        None,
    );

    let manifest_value: serde_json::Value =
        serde_json::from_slice(&fs::read(&manifest).expect("read manifest")).expect("json");
    assert_eq!(
        manifest_value
            .get("schema_version")
            .and_then(|v| v.as_u64()),
        Some(1)
    );

    let payload = manifest_value.get("payload").expect("payload");
    assert_eq!(
        payload
            .get("format")
            .and_then(|v| v.as_str())
            .expect("format"),
        "clg.assurance_manifest.v1"
    );
    assert_eq!(
        payload
            .get("assurance")
            .and_then(|v| v.get("tier"))
            .and_then(|v| v.as_str()),
        Some("L1")
    );
    assert!(payload
        .get("assumptions")
        .and_then(|v| v.get("items"))
        .and_then(|v| v.as_array())
        .is_some());
    assert!(payload
        .get("dependency_trust_labels")
        .and_then(|v| v.as_array())
        .is_some());
    assert!(payload
        .get("toolchain")
        .and_then(|v| v.get("fingerprint_sha256"))
        .and_then(|v| v.as_str())
        .is_some());
    assert_eq!(
        payload
            .get("assurance_claim")
            .and_then(|v| v.get("compiler_mode"))
            .and_then(|v| v.as_str()),
        Some("standard")
    );
    assert_eq!(
        payload
            .get("assurance_claim")
            .and_then(|v| v.get("non_strict_evidence_only"))
            .and_then(|v| v.as_bool()),
        Some(true)
    );
    assert_eq!(
        payload
            .get("assurance_claim")
            .and_then(|v| v.get("release_grade_trust"))
            .and_then(|v| v.as_bool()),
        Some(false)
    );
    assert_eq!(
        payload.get("proof_status").and_then(|v| v.as_str()),
        Some("not_proved_all")
    );

    let payload_hash = manifest_value
        .get("signature")
        .and_then(|v| v.get("payload_hash"))
        .and_then(|v| v.as_str())
        .expect("payload_hash");
    let canonical_payload = canonical_json_string(payload);
    assert_eq!(payload_hash, sha256_hex(canonical_payload.as_bytes()));

    let signature_hex = manifest_value
        .get("signature")
        .and_then(|v| v.get("signature"))
        .and_then(|v| v.as_str())
        .expect("signature hex");
    let signature_bytes = hex::decode(signature_hex).expect("decode signature");
    let signature = ed25519_dalek::Signature::from_slice(&signature_bytes).expect("signature");

    let pub_json: serde_json::Value =
        serde_json::from_slice(&fs::read(pub_path).expect("read pubkey")).expect("pubkey json");
    let pub_hex = pub_json
        .get("public_key")
        .and_then(|v| v.as_str())
        .expect("public key");
    let pub_key_bytes: [u8; 32] = hex::decode(pub_hex)
        .expect("decode public key")
        .try_into()
        .expect("public key length");
    let verifying = ed25519_dalek::VerifyingKey::from_bytes(&pub_key_bytes).expect("verify key");
    verifying
        .verify_strict(canonical_payload.as_bytes(), &signature)
        .expect("manifest signature should verify");
}

#[test]
fn build_sign_defaults_assurance_manifest_path_from_sig_out() {
    let tmp = tempdir().unwrap();
    let (_wasm_path, sig_path, _pub_path) = build_signed_module(tmp.path());
    let manifest_path = sig_path.with_file_name("out.assurance.json");
    assert!(
        manifest_path.exists(),
        "expected default assurance manifest at {}",
        manifest_path.display()
    );
}

#[test]
fn verify_fails_when_proofs_hash_tampered() {
    let tmp = tempdir().unwrap();
    let (wasm_path, sig_path, pub_path) = build_signed_module(tmp.path());
    tamper_proofs_hash(&wasm_path);

    let mut verify = Command::cargo_bin("clg").expect("bin");
    verify
        .args(["--json-errors", "verify"])
        .arg("--module")
        .arg(&wasm_path)
        .arg("--sig")
        .arg(&sig_path)
        .arg("--pubkey")
        .arg(&pub_path);
    verify.assert().failure().stdout(
        predicate::str::contains("\"code\": \"V003\"")
            .and(predicate::str::contains("hash mismatch")),
    );
}

#[test]
fn verify_fails_when_proof_section_missing() {
    let tmp = tempdir().unwrap();
    let (wasm_path, sig_path, pub_path) = build_signed_module(tmp.path());
    strip_proof_section(&wasm_path);

    let mut verify = Command::cargo_bin("clg").expect("bin");
    verify
        .args(["--json-errors", "verify"])
        .arg("--module")
        .arg(&wasm_path)
        .arg("--sig")
        .arg(&sig_path)
        .arg("--pubkey")
        .arg(&pub_path);
    verify.assert().failure().stdout(
        predicate::str::contains("\"code\": \"V002\"")
            .and(predicate::str::contains("proof section not found")),
    );
}

#[test]
fn verify_fails_when_signature_is_invalid() {
    let tmp = tempdir().unwrap();
    let (wasm_path, sig_path, pub_path) = build_signed_module(tmp.path());
    tamper_signature(&sig_path);

    let mut verify = Command::cargo_bin("clg").expect("bin");
    verify
        .args(["--json-errors", "verify"])
        .arg("--module")
        .arg(&wasm_path)
        .arg("--sig")
        .arg(&sig_path)
        .arg("--pubkey")
        .arg(&pub_path);
    verify.assert().failure().stdout(
        predicate::str::contains("\"code\": \"V001\"")
            .and(predicate::str::contains("signature verification failed")),
    );
}

#[test]
fn verify_compile_time_requires_trust_policy() {
    let tmp = tempdir().unwrap();
    let (wasm_path, sig_path, pub_path) = build_signed_module(tmp.path());

    let mut verify = Command::cargo_bin("clg").expect("bin");
    verify
        .args(["--json-errors", "verify"])
        .arg("--module")
        .arg(&wasm_path)
        .arg("--sig")
        .arg(&sig_path)
        .arg("--pubkey")
        .arg(&pub_path)
        .args(["--verify-mode", "compile-time"]);
    verify.assert().failure().stdout(
        predicate::str::contains("\"code\": \"V004\"")
            .and(predicate::str::contains("requires `--trust-policy <FILE>`")),
    );
}

#[test]
fn verify_runtime_rejects_trust_policy() {
    let tmp = tempdir().unwrap();
    let (wasm_path, sig_path, pub_path) = build_signed_module(tmp.path());
    let trust_policy = write_trust_policy(tmp.path(), "4.14.0", "8.19.2");

    let mut verify = Command::cargo_bin("clg").expect("bin");
    verify
        .args(["--json-errors", "verify"])
        .arg("--module")
        .arg(&wasm_path)
        .arg("--sig")
        .arg(&sig_path)
        .arg("--pubkey")
        .arg(&pub_path)
        .arg("--trust-policy")
        .arg(&trust_policy);
    verify
        .assert()
        .failure()
        .stdout(
            predicate::str::contains("\"code\": \"V004\"").and(predicate::str::contains(
                "requires `--verify-mode compile-time`",
            )),
        );
}

#[test]
fn verify_compile_time_fails_when_signature_missing_trust_anchors() {
    let tmp = tempdir().unwrap();
    let (wasm_path, sig_path, pub_path) = build_signed_module(tmp.path());
    let trust_policy = write_trust_policy(tmp.path(), "4.14.0", "8.19.2");

    let mut verify = Command::cargo_bin("clg").expect("bin");
    verify
        .args(["--json-errors", "verify"])
        .arg("--module")
        .arg(&wasm_path)
        .arg("--sig")
        .arg(&sig_path)
        .arg("--pubkey")
        .arg(&pub_path)
        .args(["--verify-mode", "compile-time"])
        .arg("--trust-policy")
        .arg(&trust_policy);
    verify
        .assert()
        .failure()
        .stdout(
            predicate::str::contains("\"code\": \"V004\"").and(predicate::str::contains(
                "signature payload missing trust_anchors",
            )),
        );
}

#[test]
fn verify_compile_time_fails_when_trust_policy_mismatches_signature() {
    let tmp = tempdir().unwrap();
    let (wasm_path, sig_path, pub_path) =
        build_signed_module_with_trust_anchors(tmp.path(), Some("4.14.0"), Some("8.19.2"));
    let trust_policy = write_trust_policy(tmp.path(), "4.15.0", "8.19.2");

    let mut verify = Command::cargo_bin("clg").expect("bin");
    verify
        .args(["--json-errors", "verify"])
        .arg("--module")
        .arg(&wasm_path)
        .arg("--sig")
        .arg(&sig_path)
        .arg("--pubkey")
        .arg(&pub_path)
        .args(["--verify-mode", "compile-time"])
        .arg("--trust-policy")
        .arg(&trust_policy);
    verify
        .assert()
        .failure()
        .stdout(
            predicate::str::contains("\"code\": \"V004\"").and(predicate::str::contains(
                "trust-anchor checker version mismatch",
            )),
        );
}

#[test]
fn verify_compile_time_succeeds_when_trust_policy_matches_signature() {
    let tmp = tempdir().unwrap();
    let (wasm_path, sig_path, pub_path) =
        build_signed_module_with_trust_anchors(tmp.path(), Some("4.14.0"), Some("8.19.2"));
    let trust_policy = write_trust_policy(tmp.path(), "4.14.0", "8.19.2");

    let mut verify = Command::cargo_bin("clg").expect("bin");
    verify
        .args(["verify"])
        .arg("--module")
        .arg(&wasm_path)
        .arg("--sig")
        .arg(&sig_path)
        .arg("--pubkey")
        .arg(&pub_path)
        .args(["--verify-mode", "compile-time"])
        .arg("--trust-policy")
        .arg(&trust_policy);
    verify.assert().success();
}

#[test]
fn verify_release_policy_succeeds_when_manifest_meets_required_tier() {
    let tmp = tempdir().unwrap();
    let (wasm_path, sig_path, pub_path) = build_signed_module(tmp.path());
    let manifest = sig_path.with_file_name("out.assurance.json");
    let policy = write_release_policy(tmp.path(), "L1");

    let mut verify = Command::cargo_bin("clg").expect("bin");
    verify
        .args(["verify"])
        .arg("--module")
        .arg(&wasm_path)
        .arg("--sig")
        .arg(&sig_path)
        .arg("--pubkey")
        .arg(&pub_path)
        .arg("--assurance-manifest")
        .arg(&manifest)
        .arg("--release-policy")
        .arg(&policy);
    verify.assert().success();
}

#[test]
fn verify_release_policy_rejects_manifest_below_required_tier_with_v005() {
    let tmp = tempdir().unwrap();
    let (wasm_path, sig_path, pub_path) = build_signed_module(tmp.path());
    let manifest = sig_path.with_file_name("out.assurance.json");
    let policy = write_release_policy(tmp.path(), "L2");

    let mut verify = Command::cargo_bin("clg").expect("bin");
    verify
        .args(["--json-errors", "verify"])
        .arg("--module")
        .arg(&wasm_path)
        .arg("--sig")
        .arg(&sig_path)
        .arg("--pubkey")
        .arg(&pub_path)
        .arg("--assurance-manifest")
        .arg(&manifest)
        .arg("--release-policy")
        .arg(&policy);
    verify.assert().failure().stdout(
        predicate::str::contains("\"code\": \"V005\"")
            .and(predicate::str::contains("below required")),
    );
}

#[test]
fn verify_release_policy_rejects_manifest_hash_mismatch_with_v005() {
    let tmp_a = tempdir().unwrap();
    let (wasm_path, sig_path, pub_path) = build_signed_module(tmp_a.path());
    let policy = write_release_policy(tmp_a.path(), "L0");

    let tmp_b = tempdir().unwrap();
    let (_other_wasm, other_sig, _other_pub) =
        build_signed_module_from_source_with_manifest_and_trust_anchors(
            tmp_b.path(),
            sample_source_with_assumptions(),
            None,
            None,
            None,
        );
    let mismatched_manifest = other_sig.with_file_name("out.assurance.json");

    let mut verify = Command::cargo_bin("clg").expect("bin");
    verify
        .args(["--json-errors", "verify"])
        .arg("--module")
        .arg(&wasm_path)
        .arg("--sig")
        .arg(&sig_path)
        .arg("--pubkey")
        .arg(&pub_path)
        .arg("--assurance-manifest")
        .arg(&mismatched_manifest)
        .arg("--release-policy")
        .arg(&policy);
    verify
        .assert()
        .failure()
        .stdout(
            predicate::str::contains("\"code\": \"V005\"").and(predicate::str::contains(
                "does not match verified signature payload hashes",
            )),
        );
}

#[test]
fn verify_release_policy_rejects_invalid_manifest_signature_with_v005() {
    let tmp = tempdir().unwrap();
    let (wasm_path, sig_path, pub_path) = build_signed_module(tmp.path());
    let manifest = sig_path.with_file_name("out.assurance.json");
    tamper_assurance_manifest_signature(&manifest);
    let policy = write_release_policy(tmp.path(), "L0");

    let mut verify = Command::cargo_bin("clg").expect("bin");
    verify
        .args(["--json-errors", "verify"])
        .arg("--module")
        .arg(&wasm_path)
        .arg("--sig")
        .arg(&sig_path)
        .arg("--pubkey")
        .arg(&pub_path)
        .arg("--assurance-manifest")
        .arg(&manifest)
        .arg("--release-policy")
        .arg(&policy);
    verify
        .assert()
        .failure()
        .stdout(
            predicate::str::contains("\"code\": \"V005\"").and(predicate::str::contains(
                "release policy manifest validation failed",
            )),
        );
}

#[test]
fn verify_require_assurance_rejects_not_proved_all_with_v005() {
    let tmp = tempdir().unwrap();
    let (wasm_path, sig_path, pub_path) = build_signed_module(tmp.path());

    let mut verify = Command::cargo_bin("clg").expect("bin");
    verify
        .args(["--json-errors", "verify"])
        .arg("--module")
        .arg(&wasm_path)
        .arg("--sig")
        .arg(&sig_path)
        .arg("--pubkey")
        .arg(&pub_path)
        .arg("--require-assurance")
        .arg("proved_all");
    verify.assert().failure().stdout(
        predicate::str::contains("\"code\": \"V005\"")
            .and(predicate::str::contains("required assurance `proved_all` not satisfied"))
            .and(predicate::str::contains("not_proved_all")),
    );
}

#[test]
fn verify_require_assurance_rejects_invalid_value_with_v005() {
    let tmp = tempdir().unwrap();
    let (wasm_path, sig_path, pub_path) = build_signed_module(tmp.path());

    let mut verify = Command::cargo_bin("clg").expect("bin");
    verify
        .args(["--json-errors", "verify"])
        .arg("--module")
        .arg(&wasm_path)
        .arg("--sig")
        .arg(&sig_path)
        .arg("--pubkey")
        .arg(&pub_path)
        .arg("--require-assurance")
        .arg("l3");
    verify.assert().failure().stdout(
        predicate::str::contains("\"code\": \"V005\"").and(predicate::str::contains(
            "invalid `--require-assurance` value",
        )),
    );
}

#[test]
fn verify_require_assurance_rejects_manifest_with_assumption_boundaries_when_proved_all() {
    let tmp = tempdir().unwrap();
    let manifest = tmp.path().join("out.assurance.json");
    let (wasm_path, sig_path, pub_path) =
        build_signed_module_from_source_with_manifest_and_trust_anchors(
            tmp.path(),
            sample_source_with_assumptions(),
            Some(&manifest),
            None,
            None,
        );

    rewrite_signature_payload_proof_status_and_resign(&sig_path, "proved_all");
    rewrite_assurance_manifest_payload_proof_status_and_resign(&manifest, "proved_all");
    let policy = write_release_policy(tmp.path(), "L0");

    let mut verify = Command::cargo_bin("clg").expect("bin");
    verify
        .args(["--json-errors", "verify"])
        .arg("--module")
        .arg(&wasm_path)
        .arg("--sig")
        .arg(&sig_path)
        .arg("--pubkey")
        .arg(&pub_path)
        .arg("--assurance-manifest")
        .arg(&manifest)
        .arg("--release-policy")
        .arg(&policy)
        .arg("--require-assurance")
        .arg("proved_all");
    verify.assert().failure().stdout(
        predicate::str::contains("\"code\": \"V005\"")
            .and(predicate::str::contains("zero assumption boundaries"))
            .and(predicate::str::contains("crypto.uninterpreted")),
    );
}
