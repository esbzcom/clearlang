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

