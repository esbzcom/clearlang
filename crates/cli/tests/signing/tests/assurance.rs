#[test]
fn verify_require_assurance_rejects_forged_proof_status_when_module_derivation_disagrees() {
    let tmp = tempdir().unwrap();
    let manifest = tmp.path().join("out.assurance.json");
    let (wasm_path, sig_path, pub_path) = build_signed_module_with_manifest_and_trust_anchors(
        tmp.path(),
        Some(&manifest),
        None,
        None,
    );

    rewrite_signature_payload_proof_status_and_resign(&sig_path, "proved_all");
    rewrite_assurance_manifest_payload_proof_status_and_resign(&manifest, "proved_all");

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
        .arg("--require-assurance")
        .arg("proved_all");
    verify.assert().failure().stderr(predicate::str::contains(
        "does not match module proof section derived proof_status",
    ));
}

#[test]
fn verify_require_assurance_rejects_signature_missing_assumption_boundaries_with_v005() {
    let tmp = tempdir().unwrap();
    let (wasm_path, sig_path, pub_path) = build_signed_module(tmp.path());
    remove_signature_payload_fields_and_resign(&sig_path, &["assumption_boundaries"]);

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
        predicate::str::contains("\"code\": \"V005\"").and(predicate::str::contains(
            "invalid assumption_boundaries",
        )),
    );
}

#[test]
fn verify_require_assurance_rejects_signature_missing_bundle_symbols_with_v005() {
    let tmp = tempdir().unwrap();
    let (wasm_path, sig_path, pub_path) = build_signed_module(tmp.path());
    remove_signature_payload_fields_and_resign(&sig_path, &["bundle_symbols"]);

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
        predicate::str::contains("\"code\": \"V005\"").and(predicate::str::contains(
            "invalid bundle_symbols",
        )),
    );
}

#[test]
fn verify_require_assurance_rejects_bundle_symbols_mismatch_with_module_derivation() {
    let tmp = tempdir().unwrap();
    let (wasm_path, sig_path, pub_path) = build_signed_module(tmp.path());
    rewrite_signature_payload_bundle_symbols_and_resign(&sig_path, &["std::bytes::eq_ct"]);

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
            .and(predicate::str::contains("bundle_symbols"))
            .and(predicate::str::contains(
                "do not match module proof section derived bundle_symbols",
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
            .and(predicate::str::contains("derived proof_status"))
            .and(predicate::str::contains("not_proved_all")),
    );
}

#[test]
fn verify_require_assurance_rejects_symbols_outside_proved_allowlist_when_proved_all() {
    let tmp = tempdir().unwrap();
    let (wasm_path, sig_path, pub_path) = build_signed_module(tmp.path());

    rewrite_signature_payload_proof_status_and_symbols_and_resign(
        &sig_path,
        "proved_all",
        &["std::bytes::eq_ct"],
    );

    let matrix_path = repo_root()
        .join("docs")
        .join("proofs")
        .join("proof-coverage-matrix.json");

    let mut verify = Command::cargo_bin("clg").expect("bin");
    verify
        .env("CLG_PROOF_MATRIX_PATH", matrix_path)
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
            .and(predicate::str::contains("derived proof_status"))
            .and(predicate::str::contains("not_proved_all")),
    );
}

#[test]
fn verify_require_assurance_ignores_proof_matrix_env_override() {
    let tmp = tempdir().unwrap();
    let (wasm_path, sig_path, pub_path) = build_signed_module(tmp.path());

    let fake_symbol = "std::fake::always_disallowed";
    rewrite_signature_payload_proof_status_and_symbols_and_resign(
        &sig_path,
        "proved_all",
        &[fake_symbol],
    );

    let env_matrix_path = tmp.path().join("env-proof-matrix.json");
    fs::write(
        &env_matrix_path,
        serde_json::to_vec_pretty(&json!({
            "schema_version": 1,
            "entries": [
                {
                    "surface": fake_symbol,
                    "status": "proved"
                }
            ]
        }))
        .expect("serialize matrix"),
    )
    .expect("write matrix");

    let mut verify = Command::cargo_bin("clg").expect("bin");
    verify
        .env("CLG_PROOF_MATRIX_PATH", &env_matrix_path)
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
            .and(predicate::str::contains("derived proof_status"))
            .and(predicate::str::contains("not_proved_all")),
    );
}

#[test]
fn verify_accepts_matching_proof_artifact_claims() {
    let tmp = tempdir().unwrap();
    let manifest = tmp.path().join("out.assurance.json");
    let (wasm_path, sig_path, pub_path, proof_path) =
        build_signed_module_with_manifest_and_proof_artifact(tmp.path(), Some(&manifest));

    let sig_value: serde_json::Value =
        serde_json::from_slice(&fs::read(&sig_path).expect("read signature")).expect("json");
    assert!(sig_value
        .get("payload")
        .and_then(|payload| payload.get("proof_artifact_hash"))
        .and_then(|value| value.as_str())
        .is_some());
    assert!(sig_value
        .get("payload")
        .and_then(|payload| payload.get("solver_profile_hash"))
        .and_then(|value| value.as_str())
        .is_some());

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
        .arg("--proof-artifact")
        .arg(&proof_path);
    verify.assert().success();
}

#[test]
fn verify_rejects_missing_proof_artifact_when_claim_present_with_v006() {
    let tmp = tempdir().unwrap();
    let manifest = tmp.path().join("out.assurance.json");
    let (wasm_path, sig_path, pub_path, _proof_path) =
        build_signed_module_with_manifest_and_proof_artifact(tmp.path(), Some(&manifest));

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
        .arg(&manifest);
    verify.assert().failure().stdout(
        predicate::str::contains("\"code\": \"V006\"")
            .and(predicate::str::contains("proof artifact claim is present")),
    );
}

#[test]
fn verify_rejects_tampered_proof_artifact_with_v006() {
    let tmp = tempdir().unwrap();
    let manifest = tmp.path().join("out.assurance.json");
    let (wasm_path, sig_path, pub_path, proof_path) =
        build_signed_module_with_manifest_and_proof_artifact(tmp.path(), Some(&manifest));
    tamper_proof_artifact(&proof_path);

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
        .arg("--proof-artifact")
        .arg(&proof_path);
    verify.assert().failure().stdout(
        predicate::str::contains("\"code\": \"V006\"")
            .and(predicate::str::contains("proof artifact hash mismatch")),
    );
}
