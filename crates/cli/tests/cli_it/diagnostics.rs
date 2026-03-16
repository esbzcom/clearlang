use super::*;
use ed25519_dalek::{Signer, SigningKey};
use sha2::{Digest, Sha256};

fn assert_single_json_error(v: &Value, code: &str, stage: &str) {
    assert_eq!(v.get("ok").and_then(|b| b.as_bool()), Some(false));
    let errs = v
        .get("errors")
        .and_then(|e| e.as_array())
        .expect("errors array");
    assert_eq!(errs.len(), 1, "expected exactly one error, got {errs:?}");
    let e0 = &errs[0];
    assert_eq!(e0.get("code").and_then(|s| s.as_str()), Some(code));
    assert_eq!(e0.get("stage").and_then(|s| s.as_str()), Some(stage));
}

fn assert_json_error_codes(v: &Value, stage: &str, expected_codes: &[&str]) -> Vec<Value> {
    assert_eq!(v.get("ok").and_then(|b| b.as_bool()), Some(false));
    let errs = v
        .get("errors")
        .and_then(|e| e.as_array())
        .expect("errors array");
    let actual_codes = errs
        .iter()
        .map(|err| err.get("code").and_then(|s| s.as_str()).unwrap_or_default())
        .collect::<Vec<_>>();
    assert_eq!(actual_codes, expected_codes);
    for err in errs {
        assert_eq!(err.get("stage").and_then(|s| s.as_str()), Some(stage));
    }
    errs.to_vec()
}

fn write_minimal_strict_lockfile(root: &Path) {
    fs::write(
        root.join("clg.lock.json"),
        r#"{"schema_version":0,"dependencies":[]}"#,
    )
    .expect("write strict lockfile");
}

fn write_minimal_strict_trust_policy(root: &Path) {
    fs::write(
        root.join("clg.trust-policy.json"),
        r#"{
  "schema_version": 0,
  "trusted_signers": [
    {
      "key_id": "std-core-release-ed25519-2026q1",
      "scheme": "ed25519",
      "public_key": "hex:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "not_before": "2026-01-01T00:00:00Z",
      "not_after": "2027-01-01T00:00:00Z"
    }
  ],
  "revoked_key_ids": []
}"#,
    )
    .expect("write strict trust policy");
}

fn write_minimal_strict_host_profile(root: &Path) {
    fs::write(
        root.join("clg.host-profile.json"),
        r#"{
  "schema_version": 0,
  "profile": "contract_static",
  "capabilities": []
}"#,
    )
    .expect("write strict host profile");
}

fn write_minimal_strict_package_metadata(root: &Path) {
    fs::write(
        root.join("clg.package-metadata.json"),
        r#"{
  "schema_version": 0,
  "packages": []
}"#,
    )
    .expect("write strict package metadata");
}

fn write_minimal_strict_package_abi(root: &Path) {
    fs::write(
        root.join("clg.package-abi.json"),
        r#"{
  "schema_version": 0,
  "contracts": []
}"#,
    )
    .expect("write strict package ABI");
}

fn write_minimal_strict_preflight_files(root: &Path) {
    write_minimal_strict_lockfile(root);
    write_minimal_strict_trust_policy(root);
    write_minimal_strict_host_profile(root);
    write_minimal_strict_package_metadata(root);
    write_minimal_strict_package_abi(root);
}

fn write_signed_strict_dependency_fixture(root: &Path, imports_json: &str) {
    let signing = SigningKey::from_bytes(&[7u8; 32]);
    let public_key_hex = hex::encode(signing.verifying_key().to_bytes());

    fs::write(
        root.join("clg.lock.json"),
        r#"{
  "schema_version": 0,
  "dependencies": [
    {
      "name": "std::core",
      "version": "1.0.0",
      "digest": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
    }
  ]
}"#,
    )
    .expect("write strict lockfile");
    fs::write(
        root.join("clg.package-metadata.json"),
        r#"{
  "schema_version": 0,
  "packages": [
    {
      "name": "std::core",
      "version": "1.0.0",
      "digest": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "artifact": { "format": "wasm", "path": "store/std-core-1.0.0.wasm" },
      "abi_id": "abi:std::core:1.0.0"
    }
  ]
}"#,
    )
    .expect("write strict package metadata");
    fs::write(
        root.join("clg.package-abi.json"),
        format!(
            r#"{{
  "schema_version": 0,
  "contracts": [
    {{
      "abi_id": "abi:std::core:1.0.0",
      "package": "std::core",
      "version": "1.0.0",
      "imports": {imports_json}
    }}
  ]
}}"#
        ),
    )
    .expect("write strict package abi");
    fs::write(
        root.join("clg.trust-policy.json"),
        format!(
            r#"{{
  "schema_version": 0,
  "trusted_signers": [
    {{
      "key_id": "k1",
      "scheme": "ed25519",
      "public_key": "hex:{public_key_hex}",
      "not_before": "2026-01-01T00:00:00Z",
      "not_after": "2027-01-01T00:00:00Z"
    }}
  ],
  "revoked_key_ids": []
}}"#
        ),
    )
    .expect("write strict trust policy");
    fs::write(
        root.join("clg.host-profile.json"),
        r#"{
  "schema_version": 0,
  "profile": "contract_static",
  "capabilities": []
}"#,
    )
    .expect("write strict host profile");

    let signed_at = "2026-06-01T00:00:00Z";
    let payload = format!(
        "clg-package-signature-v0\n{}\n{}\n{}\n{}\n",
        "std::core",
        "1.0.0",
        "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        signed_at
    );
    let signature = signing.sign(payload.as_bytes());
    fs::write(
        root.join("clg.package-signatures.json"),
        format!(
            r#"{{
  "schema_version": 0,
  "signatures": [
    {{
      "name": "std::core",
      "version": "1.0.0",
      "digest": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "key_id": "k1",
      "signed_at": "{signed_at}",
      "signature_format": "ed25519",
      "signature": "{}"
    }}
  ]
}}"#,
            hex::encode(signature.to_bytes())
        ),
    )
    .expect("write strict package signatures");
}

fn run_strict_build_json_failure(root: &Path, file: &Path) -> Value {
    let out = root.join("out.wasm");
    let vcs = root.join("out.vc.json");
    let mut cmd = Command::cargo_bin("clg").unwrap();
    cmd.args(["--json-errors", "build"])
        .arg(file)
        .args(["-o"])
        .arg(&out)
        .args(["--emit-vcs"])
        .arg(&vcs)
        .args(["--compiler-mode", "strict"]);
    let output = cmd.assert().failure().get_output().stdout.clone();
    serde_json::from_slice(&output).expect("json")
}

fn run_strict_build_success(root: &Path, file: &Path) {
    let out = root.join("out.wasm");
    let vcs = root.join("out.vc.json");
    let mut cmd = Command::cargo_bin("clg").unwrap();
    cmd.args(["build"])
        .arg(file)
        .args(["-o"])
        .arg(&out)
        .args(["--emit-vcs"])
        .arg(&vcs)
        .args(["--compiler-mode", "strict"]);
    cmd.assert().success();
}

fn strict_import_map_artifact_path(out: &Path) -> PathBuf {
    let file_name = out
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("out.wasm");
    let artifact_name = if let Some(stem) = file_name.strip_suffix(".wasm") {
        format!("{stem}.strict-import-map.json")
    } else {
        format!("{file_name}.strict-import-map.json")
    };
    match out.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent.join(artifact_name),
        _ => PathBuf::from(artifact_name),
    }
}

#[test]
fn type_error_reports_json_with_span_and_code() {
    // Create a small source with a type mismatch: add(1, true)
    let src = r#"
        pure function add(x: Int, y: Int) -> Int { x + y }
        function main() -> Int { add(1, true) }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("bad.clear");
    fs::write(&file, src).expect("write");
    let out = tmp.path().join("out.wasm");
    let mut cmd = Command::cargo_bin("clg").unwrap();
    cmd.args(["--json-errors", "build"])
        .arg(&file)
        .args(["-o"])
        .arg(&out);
    let output = cmd.assert().failure().get_output().stdout.clone();
    let v: Value = serde_json::from_slice(&output).expect("json");
    assert_eq!(v.get("ok").and_then(|b| b.as_bool()), Some(false));
    let errs = v
        .get("errors")
        .and_then(|e| e.as_array())
        .expect("errors array");
    assert_eq!(errs.len(), 1);
    let e0 = &errs[0];
    // Arg type mismatch should map to T003
    assert_eq!(e0.get("code").and_then(|s| s.as_str()), Some("T003"));
    assert_eq!(e0.get("stage").and_then(|s| s.as_str()), Some("type"));
    // Span presence
    assert!(e0.get("start").and_then(|n| n.as_u64()).is_some());
    assert!(e0.get("end").and_then(|n| n.as_u64()).is_some());
}

#[test]
fn unsigned_literal_out_of_range_reports_t112_in_json() {
    let src = r#"
        function main() -> U8 { U8(300) }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("bad_u8.clear");
    fs::write(&file, src).expect("write");
    let out = tmp.path().join("out.wasm");
    let mut cmd = Command::cargo_bin("clg").unwrap();
    cmd.args(["--json-errors", "build"])
        .arg(&file)
        .args(["-o"])
        .arg(&out);
    let output = cmd.assert().failure().get_output().stdout.clone();
    let v: Value = serde_json::from_slice(&output).expect("json");
    let errs = v
        .get("errors")
        .and_then(|e| e.as_array())
        .expect("errors array");
    let e0 = &errs[0];
    assert_eq!(e0.get("code").and_then(|s| s.as_str()), Some("T112"));
    assert_eq!(e0.get("stage").and_then(|s| s.as_str()), Some("type"));
}

#[test]
fn unsigned_constant_overflow_reports_t113_in_json() {
    let src = r#"
        function main() -> U64 { U64(9223372036854775807) * U64(3) }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("bad_u64_overflow.clear");
    fs::write(&file, src).expect("write");
    let out = tmp.path().join("out.wasm");
    let mut cmd = Command::cargo_bin("clg").unwrap();
    cmd.args(["--json-errors", "build"])
        .arg(&file)
        .args(["-o"])
        .arg(&out);
    let output = cmd.assert().failure().get_output().stdout.clone();
    let v: Value = serde_json::from_slice(&output).expect("json");
    let errs = v
        .get("errors")
        .and_then(|e| e.as_array())
        .expect("errors array");
    let e0 = &errs[0];
    assert_eq!(e0.get("code").and_then(|s| s.as_str()), Some("T113"));
    assert_eq!(e0.get("stage").and_then(|s| s.as_str()), Some("type"));
}

#[test]
fn array_index_out_of_bounds_reports_t114_in_json() {
    let src = r#"
        function main() -> Int { [1, 2][3] }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("bad_array_index.clear");
    fs::write(&file, src).expect("write");
    let out = tmp.path().join("out.wasm");
    let mut cmd = Command::cargo_bin("clg").unwrap();
    cmd.args(["--json-errors", "build"])
        .arg(&file)
        .args(["-o"])
        .arg(&out);
    let output = cmd.assert().failure().get_output().stdout.clone();
    let v: Value = serde_json::from_slice(&output).expect("json");
    let errs = v
        .get("errors")
        .and_then(|e| e.as_array())
        .expect("errors array");
    let e0 = &errs[0];
    assert_eq!(e0.get("code").and_then(|s| s.as_str()), Some("T114"));
    assert_eq!(e0.get("stage").and_then(|s| s.as_str()), Some("type"));
}

#[test]
fn tuple_index_requires_constant_reports_t115_in_json() {
    let src = r#"
        function main() -> Int { let i = 1; (1, 2, 3)[i] }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("bad_tuple_index.clear");
    fs::write(&file, src).expect("write");
    let out = tmp.path().join("out.wasm");
    let mut cmd = Command::cargo_bin("clg").unwrap();
    cmd.args(["--json-errors", "build"])
        .arg(&file)
        .args(["-o"])
        .arg(&out);
    let output = cmd.assert().failure().get_output().stdout.clone();
    let v: Value = serde_json::from_slice(&output).expect("json");
    let errs = v
        .get("errors")
        .and_then(|e| e.as_array())
        .expect("errors array");
    let e0 = &errs[0];
    assert_eq!(e0.get("code").and_then(|s| s.as_str()), Some("T115"));
    assert_eq!(e0.get("stage").and_then(|s| s.as_str()), Some("type"));
}

#[test]
fn collections_error_reports_collection_kind_error_in_json() {
    // Calling a collection API with wrong kind should yield T207 via --json-errors
    let src = r#"
        function main() -> Int { std::map::len(0) }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("bad_collections.clear");
    std::fs::write(&file, src).expect("write");
    let out = tmp.path().join("out.wasm");
    let mut cmd = Command::cargo_bin("clg").unwrap();
    cmd.args(["--json-errors", "build"])
        .arg(&file)
        .args(["-o"])
        .arg(&out);
    let output = cmd.assert().failure().get_output().stdout.clone();
    let v: Value = serde_json::from_slice(&output).expect("json");
    assert_eq!(v.get("ok").and_then(|b| b.as_bool()), Some(false));
    let errs = v
        .get("errors")
        .and_then(|e| e.as_array())
        .expect("errors array");
    assert!(!errs.is_empty());
    let e0 = &errs[0];
    assert_eq!(e0.get("code").and_then(|s| s.as_str()), Some("T207"));
    assert_eq!(e0.get("stage").and_then(|s| s.as_str()), Some("type"));
}

#[test]
fn if_branch_mismatch_reports_t301_in_json() {
    let src = r#"
        function main() -> Int { if true { 1 } else { false } }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("bad_if.clear");
    std::fs::write(&file, src).expect("write");
    let out = tmp.path().join("out.wasm");
    let mut cmd = Command::cargo_bin("clg").unwrap();
    cmd.args(["--json-errors", "build"])
        .arg(&file)
        .args(["-o"])
        .arg(&out);
    let output = cmd.assert().failure().get_output().stdout.clone();
    let v: Value = serde_json::from_slice(&output).expect("json");
    assert_eq!(v.get("ok").and_then(|b| b.as_bool()), Some(false));
    let errs = v
        .get("errors")
        .and_then(|e| e.as_array())
        .expect("errors array");
    assert!(!errs.is_empty());
    let e0 = &errs[0];
    assert_eq!(e0.get("code").and_then(|s| s.as_str()), Some("T301"));
    assert_eq!(e0.get("stage").and_then(|s| s.as_str()), Some("type"));
}

#[test]
fn migration_deferred_ergonomics_type_restrictions_report_stable_codes() {
    let cases = [
        ("migration/02_interface_type_params.clear", "T246"),
        (
            "migration/03_implementation_method_type_params.clear",
            "T245",
        ),
        ("migration/05_set_resource.clear", "T806"),
        ("migration/06_array_resource.clear", "T806"),
    ];

    for (file, expected_code) in cases {
        let tmp = tempdir().unwrap();
        let out = tmp.path().join("out.wasm");
        let mut cmd = Command::cargo_bin("clg").unwrap();
        cmd.args(["--json-errors", "build"])
            .arg(repo_sample(file))
            .args(["-o"])
            .arg(&out);
        let output = cmd.assert().failure().get_output().stdout.clone();
        let v: Value = serde_json::from_slice(&output).expect("json");
        assert_single_json_error(&v, expected_code, "type");
    }
}

#[test]
fn migration_refinement_ergonomics_samples_now_pass() {
    Command::cargo_bin("clg")
        .unwrap()
        .args(["parse"])
        .arg(repo_sample("migration/01_inline_refinement_param.clear"))
        .assert()
        .success();

    let tmp = tempdir().unwrap();
    let out = tmp.path().join("out.wasm");
    Command::cargo_bin("clg")
        .unwrap()
        .args(["build"])
        .arg(repo_sample("migration/04_generic_refinement_alias.clear"))
        .args(["-o"])
        .arg(&out)
        .assert()
        .success();
}

#[test]
fn strict_compiler_mode_requires_emit_vcs_with_c029() {
    let src = r#"
        function main() -> Int { 0 }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("strict_mode_no_vc.clear");
    fs::write(&file, src).expect("write");
    let out = tmp.path().join("out.wasm");

    let mut cmd = Command::cargo_bin("clg").unwrap();
    cmd.args(["--json-errors", "build"])
        .arg(&file)
        .args(["-o"])
        .arg(&out)
        .args(["--compiler-mode", "strict"]);
    let output = cmd.assert().failure().get_output().stdout.clone();
    let v: Value = serde_json::from_slice(&output).expect("json");
    assert_single_json_error(&v, "C029", "build");
}

#[test]
fn strict_acceptance_positive_all_gates_pass_with_signed_fixture() {
    let src = r#"
        function main() -> Int { std::core::math::add(1, 2) }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("strict_acceptance_ok.clear");
    fs::write(&file, src).expect("write");
    write_signed_strict_dependency_fixture(
        tmp.path(),
        r#"[
        {
          "symbol": "std::core::math::add",
          "effect": "pure",
          "params": ["Int", "Int"],
          "ret": "Int",
          "capability": null
        }
      ]"#,
    );
    run_strict_build_success(tmp.path(), &file);
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
        r#"{"schema_version":1,"packages":[]}"#,
    )
    .expect("tamper package metadata schema");
    let v = run_strict_build_json_failure(tmp.path(), &file);
    assert_single_json_error(&v, "C104", "build");
}

#[test]
fn strict_acceptance_abi_link_tamper_fails_with_c105() {
    let src = r#"
        function main() -> Int { std::core::math::add(1, 2) }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("strict_acceptance_abi_tamper.clear");
    fs::write(&file, src).expect("write");
    write_signed_strict_dependency_fixture(
        tmp.path(),
        r#"[
        {
          "symbol": "std::core::math::add",
          "effect": "pure",
          "params": ["Int", "Int"],
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
        function main() -> Int { std::core::math::add(1, 2) }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("strict_acceptance_host_tamper.clear");
    fs::write(&file, src).expect("write");
    write_signed_strict_dependency_fixture(
        tmp.path(),
        r#"[
        {
          "symbol": "std::core::math::add",
          "effect": "pure",
          "params": ["Int", "Int"],
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
fn strict_acceptance_gate_reports_complete_violation_list() {
    let src = r#"
        function main() -> Int {
            std::core::math::add(
                std::core::math::sub(4, 1),
                2
            )
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
          "symbol": "std::core::math::add",
          "effect": "pure",
          "params": ["Int", "Int"],
          "ret": "Int",
          "capability": "std::crypto::hash"
        },
        {
          "symbol": "std::core::math::sub",
          "effect": "pure",
          "params": ["Int", "Int"],
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
        .contains("std::core::math::add"));
    assert!(errs[1]
        .get("message")
        .and_then(|s| s.as_str())
        .unwrap_or_default()
        .contains("std::core::math::sub"));
    assert!(
        !tmp.path().join("out.wasm").exists(),
        "strict gate failure should stop before final wasm output emission"
    );
}

#[test]
fn strict_acceptance_import_map_artifact_matches_failure_diagnostics() {
    let src = r#"
        function main() -> Int {
            std::core::math::add(
                std::core::math::sub(4, 1),
                2
            )
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
          "symbol": "std::core::math::add",
          "effect": "pure",
          "params": ["Int", "Int"],
          "ret": "Int",
          "capability": "std::crypto::hash"
        },
        {
          "symbol": "std::core::math::sub",
          "effect": "pure",
          "params": ["Int", "Int"],
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

#[test]
fn strict_acceptance_import_map_artifact_is_deterministic_across_identical_runs() {
    let src = r#"
        function main() -> Int {
            std::core::math::add(
                std::core::math::sub(4, 1),
                2
            )
        }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp
        .path()
        .join("strict_acceptance_import_map_determinism.clear");
    fs::write(&file, src).expect("write");
    write_signed_strict_dependency_fixture(
        tmp.path(),
        r#"[
        {
          "symbol": "std::core::math::add",
          "effect": "pure",
          "params": ["Int", "Int"],
          "ret": "Int",
          "capability": null
        },
        {
          "symbol": "std::core::math::sub",
          "effect": "pure",
          "params": ["Int", "Int"],
          "ret": "Int",
          "capability": null
        }
      ]"#,
    );

    let out1 = tmp.path().join("run1.wasm");
    let vcs1 = tmp.path().join("run1.vc.json");
    Command::cargo_bin("clg")
        .unwrap()
        .args(["build"])
        .arg(&file)
        .args(["-o"])
        .arg(&out1)
        .args(["--emit-vcs"])
        .arg(&vcs1)
        .args(["--compiler-mode", "strict"])
        .assert()
        .success();

    let artifact1 = strict_import_map_artifact_path(&out1);
    let bytes1 = fs::read(&artifact1).expect("read run1 strict import-map artifact");
    let hash1 = hex::encode(Sha256::digest(bytes1.as_slice()));

    let out2 = tmp.path().join("run2.wasm");
    let vcs2 = tmp.path().join("run2.vc.json");
    Command::cargo_bin("clg")
        .unwrap()
        .args(["build"])
        .arg(&file)
        .args(["-o"])
        .arg(&out2)
        .args(["--emit-vcs"])
        .arg(&vcs2)
        .args(["--compiler-mode", "strict"])
        .assert()
        .success();

    let artifact2 = strict_import_map_artifact_path(&out2);
    let bytes2 = fs::read(&artifact2).expect("read run2 strict import-map artifact");
    let hash2 = hex::encode(Sha256::digest(bytes2.as_slice()));

    assert_eq!(
        bytes1, bytes2,
        "canonical import-map artifact bytes must match"
    );
    assert_eq!(
        hash1, hash2,
        "canonical import-map artifact hash must match"
    );

    let artifact_json: Value = serde_json::from_slice(&bytes1).expect("artifact json");
    assert_eq!(
        artifact_json
            .get("kind")
            .and_then(|value| value.as_str())
            .unwrap_or_default(),
        "clg.strict_direct_dependency_import_map.v0"
    );
    let imports = artifact_json
        .get("imports")
        .and_then(|value| value.as_array())
        .expect("artifact imports");
    assert_eq!(imports.len(), 2);
    assert_eq!(
        imports[0]
            .get("symbol")
            .and_then(|value| value.as_str())
            .unwrap_or_default(),
        "std::core::math::add"
    );
    assert_eq!(
        imports[1]
            .get("symbol")
            .and_then(|value| value.as_str())
            .unwrap_or_default(),
        "std::core::math::sub"
    );
}

#[test]
fn strict_compiler_mode_requires_lockfile_with_c101() {
    let src = r#"
        function main() -> Int { 0 }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("strict_mode_no_lock.clear");
    fs::write(&file, src).expect("write");
    let out = tmp.path().join("out.wasm");
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
    assert_single_json_error(&v, "C101", "build");
}

#[test]
fn strict_compiler_mode_rejects_legacy_package_metadata_source_with_c101() {
    let src = r#"
        function main() -> Int { 0 }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("strict_mode_legacy_package_source.clear");
    fs::write(&file, src).expect("write");
    write_minimal_strict_preflight_files(tmp.path());
    fs::write(
        tmp.path().join("clg-packages.json"),
        r#"{
  "schema_version": 1,
  "packages": []
}"#,
    )
    .expect("write legacy package metadata source");
    let out = tmp.path().join("out.wasm");
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
    assert_single_json_error(&v, "C101", "build");
}

#[test]
fn strict_compiler_mode_preflight_errors_are_reported_before_typecheck() {
    let src = r#"
        function main() -> Int { unknown_fn(1) }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp
        .path()
        .join("strict_mode_preflight_before_typecheck.clear");
    fs::write(&file, src).expect("write");
    let out = tmp.path().join("out.wasm");
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
    assert_single_json_error(&v, "C101", "build");
}

#[test]
fn strict_compiler_mode_requires_trust_policy_with_c103() {
    let src = r#"
        function main() -> Int { 0 }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("strict_mode_no_trust_policy.clear");
    fs::write(&file, src).expect("write");
    write_minimal_strict_lockfile(tmp.path());
    write_minimal_strict_host_profile(tmp.path());
    write_minimal_strict_package_metadata(tmp.path());
    write_minimal_strict_package_abi(tmp.path());
    let out = tmp.path().join("out.wasm");
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
    assert_single_json_error(&v, "C103", "build");
}

#[test]
fn strict_compiler_mode_requires_host_profile_with_c106() {
    let src = r#"
        function main() -> Int { 0 }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("strict_mode_no_host_profile.clear");
    fs::write(&file, src).expect("write");
    write_minimal_strict_lockfile(tmp.path());
    write_minimal_strict_trust_policy(tmp.path());
    write_minimal_strict_package_metadata(tmp.path());
    write_minimal_strict_package_abi(tmp.path());
    let out = tmp.path().join("out.wasm");
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
    assert_single_json_error(&v, "C106", "build");
}

#[test]
fn strict_compiler_mode_rejects_invalid_trust_policy_schema_with_c103() {
    let src = r#"
        function main() -> Int { 0 }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("strict_mode_bad_trust_schema.clear");
    fs::write(&file, src).expect("write");
    write_minimal_strict_lockfile(tmp.path());
    fs::write(
        tmp.path().join("clg.trust-policy.json"),
        r#"{"schema_version":1,"trusted_signers":[],"revoked_key_ids":[]}"#,
    )
    .expect("write invalid strict trust policy");
    write_minimal_strict_host_profile(tmp.path());
    write_minimal_strict_package_metadata(tmp.path());
    write_minimal_strict_package_abi(tmp.path());
    let out = tmp.path().join("out.wasm");
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
    assert_single_json_error(&v, "C103", "build");
}

#[test]
fn strict_compiler_mode_rejects_invalid_host_profile_schema_with_c106() {
    let src = r#"
        function main() -> Int { 0 }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("strict_mode_bad_host_schema.clear");
    fs::write(&file, src).expect("write");
    write_minimal_strict_lockfile(tmp.path());
    write_minimal_strict_trust_policy(tmp.path());
    write_minimal_strict_package_metadata(tmp.path());
    write_minimal_strict_package_abi(tmp.path());
    fs::write(
        tmp.path().join("clg.host-profile.json"),
        r#"{"schema_version":1,"profile":"contract_static","capabilities":[]}"#,
    )
    .expect("write invalid strict host profile");
    let out = tmp.path().join("out.wasm");
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
    assert_single_json_error(&v, "C106", "build");
}

#[test]
fn strict_compiler_mode_rejects_invalid_lockfile_schema_with_c104() {
    let src = r#"
        function main() -> Int { 0 }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("strict_mode_lock_schema.clear");
    fs::write(&file, src).expect("write");
    write_minimal_strict_trust_policy(tmp.path());
    write_minimal_strict_host_profile(tmp.path());
    write_minimal_strict_package_metadata(tmp.path());
    write_minimal_strict_package_abi(tmp.path());
    fs::write(
        tmp.path().join("clg.lock.json"),
        r#"{"schema_version":1,"dependencies":[]}"#,
    )
    .expect("write lockfile");
    let out = tmp.path().join("out.wasm");
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
    assert_single_json_error(&v, "C104", "build");
}

#[test]
fn strict_compiler_mode_rejects_invalid_package_metadata_schema_with_c104() {
    let src = r#"
        function main() -> Int { 0 }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("strict_mode_pkg_metadata_schema.clear");
    fs::write(&file, src).expect("write");
    write_minimal_strict_lockfile(tmp.path());
    write_minimal_strict_trust_policy(tmp.path());
    write_minimal_strict_host_profile(tmp.path());
    write_minimal_strict_package_abi(tmp.path());
    fs::write(
        tmp.path().join("clg.package-metadata.json"),
        r#"{"schema_version":1,"packages":[]}"#,
    )
    .expect("write strict package metadata");
    let out = tmp.path().join("out.wasm");
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
    assert_single_json_error(&v, "C104", "build");
}

#[test]
fn strict_compiler_mode_rejects_invalid_package_abi_schema_with_c104() {
    let src = r#"
        function main() -> Int { 0 }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("strict_mode_pkg_abi_schema.clear");
    fs::write(&file, src).expect("write");
    write_minimal_strict_lockfile(tmp.path());
    write_minimal_strict_trust_policy(tmp.path());
    write_minimal_strict_host_profile(tmp.path());
    write_minimal_strict_package_metadata(tmp.path());
    fs::write(
        tmp.path().join("clg.package-abi.json"),
        r#"{"schema_version":1,"contracts":[]}"#,
    )
    .expect("write strict package ABI");
    let out = tmp.path().join("out.wasm");
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
    assert_single_json_error(&v, "C104", "build");
}

#[test]
fn strict_compiler_mode_rejects_invalid_lockfile_digest_with_c102() {
    let src = r#"
        function main() -> Int { 0 }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("strict_mode_lock_digest.clear");
    fs::write(&file, src).expect("write");
    write_minimal_strict_trust_policy(tmp.path());
    write_minimal_strict_host_profile(tmp.path());
    write_minimal_strict_package_metadata(tmp.path());
    write_minimal_strict_package_abi(tmp.path());
    fs::write(
        tmp.path().join("clg.lock.json"),
        r#"{
  "schema_version": 0,
  "dependencies": [
    { "name": "std::core", "version": "1.0.0", "digest": "sha256:ABCDEF" }
  ]
}"#,
    )
    .expect("write lockfile");
    let out = tmp.path().join("out.wasm");
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
    assert_single_json_error(&v, "C102", "build");
}

#[test]
fn strict_compiler_mode_rejects_lockfile_package_digest_mismatch_with_c102() {
    let src = r#"
        function main() -> Int { 0 }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp
        .path()
        .join("strict_mode_lockfile_package_digest_mismatch.clear");
    fs::write(&file, src).expect("write");
    write_minimal_strict_trust_policy(tmp.path());
    write_minimal_strict_host_profile(tmp.path());
    fs::write(
        tmp.path().join("clg.lock.json"),
        r#"{
  "schema_version": 0,
  "dependencies": [
    {
      "name": "std::core",
      "version": "1.0.0",
      "digest": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
    }
  ]
}"#,
    )
    .expect("write lockfile");
    fs::write(
        tmp.path().join("clg.package-metadata.json"),
        r#"{
  "schema_version": 0,
  "packages": [
    {
      "name": "std::core",
      "version": "1.0.0",
      "digest": "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
      "artifact": { "format": "wasm", "path": "store/std-core-1.0.0.wasm" },
      "abi_id": "abi:std::core:1.0.0"
    }
  ]
}"#,
    )
    .expect("write strict package metadata");
    fs::write(
        tmp.path().join("clg.package-abi.json"),
        r#"{
  "schema_version": 0,
  "contracts": [
    {
      "abi_id": "abi:std::core:1.0.0",
      "package": "std::core",
      "version": "1.0.0",
      "imports": []
    }
  ]
}"#,
    )
    .expect("write strict package abi");
    let out = tmp.path().join("out.wasm");
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
    assert_single_json_error(&v, "C102", "build");
}

#[test]
fn strict_compiler_mode_requires_package_signatures_with_c103() {
    let src = r#"
        function main() -> Int { 0 }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp
        .path()
        .join("strict_mode_missing_package_signatures.clear");
    fs::write(&file, src).expect("write");
    write_minimal_strict_trust_policy(tmp.path());
    write_minimal_strict_host_profile(tmp.path());
    fs::write(
        tmp.path().join("clg.lock.json"),
        r#"{
  "schema_version": 0,
  "dependencies": [
    {
      "name": "std::core",
      "version": "1.0.0",
      "digest": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
    }
  ]
}"#,
    )
    .expect("write lockfile");
    fs::write(
        tmp.path().join("clg.package-metadata.json"),
        r#"{
  "schema_version": 0,
  "packages": [
    {
      "name": "std::core",
      "version": "1.0.0",
      "digest": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "artifact": { "format": "wasm", "path": "store/std-core-1.0.0.wasm" },
      "abi_id": "abi:std::core:1.0.0"
    }
  ]
}"#,
    )
    .expect("write strict package metadata");
    fs::write(
        tmp.path().join("clg.package-abi.json"),
        r#"{
  "schema_version": 0,
  "contracts": [
    {
      "abi_id": "abi:std::core:1.0.0",
      "package": "std::core",
      "version": "1.0.0",
      "imports": []
    }
  ]
}"#,
    )
    .expect("write strict package abi");
    let out = tmp.path().join("out.wasm");
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
    assert_single_json_error(&v, "C103", "build");
}

#[test]
fn strict_compiler_mode_requires_package_metadata_with_c104() {
    let src = r#"
        function main() -> Int { 0 }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("strict_mode_no_pkg_metadata.clear");
    fs::write(&file, src).expect("write");
    write_minimal_strict_lockfile(tmp.path());
    write_minimal_strict_trust_policy(tmp.path());
    write_minimal_strict_host_profile(tmp.path());
    write_minimal_strict_package_abi(tmp.path());
    let out = tmp.path().join("out.wasm");
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
    assert_single_json_error(&v, "C104", "build");
}

#[test]
fn strict_compiler_mode_allows_signed_dependency_with_empty_abi_imports() {
    let src = r#"
        function main() -> Int { 0 }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("strict_mode_signed_dep_ok.clear");
    fs::write(&file, src).expect("write");
    write_signed_strict_dependency_fixture(tmp.path(), "[]");
    let out = tmp.path().join("out.wasm");
    let vcs = tmp.path().join("out.vc.json");

    let mut cmd = Command::cargo_bin("clg").unwrap();
    cmd.args(["build"])
        .arg(&file)
        .args(["-o"])
        .arg(&out)
        .args(["--emit-vcs"])
        .arg(&vcs)
        .args(["--compiler-mode", "strict"]);
    cmd.assert().success();
}

#[test]
fn strict_compiler_mode_allows_linked_abi_symbol_from_strict_contract() {
    let src = r#"
        function main() -> Int { std::core::math::add(1, 2) }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("strict_mode_linked_abi_symbol.clear");
    fs::write(&file, src).expect("write");
    write_signed_strict_dependency_fixture(
        tmp.path(),
        r#"[
        {
          "symbol": "std::core::math::add",
          "effect": "pure",
          "params": ["Int", "Int"],
          "ret": "Int",
          "capability": null
        }
      ]"#,
    );
    let out = tmp.path().join("out.wasm");
    let vcs = tmp.path().join("out.vc.json");

    let mut cmd = Command::cargo_bin("clg").unwrap();
    cmd.args(["build"])
        .arg(&file)
        .args(["-o"])
        .arg(&out)
        .args(["--emit-vcs"])
        .arg(&vcs)
        .args(["--compiler-mode", "strict"]);
    cmd.assert().success();
}

#[test]
fn strict_compiler_mode_rejects_package_abi_mismatch_with_c105() {
    let src = r#"
        function main() -> Int { 0 }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("strict_mode_bad_pkg_abi_match.clear");
    fs::write(&file, src).expect("write");
    write_minimal_strict_lockfile(tmp.path());
    write_minimal_strict_trust_policy(tmp.path());
    write_minimal_strict_host_profile(tmp.path());
    fs::write(
        tmp.path().join("clg.package-metadata.json"),
        r#"{
  "schema_version": 0,
  "packages": [
    {
      "name": "std::core",
      "version": "1.0.0",
      "digest": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "artifact": { "format": "wasm", "path": "store/std-core-1.0.0.wasm" },
      "abi_id": "abi:std::core:1.0.0"
    }
  ]
}"#,
    )
    .expect("write strict package metadata");
    fs::write(
        tmp.path().join("clg.package-abi.json"),
        r#"{
  "schema_version": 0,
  "contracts": []
}"#,
    )
    .expect("write strict package abi");
    let out = tmp.path().join("out.wasm");
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
    assert_single_json_error(&v, "C105", "build");
}

#[test]
fn strict_compiler_mode_rejects_proof_strict_false_override_with_c030() {
    let src = r#"
        function main() -> Int { 0 }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("strict_mode_override_false.clear");
    fs::write(&file, src).expect("write");
    write_minimal_strict_preflight_files(tmp.path());
    let out = tmp.path().join("out.wasm");
    let vcs = tmp.path().join("out.vc.json");

    let mut cmd = Command::cargo_bin("clg").unwrap();
    cmd.args(["--json-errors", "build"])
        .arg(&file)
        .args(["-o"])
        .arg(&out)
        .args(["--emit-vcs"])
        .arg(&vcs)
        .args(["--compiler-mode", "strict", "--proof-strict=false"]);
    let output = cmd.assert().failure().get_output().stdout.clone();
    let v: Value = serde_json::from_slice(&output).expect("json");
    assert_single_json_error(&v, "C030", "build");
}

#[test]
fn strict_compiler_mode_rejects_assumed_surfaces_with_c033() {
    let src = r#"
        pure function check(a: Bytes, b: Bytes, x: U64, y: U64) -> Bool
            ensure { result == std::bytes::eq_ct(a, b) }
            ensure { (x & y) == x }
        {
            std::bytes::eq_ct(a, b)
        }
        function main() -> Int { 0 }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("strict_mode_assumed_surface.clear");
    fs::write(&file, src).expect("write");
    write_minimal_strict_preflight_files(tmp.path());
    let out = tmp.path().join("out.wasm");
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
    assert_single_json_error(&v, "C033", "build");
}

#[test]
fn strict_compiler_mode_allows_program_without_assumptions() {
    let src = r#"
        pure function id(x: Int) -> Int { x }
        function main() -> Int { id(1) }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("strict_mode_no_assumptions.clear");
    fs::write(&file, src).expect("write");
    write_minimal_strict_preflight_files(tmp.path());
    let out = tmp.path().join("out.wasm");
    let vcs = tmp.path().join("out.vc.json");

    Command::cargo_bin("clg")
        .unwrap()
        .args(["build"])
        .arg(&file)
        .args(["-o"])
        .arg(&out)
        .args(["--emit-vcs"])
        .arg(&vcs)
        .args(["--compiler-mode", "strict"])
        .assert()
        .success();
}

#[test]
fn trust_anchor_checker_versions_require_sign_with_c032() {
    let src = r#"
        function main() -> Int { 0 }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("trust_versions_require_sign.clear");
    fs::write(&file, src).expect("write");
    let out = tmp.path().join("out.wasm");

    let mut cmd = Command::cargo_bin("clg").unwrap();
    cmd.args(["--json-errors", "build"])
        .arg(&file)
        .args(["-o"])
        .arg(&out)
        .args([
            "--lean-checker-version",
            "4.14.0",
            "--coq-checker-version",
            "8.19.2",
        ]);
    let output = cmd.assert().failure().get_output().stdout.clone();
    let v: Value = serde_json::from_slice(&output).expect("json");
    assert_single_json_error(&v, "C032", "build");
}

#[test]
fn assurance_manifest_out_requires_sign_with_c034() {
    let src = r#"
        function main() -> Int { 0 }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("manifest_requires_sign.clear");
    fs::write(&file, src).expect("write");
    let out = tmp.path().join("out.wasm");
    let manifest = tmp.path().join("assurance.json");

    let mut cmd = Command::cargo_bin("clg").unwrap();
    cmd.args(["--json-errors", "build"])
        .arg(&file)
        .args(["-o"])
        .arg(&out)
        .args(["--assurance-manifest-out"])
        .arg(&manifest);
    let output = cmd.assert().failure().get_output().stdout.clone();
    let v: Value = serde_json::from_slice(&output).expect("json");
    assert_single_json_error(&v, "C034", "build");
}

#[test]
fn trust_anchor_checker_versions_must_be_paired_with_c032() {
    let src = r#"
        function main() -> Int { 0 }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("trust_versions_pair_required.clear");
    fs::write(&file, src).expect("write");
    let out = tmp.path().join("out.wasm");
    let vcs = tmp.path().join("out.vc.json");
    let sig = tmp.path().join("out.sig.json");
    let key = tmp.path().join("unused.json");

    let mut cmd = Command::cargo_bin("clg").unwrap();
    cmd.args(["--json-errors", "build"])
        .arg(&file)
        .args(["-o"])
        .arg(&out)
        .args(["--emit-vcs"])
        .arg(&vcs)
        .args(["--sign", "--key-id", "demo", "--scope", "both"])
        .args(["--key"])
        .arg(&key)
        .args(["--sig-out"])
        .arg(&sig)
        .args(["--lean-checker-version", "4.14.0"]);
    let output = cmd.assert().failure().get_output().stdout.clone();
    let v: Value = serde_json::from_slice(&output).expect("json");
    assert_single_json_error(&v, "C032", "build");
}

#[test]
fn trust_anchor_checker_versions_must_be_non_empty_with_c032() {
    let src = r#"
        function main() -> Int { 0 }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("trust_versions_non_empty_required.clear");
    fs::write(&file, src).expect("write");
    let out = tmp.path().join("out.wasm");
    let vcs = tmp.path().join("out.vc.json");
    let sig = tmp.path().join("out.sig.json");
    let key = tmp.path().join("unused.json");

    let mut cmd = Command::cargo_bin("clg").unwrap();
    cmd.args(["--json-errors", "build"])
        .arg(&file)
        .args(["-o"])
        .arg(&out)
        .args(["--emit-vcs"])
        .arg(&vcs)
        .args(["--sign", "--key-id", "demo", "--scope", "both"])
        .args(["--key"])
        .arg(&key)
        .args(["--sig-out"])
        .arg(&sig)
        .args([
            "--lean-checker-version",
            "",
            "--coq-checker-version",
            "8.19.2",
        ]);
    let output = cmd.assert().failure().get_output().stdout.clone();
    let v: Value = serde_json::from_slice(&output).expect("json");
    assert_single_json_error(&v, "C032", "build");
}
