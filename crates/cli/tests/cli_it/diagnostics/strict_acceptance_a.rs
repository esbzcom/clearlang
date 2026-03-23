#[test]
fn strict_acceptance_precompiled_std_core_rejects_intrinsic_fallback_with_c105() {
    let src = r#"
        function main() -> Int { std::bytes::len(std::bytes::from_string("abc")) }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp
        .path()
        .join("strict_acceptance_precompiled_std_core_fallback_reject.clear");
    fs::write(&file, src).expect("write");
    write_signed_strict_dependency_fixture(
        tmp.path(),
        r#"[
        {
          "symbol": "std::str::len",
          "effect": "pure",
          "params": ["String"],
          "ret": "Int",
          "capability": null
        }
      ]"#,
    );

    let out = tmp.path().join("out.wasm");
    let vcs = tmp.path().join("out.vc.json");
    let output = Command::cargo_bin("clg")
        .unwrap()
        .args(["--json-errors", "build"])
        .arg(&file)
        .args(["-o"])
        .arg(&out)
        .args(["--emit-vcs"])
        .arg(&vcs)
        .args(["--compiler-mode", "strict"])
        .args(["--std-core-link-mode", "precompiled"])
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let v: Value = serde_json::from_slice(&output).expect("json");
    assert_single_json_error(&v, "C105", "build");
    let err = v
        .get("errors")
        .and_then(|errs| errs.as_array())
        .and_then(|errs| errs.first())
        .and_then(|entry| entry.get("message"))
        .and_then(|msg| msg.as_str())
        .unwrap_or_default();
    assert!(err.contains("forbids intrinsic fallback"));
    assert!(err.contains("std::bytes::len"));
}

#[test]
fn strict_acceptance_source_of_truth_tamper_fails_with_c101() {
    let src = r#"
        function main() -> Int { 0 }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("strict_acceptance_source_tamper.clear");
    fs::write(&file, src).expect("write");
    write_signed_strict_dependency_fixture(tmp.path(), "[]");
    fs::write(
        tmp.path().join("clg-packages.json"),
        r#"{
  "schema_version": 1,
  "packages": []
}"#,
    )
    .expect("write disallowed legacy package metadata");
    let v = run_strict_build_json_failure(tmp.path(), &file);
    assert_single_json_error(&v, "C101", "build");
}

#[test]
fn strict_acceptance_artifact_identity_tamper_fails_with_c102() {
    let src = r#"
        function main() -> Int { 0 }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("strict_acceptance_digest_tamper.clear");
    fs::write(&file, src).expect("write");
    write_signed_strict_dependency_fixture(tmp.path(), "[]");
    fs::write(
        tmp.path().join("clg.lock.json"),
        r#"{
  "schema_version": 0,
  "dependencies": [
    {
      "name": "std::core",
      "version": "1.0.0",
      "digest": "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
    }
  ]
}"#,
    )
    .expect("tamper lockfile digest");
    let v = run_strict_build_json_failure(tmp.path(), &file);
    assert_single_json_error(&v, "C102", "build");
}

#[test]
fn strict_acceptance_trust_tamper_fails_with_c103() {
    let src = r#"
        function main() -> Int { 0 }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("strict_acceptance_trust_tamper.clear");
    fs::write(&file, src).expect("write");
    write_signed_strict_dependency_fixture(tmp.path(), "[]");
    fs::write(
        tmp.path().join("clg.package-signatures.json"),
        r#"{
  "schema_version": 0,
  "signatures": [
    {
      "name": "std::core",
      "version": "1.0.0",
      "digest": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "key_id": "k1",
      "signed_at": "2026-06-01T00:00:00Z",
      "signature_format": "ed25519",
      "signature": "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"
    }
  ]
}"#,
    )
    .expect("tamper package signature");
    let v = run_strict_build_json_failure(tmp.path(), &file);
    assert_single_json_error(&v, "C103", "build");
}

#[test]
fn strict_acceptance_schema_tamper_fails_with_c104() {
    let src = r#"
        function main() -> Int { 0 }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("strict_acceptance_schema_tamper.clear");
    fs::write(&file, src).expect("write");
    write_signed_strict_dependency_fixture(tmp.path(), "[]");
    fs::write(
        tmp.path().join("clg.package-metadata.json"),
        r#"{"schema_version":2,"packages":[]}"#,
    )
    .expect("tamper package metadata schema");
    let v = run_strict_build_json_failure(tmp.path(), &file);
    assert_single_json_error(&v, "C104", "build");
}

#[test]
fn strict_acceptance_abi_link_tamper_fails_with_c105() {
    let src = r#"
        function main() -> Int { std::str::len("abc") }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("strict_acceptance_abi_tamper.clear");
    fs::write(&file, src).expect("write");
    write_signed_strict_dependency_fixture(
        tmp.path(),
        r#"[
        {
          "symbol": "std::str::len",
          "effect": "pure",
          "params": ["String"],
          "ret": "Int",
          "capability": null
        }
      ]"#,
    );
    fs::write(
        tmp.path().join("clg.package-abi.json"),
        r#"{
  "schema_version": 0,
  "contracts": []
}"#,
    )
    .expect("tamper strict ABI to metadata mismatch");
    let v = run_strict_build_json_failure(tmp.path(), &file);
    assert_single_json_error(&v, "C105", "build");
}

#[test]
fn strict_acceptance_runtime_capability_tamper_fails_with_c106() {
    let src = r#"
        function main() -> Int { std::str::len("abc") }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("strict_acceptance_host_tamper.clear");
    fs::write(&file, src).expect("write");
    write_signed_strict_dependency_fixture(
        tmp.path(),
        r#"[
        {
          "symbol": "std::str::len",
          "effect": "pure",
          "params": ["String"],
          "ret": "Int",
          "capability": "std::crypto::hash"
        }
      ]"#,
    );
    let v = run_strict_build_json_failure(tmp.path(), &file);
    assert_single_json_error(&v, "C106", "build");
    assert!(
        !tmp.path().join("out.wasm").exists(),
        "strict gate failure should stop before final wasm output emission"
    );
}

#[test]
fn strict_acceptance_env_time_disallowed_in_contract_static_profile() {
    let src = r#"
        function main() -> Int { std::str::len("abc") }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp
        .path()
        .join("strict_acceptance_env_time_contract_static.clear");
    fs::write(&file, src).expect("write");
    write_signed_strict_dependency_fixture(
        tmp.path(),
        r#"[{
          "symbol": "std::str::len",
          "effect": "pure",
          "params": ["String"],
          "ret": "Int",
          "capability": "std::env::time"
        }]"#,
    );
    write_strict_host_profile(tmp.path(), "contract_static", &["std::env::time"]);
    let v = run_strict_build_json_failure(tmp.path(), &file);
    assert_single_json_error(&v, "C106", "build");
}

#[test]
fn strict_acceptance_env_random_disallowed_in_shared_app_profile() {
    let src = r#"
        function main() -> Int { std::str::len("abc") }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp
        .path()
        .join("strict_acceptance_env_random_shared_app.clear");
    fs::write(&file, src).expect("write");
    write_signed_strict_dependency_fixture(
        tmp.path(),
        r#"[{
          "symbol": "std::str::len",
          "effect": "pure",
          "params": ["String"],
          "ret": "Int",
          "capability": "std::env::random"
        }]"#,
    );
    write_strict_host_profile(tmp.path(), "shared_app", &["std::env::random"]);
    let v = run_strict_build_json_failure(tmp.path(), &file);
    assert_single_json_error(&v, "C106", "build");
}

#[test]
fn strict_acceptance_env_chain_id_allowed_when_capability_present() {
    let src = r#"
        io function main() -> Int { std::str::len(std::env::chain_id()) }
    "#;

    let run_case = |profile: &str| {
        let tmp = tempdir().unwrap();
        let file = tmp
            .path()
            .join(format!("strict_acceptance_env_chain_id_{profile}.clear"));
        fs::write(&file, src).expect("write");
        write_signed_strict_dependency_fixture(
            tmp.path(),
            r#"[
            {
              "symbol": "std::env::chain_id",
              "effect": "io",
              "params": [],
              "ret": "String",
              "capability": "std::env::chain_id"
            },
            {
              "symbol": "std::str::len",
              "effect": "pure",
              "params": ["String"],
              "ret": "Int",
              "capability": null
            }
          ]"#,
        );
        write_strict_host_profile(tmp.path(), profile, &["std::env::chain_id"]);
        run_strict_build_success(tmp.path(), &file);
    };

    run_case("contract_static");
    run_case("shared_app");
}

#[test]
fn strict_acceptance_forced_determinism_replay_mismatch_fails_with_c107() {
    let src = r#"
        function main() -> Int { std::str::len("abc") }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp
        .path()
        .join("strict_acceptance_forced_replay_mismatch.clear");
    fs::write(&file, src).expect("write");
    write_signed_strict_dependency_fixture(
        tmp.path(),
        r#"[
        {
          "symbol": "std::str::len",
          "effect": "pure",
          "params": ["String"],
          "ret": "Int",
          "capability": null
        }
      ]"#,
    );

    let out = tmp.path().join("out.wasm");
    let vcs = tmp.path().join("out.vc.json");
    let mut cmd = Command::cargo_bin("clg").unwrap();
    cmd.env("CLG_TEST_FORCE_STRICT_DETERMINISM_MISMATCH", "1");
    cmd.args(["--json-errors", "build"])
        .arg(&file)
        .args(["-o"])
        .arg(&out)
        .args(["--emit-vcs"])
        .arg(&vcs)
        .args(["--compiler-mode", "strict"])
        .args(["--std-core-link-mode", "precompiled"]);
    let output = cmd.assert().failure().get_output().stdout.clone();
    let v: Value = serde_json::from_slice(&output).expect("json");
    assert_single_json_error(&v, "C107", "build");

    let artifact = strict_import_map_artifact_path(&out);
    assert!(
        !artifact.exists(),
        "C107 determinism replay failure should not emit strict import-map artifact"
    );
}

#[test]
fn strict_acceptance_import_map_artifact_write_failure_fails_with_c108() {
    let src = r#"
        function main() -> Int { 0 }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp
        .path()
        .join("strict_acceptance_import_map_write_fail.clear");
    fs::write(&file, src).expect("write");
    write_signed_strict_dependency_fixture(tmp.path(), "[]");

    let blocked_parent = tmp.path().join("blocked_parent");
    fs::write(&blocked_parent, "not a directory").expect("write blocking file");
    let out = blocked_parent.join("out.wasm");
    let vcs = tmp.path().join("out.vc.json");
    let mut cmd = Command::cargo_bin("clg").unwrap();
    cmd.args(["--json-errors", "build"])
        .arg(&file)
        .args(["-o"])
        .arg(&out)
        .args(["--emit-vcs"])
        .arg(&vcs)
        .args(["--compiler-mode", "strict"]);
    let output = cmd.assert().failure().get_output().stdout.clone();
    let v: Value = serde_json::from_slice(&output).expect("json");
    assert_single_json_error(&v, "C108", "build");
}

#[test]
fn strict_acceptance_gate_reports_complete_violation_list() {
    let src = r#"
        function main() -> Int {
            std::str::len("abc") + std::bytes::len(std::bytes::from_string("xy"))
        }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp
        .path()
        .join("strict_acceptance_complete_violations.clear");
    fs::write(&file, src).expect("write");
    write_signed_strict_dependency_fixture(
        tmp.path(),
        r#"[
        {
          "symbol": "std::str::len",
          "effect": "pure",
          "params": ["String"],
          "ret": "Int",
          "capability": "std::crypto::hash"
        },
        {
          "symbol": "std::bytes::len",
          "effect": "pure",
          "params": ["Bytes"],
          "ret": "Int",
          "capability": "std::crypto::hash"
        }
      ]"#,
    );
    let v = run_strict_build_json_failure(tmp.path(), &file);
    let errs = assert_json_error_codes(&v, "build", &["C106", "C106"]);
    assert!(errs[0]
        .get("message")
        .and_then(|s| s.as_str())
        .unwrap_or_default()
        .contains("std::bytes::len"));
    assert!(errs[1]
        .get("message")
        .and_then(|s| s.as_str())
        .unwrap_or_default()
        .contains("std::str::len"));
    assert!(
        !tmp.path().join("out.wasm").exists(),
        "strict gate failure should stop before final wasm output emission"
    );
}

#[test]
fn strict_acceptance_import_map_artifact_matches_failure_diagnostics() {
    let src = r#"
        function main() -> Int {
            std::str::len("abc") + std::bytes::len(std::bytes::from_string("xy"))
        }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp
        .path()
        .join("strict_acceptance_import_map_failure_diagnostics.clear");
    fs::write(&file, src).expect("write");
    write_signed_strict_dependency_fixture(
        tmp.path(),
        r#"[
        {
          "symbol": "std::str::len",
          "effect": "pure",
          "params": ["String"],
          "ret": "Int",
          "capability": "std::crypto::hash"
        },
        {
          "symbol": "std::bytes::len",
          "effect": "pure",
          "params": ["Bytes"],
          "ret": "Int",
          "capability": "std::crypto::hash"
        }
      ]"#,
    );

    let output = run_strict_build_json_failure(tmp.path(), &file);
    let expected_errors = output
        .get("errors")
        .and_then(|errors| errors.as_array())
        .expect("errors array");

    let artifact_path = strict_import_map_artifact_path(&tmp.path().join("out.wasm"));
    let artifact_bytes = fs::read(&artifact_path).expect("read strict import-map artifact");
    let artifact_json: Value = serde_json::from_slice(&artifact_bytes).expect("artifact json");
    let artifact_diagnostics = artifact_json
        .get("diagnostics")
        .and_then(|diagnostics| diagnostics.as_array())
        .expect("artifact diagnostics");

    assert_eq!(artifact_diagnostics.len(), expected_errors.len());
    for (artifact_diag, expected_error) in artifact_diagnostics.iter().zip(expected_errors.iter()) {
        assert_eq!(artifact_diag.get("code"), expected_error.get("code"));
        assert_eq!(artifact_diag.get("message"), expected_error.get("message"));
    }
}

