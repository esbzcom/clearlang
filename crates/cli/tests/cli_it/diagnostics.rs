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
fn strict_compiler_mode_rejects_proof_strict_false_override_with_c030() {
    let src = r#"
        function main() -> Int { 0 }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("strict_mode_override_false.clear");
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
        .args(["--compiler-mode", "strict", "--proof-strict=false"]);
    let output = cmd.assert().failure().get_output().stdout.clone();
    let v: Value = serde_json::from_slice(&output).expect("json");
    assert_single_json_error(&v, "C030", "build");
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
