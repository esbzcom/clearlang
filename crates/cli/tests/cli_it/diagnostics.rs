use super::*;

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

fn write_minimal_strict_preflight_files(root: &Path) {
    write_minimal_strict_lockfile(root);
    write_minimal_strict_trust_policy(root);
    write_minimal_strict_host_profile(root);
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
fn strict_compiler_mode_requires_trust_policy_with_c103() {
    let src = r#"
        function main() -> Int { 0 }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("strict_mode_no_trust_policy.clear");
    fs::write(&file, src).expect("write");
    write_minimal_strict_lockfile(tmp.path());
    write_minimal_strict_host_profile(tmp.path());
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
fn strict_compiler_mode_rejects_invalid_lockfile_digest_with_c102() {
    let src = r#"
        function main() -> Int { 0 }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("strict_mode_lock_digest.clear");
    fs::write(&file, src).expect("write");
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
