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

