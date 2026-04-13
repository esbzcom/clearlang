#[test]
fn build_failure_for_non_int_or_missing_main() {
    let tmp = tempdir().unwrap();
    let out = tmp.path().join("bad.wasm");
    let mut cmd = Command::cargo_bin("clg").unwrap();
    // 07_bools has no main; codegen should fail
    cmd.args(["--json-errors", "build"])
        .arg(repo_sample("07_bools.clear"))
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
    // Missing main should map to C002
    assert_eq!(e0.get("code").and_then(|s| s.as_str()), Some("C002"));
    assert_eq!(e0.get("stage").and_then(|s| s.as_str()), Some("build"));
}

#[test]
fn build_contract_exports_apply_and_query() {
    let src = r#"
        pure function apply(state: Bytes, msg: Bytes) -> Bytes { msg }
        pure function query(state: Bytes, msg: Bytes) -> Bytes { state }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("contract.clear");
    fs::write(&file, src).expect("write");
    let out = tmp.path().join("contract.wasm");

    Command::cargo_bin("clg")
        .unwrap()
        .args(["build", "--contract"])
        .arg(&file)
        .args(["-o"])
        .arg(&out)
        .assert()
        .success();

    let bytes = fs::read(&out).expect("read wasm");
    let mut exports = Vec::new();
    for payload in wasmparser::Parser::new(0).parse_all(&bytes) {
        if let wasmparser::Payload::ExportSection(reader) = payload.expect("payload") {
            for export in reader {
                let export = export.expect("export");
                exports.push(export.name.to_string());
            }
        }
    }
    assert!(exports.contains(&"init".to_string()));
    assert!(exports.contains(&"handle".to_string()));
    assert!(exports.contains(&"query".to_string()));
}

#[test]
fn build_contract_requires_query_function() {
    let src = r#"
        pure function apply(state: Bytes, msg: Bytes) -> Bytes { msg }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("contract_missing_query.clear");
    fs::write(&file, src).expect("write");
    let out = tmp.path().join("contract_missing_query.wasm");

    let output = Command::cargo_bin("clg")
        .unwrap()
        .args(["--json-errors", "build", "--contract"])
        .arg(&file)
        .args(["-o"])
        .arg(&out)
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let v: Value = serde_json::from_slice(&output).expect("json");
    let errs = v
        .get("errors")
        .and_then(|e| e.as_array())
        .expect("errors array");
    assert_eq!(errs.len(), 1);
    let e0 = &errs[0];
    assert_eq!(e0.get("code").and_then(|s| s.as_str()), Some("C011"));
    assert_eq!(e0.get("stage").and_then(|s| s.as_str()), Some("build"));
}

#[test]
fn release_help_exposes_gate_c_primary_shape() {
    let output = Command::cargo_bin("clg")
        .unwrap()
        .args(["release", "--help"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let help = String::from_utf8(output).expect("utf8");
    assert!(help.contains("release [OPTIONS]"));
    assert!(help.contains("--key"));
    assert!(help.contains("--pubkey"));
    assert!(help.contains("--root"));
    assert!(
        !help.contains("--advisory-as-of"),
        "release must not expose retired advisory override flag"
    );
    assert!(
        !help.contains("--key-id"),
        "release must not expose retired key-id override flag"
    );
    assert!(
        !help.contains("--out-dir"),
        "release must not expose retired out-dir override flag"
    );
    assert!(
        !help.contains("--trust-policy"),
        "release must not expose retired trust-policy override flag"
    );
    assert!(
        !help.contains("--compiler-mode"),
        "release command shape should hide build internals from primary help"
    );
    assert!(
        !help.contains("--release-profile"),
        "release command should not expose release profile downgrades"
    );
    assert!(
        !help.contains("--proof-strict"),
        "release command should not expose proof strictness downgrade controls"
    );
    assert!(
        !help.contains("--require-assurance"),
        "release command should not expose assurance downgrade controls"
    );
}

#[test]
fn release_requires_minimal_required_flags() {
    let output = Command::cargo_bin("clg")
        .unwrap()
        .args(["release"])
        .assert()
        .failure()
        .get_output()
        .stderr
        .clone();
    let text = String::from_utf8(output).expect("utf8");
    assert!(text.contains("--key"));
    assert!(text.contains("--pubkey"));
}

#[test]
fn strict_help_exposes_init_subcommand() {
    let output = Command::cargo_bin("clg")
        .unwrap()
        .args(["strict", "--help"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let help = String::from_utf8(output).expect("utf8");
    assert!(help.contains("strict"));
    assert!(help.contains("init"));
}

#[test]
fn strict_init_creates_required_preflight_files() {
    let tmp = tempdir().expect("tempdir");
    let root = tmp.path().join("project");
    Command::cargo_bin("clg")
        .unwrap()
        .args(["strict", "init"])
        .arg(&root)
        .assert()
        .success();

    for file in [
        "clg.project.json",
        "clg.lock.json",
        "clg.trust-policy.json",
        "clg.host-profile.json",
        "clg.package-metadata.json",
        "clg.package-abi.json",
        "trust-policy.json",
    ] {
        assert!(
            root.join(file).exists(),
            "strict init should create `{}`",
            file
        );
    }
}

#[test]
fn strict_init_writes_project_manifest_schema_v1_with_project_metadata_fields() {
    let tmp = tempdir().expect("tempdir");
    let root = tmp.path().join("project");
    Command::cargo_bin("clg")
        .unwrap()
        .args(["strict", "init"])
        .arg(&root)
        .assert()
        .success();

    let manifest_bytes = fs::read(root.join("clg.project.json")).expect("read project manifest");
    let manifest: Value = serde_json::from_slice(&manifest_bytes).expect("parse project manifest");
    assert_eq!(
        manifest.get("schema_version").and_then(|v| v.as_u64()),
        Some(1)
    );
    assert!(
        manifest.get("project").is_some(),
        "project metadata missing"
    );
    assert_eq!(
        manifest.get("project").and_then(|p| p.get("entry")).and_then(|v| v.as_str()),
        Some("main.clear"),
        "project.entry must be present in strict-init manifest template"
    );
    assert!(
        manifest.get("dependencies").is_some(),
        "dependencies array missing"
    );
    assert!(
        manifest.get("release_defaults").is_some(),
        "release_defaults missing"
    );
}

#[test]
fn strict_init_fails_when_existing_preflight_file_is_invalid() {
    let tmp = tempdir().expect("tempdir");
    let root = tmp.path().join("project");
    fs::create_dir_all(&root).expect("create root");
    fs::write(root.join("clg.lock.json"), r#"{"schema_version":"bad"}"#).expect("write bad lock");

    let output = Command::cargo_bin("clg")
        .unwrap()
        .args(["--json-errors", "strict", "init"])
        .arg(&root)
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let v: Value = serde_json::from_slice(&output).expect("json");
    let errs = v
        .get("errors")
        .and_then(|e| e.as_array())
        .expect("errors array");
    assert_eq!(errs.len(), 1);
    let e0 = &errs[0];
    assert_eq!(e0.get("code").and_then(|s| s.as_str()), Some("C104"));
    assert_eq!(e0.get("stage").and_then(|s| s.as_str()), Some("strict"));
}

#[test]
fn build_release_like_usage_emits_migration_guidance_to_release() {
    let tmp = tempdir().expect("tempdir");
    let file = tmp.path().join("main.clear");
    let out = tmp.path().join("out.wasm");
    fs::write(&file, "function main() -> Int { 0 }").expect("write source");

    let output = Command::cargo_bin("clg")
        .unwrap()
        .args(["build"])
        .arg(&file)
        .args(["-o"])
        .arg(&out)
        .args(["--release-profile", "production"])
        .assert()
        .failure()
        .get_output()
        .stderr
        .clone();
    let text = String::from_utf8(output).expect("utf8 stderr");
    assert!(
        text.contains("migration: prefer `clg release"),
        "expected migration guidance, got: {text}"
    );
}

#[test]
fn verify_release_gate_usage_emits_migration_guidance_to_release() {
    let tmp = tempdir().expect("tempdir");
    let module = tmp.path().join("missing.wasm");
    let sig = tmp.path().join("missing.sig.json");
    let pubkey = tmp.path().join("missing.public.json");

    let output = Command::cargo_bin("clg")
        .unwrap()
        .args(["verify"])
        .args(["--module"])
        .arg(&module)
        .args(["--sig"])
        .arg(&sig)
        .args(["--pubkey"])
        .arg(&pubkey)
        .args(["--require-assurance", "proved_all"])
        .assert()
        .failure()
        .get_output()
        .stderr
        .clone();
    let text = String::from_utf8(output).expect("utf8 stderr");
    assert!(
        text.contains("migration: prefer `clg release"),
        "expected migration guidance, got: {text}"
    );
}

#[test]
fn build_help_marks_command_as_advanced() {
    let output = Command::cargo_bin("clg")
        .unwrap()
        .args(["build", "--help"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let help = String::from_utf8(output).expect("utf8");
    assert!(help.contains("Advanced expert/debug compile command"));
}
