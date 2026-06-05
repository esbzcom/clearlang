#[test]
fn run_wasm_with_runtime_link_artifacts_uses_local_store_loader_core() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path();
    let wasm_path = root.join("out.wasm");

    Command::cargo_bin("clg")
        .expect("bin")
        .args(["emit-hello", "-o"])
        .arg(&wasm_path)
        .assert()
        .success();

    let store_dir = root.join("store");
    fs::create_dir_all(&store_dir).expect("create store");
    let artifact_path = store_dir.join("hello.wasm");
    fs::copy(&wasm_path, &artifact_path).expect("copy artifact");
    let artifact_bytes = fs::read(&artifact_path).expect("read artifact");
    let digest = format!("sha256:{}", sha256_hex(artifact_bytes.as_slice()));

    write_runtime_loader_gate_artifacts(root, digest.as_str(), true);

    Command::cargo_bin("clg")
        .expect("bin")
        .args(["run"])
        .arg(&wasm_path)
        .assert()
        .success()
        .stdout(predicates::str::contains("42\n"));
}

#[test]
fn run_wasm_with_runtime_link_but_missing_store_index_fails_closed_with_r012() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path();
    let wasm_path = root.join("out.wasm");

    Command::cargo_bin("clg")
        .expect("bin")
        .args(["emit-hello", "-o"])
        .arg(&wasm_path)
        .assert()
        .success();

    let artifact_bytes = fs::read(&wasm_path).expect("read wasm");
    let digest = format!("sha256:{}", sha256_hex(artifact_bytes.as_slice()));
    write_runtime_loader_gate_artifacts(root, digest.as_str(), false);

    let output = Command::cargo_bin("clg")
        .expect("bin")
        .args(["--json-errors", "run"])
        .arg(&wasm_path)
        .output()
        .expect("run command");
    assert!(!output.status.success(), "run should fail");
    let v: Value = serde_json::from_slice(&output.stdout).expect("json");
    assert_eq!(v.get("ok").and_then(|b| b.as_bool()), Some(false));
    let errors = v
        .get("errors")
        .and_then(|e| e.as_array())
        .expect("errors array");
    assert!(!errors.is_empty());
    let first = &errors[0];
    assert_eq!(first.get("code").and_then(|s| s.as_str()), Some("R012"));
    assert_eq!(first.get("stage").and_then(|s| s.as_str()), Some("runtime"));
}

#[test]
fn runtime_loader_replay_missing_provider_symbol_json_is_deterministic() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path();
    let wasm_path = root.join("out.wasm");

    Command::cargo_bin("clg")
        .expect("bin")
        .args(["emit-hello", "-o"])
        .arg(&wasm_path)
        .assert()
        .success();

    let store_dir = root.join("store");
    fs::create_dir_all(&store_dir).expect("create store");
    let artifact_path = store_dir.join("hello.wasm");
    fs::copy(&wasm_path, &artifact_path).expect("copy artifact");
    let artifact_bytes = fs::read(&artifact_path).expect("read artifact");
    let digest = format!("sha256:{}", sha256_hex(artifact_bytes.as_slice()));
    write_runtime_loader_gate_artifacts(root, digest.as_str(), true);

    let runtime_link = serde_json::json!({
        "schema_version": 0,
        "resolver_version": 1,
        "packages": [
            {
                "id": "app::hello@1.0.0",
                "digest": digest,
                "artifact_path": "store/hello.wasm",
                "abi_id": "abi:app::hello:1.0.0"
            }
        ],
        "bindings": [
            {
                "import_module": "app::hello",
                "import_name": "main",
                "provider_package_id": "app::hello@1.0.0",
                "provider_symbol": "missing_symbol"
            }
        ]
    });
    fs::write(
        root.join("clg.runtime-link.json"),
        serde_json::to_vec_pretty(&runtime_link).expect("serialize runtime link"),
    )
    .expect("write runtime link");
    fs::write(
        root.join("clg.runtime-link.sha256"),
        format!(
            "{}\n",
            sha256_hex(canonical_json_bytes(&runtime_link).as_slice())
        ),
    )
    .expect("write runtime link hash");

    let output_a = Command::cargo_bin("clg")
        .expect("bin")
        .args(["--json-errors", "run"])
        .arg(&wasm_path)
        .output()
        .expect("run command");
    let output_b = Command::cargo_bin("clg")
        .expect("bin")
        .args(["--json-errors", "run"])
        .arg(&wasm_path)
        .output()
        .expect("run command");
    assert!(!output_a.status.success(), "run should fail");
    assert!(!output_b.status.success(), "run should fail");
    assert_eq!(
        output_a.stdout, output_b.stdout,
        "runtime loader JSON output should be replay-stable for missing provider symbol"
    );
    let v: Value = serde_json::from_slice(&output_a.stdout).expect("json");
    assert_eq!(v.get("ok").and_then(|b| b.as_bool()), Some(false));
    let errors = v
        .get("errors")
        .and_then(|e| e.as_array())
        .expect("errors array");
    assert!(!errors.is_empty());
    let first = &errors[0];
    assert_eq!(first.get("code").and_then(|s| s.as_str()), Some("R015"));
    assert_eq!(first.get("stage").and_then(|s| s.as_str()), Some("runtime"));
}

#[test]
fn runtime_loader_replay_missing_required_capability_json_is_deterministic() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path();
    let wasm_path = root.join("out.wasm");

    let wat = r#"
        (module
          (import "clearlang_env" "env_time" (func $env_time (result i64)))
          (memory (export "memory") 1)
          (global $__clg_heap_ptr (mut i32) (i32.const 0))
          (export "__clg_heap_ptr" (global $__clg_heap_ptr))
          (func (export "main") (result i32)
            call $env_time
            drop
            i32.const 0
          )
        )
    "#;
    fs::write(&wasm_path, wat_parse_str(wat).expect("wat parse")).expect("write wasm");
    write_empty_runtime_link(root);
    fs::write(
        root.join("clg.host-profile.json"),
        r#"{
  "schema_version": 0,
  "profile": "contract_static",
  "capabilities": ["std::wasi::print"]
}"#,
    )
    .expect("write host profile");

    let run_a = run_json_error_output(&wasm_path);
    let run_b = run_json_error_output(&wasm_path);
    assert_eq!(
        run_a, run_b,
        "runtime loader JSON output should be replay-stable for missing required capability"
    );
    assert_first_error_code(run_a.as_slice(), "R016");
}

#[test]
fn run_wasm_with_runtime_link_uses_mirror_when_primary_store_is_missing() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path();
    let wasm_path = root.join("out.wasm");

    Command::cargo_bin("clg")
        .expect("bin")
        .args(["emit-hello", "-o"])
        .arg(&wasm_path)
        .assert()
        .success();

    let store_dir = root.join("store");
    fs::create_dir_all(&store_dir).expect("create store");
    let primary_artifact_path = store_dir.join("hello.wasm");
    fs::copy(&wasm_path, &primary_artifact_path).expect("copy artifact");
    let artifact_bytes = fs::read(&primary_artifact_path).expect("read artifact");
    let digest = format!("sha256:{}", sha256_hex(artifact_bytes.as_slice()));
    write_runtime_loader_gate_artifacts(root, digest.as_str(), true);

    fs::remove_file(&primary_artifact_path).expect("remove primary artifact");
    let mirror_store = root.join("mirror-a").join("store");
    fs::create_dir_all(&mirror_store).expect("create mirror store");
    fs::copy(&wasm_path, mirror_store.join("hello.wasm")).expect("copy mirror artifact");
    fs::write(
        root.join("clg.runtime-loader.json"),
        r#"{
  "schema_version": 0,
  "offline_mode": true,
  "mirror_paths": ["mirror-a"],
  "artifact_read_retries": 2
}"#,
    )
    .expect("write runtime loader policy");

    Command::cargo_bin("clg")
        .expect("bin")
        .args(["run"])
        .arg(&wasm_path)
        .assert()
        .success()
        .stdout(predicates::str::contains("42\n"));
}

#[test]
fn runtime_loader_tamper_missing_artifact_reports_r012() {
    let (tmp, wasm_path) = setup_runtime_loader_fixture();
    let root = tmp.path();
    fs::remove_file(root.join("store").join("hello.wasm")).expect("remove artifact");
    let stdout = run_json_error_output(&wasm_path);
    assert_first_error_code(stdout.as_slice(), "R012");
}

#[test]
fn runtime_loader_tamper_digest_mismatch_reports_r013() {
    let (tmp, wasm_path) = setup_runtime_loader_fixture();
    let root = tmp.path();
    fs::write(root.join("store").join("hello.wasm"), b"tampered-bytes").expect("tamper artifact");
    let stdout = run_json_error_output(&wasm_path);
    assert_first_error_code(stdout.as_slice(), "R013");
}

#[test]
fn runtime_loader_tamper_untrusted_signer_reports_r014() {
    let (tmp, wasm_path) = setup_runtime_loader_fixture();
    let root = tmp.path();
    let trust_path = root.join("clg.trust-policy.json");
    let mut trust: Value =
        serde_json::from_slice(fs::read(&trust_path).expect("read trust policy").as_slice())
            .expect("parse trust policy");
    trust["revoked_key_ids"] = serde_json::json!(["k1"]);
    fs::write(
        &trust_path,
        serde_json::to_vec_pretty(&trust).expect("serialize trust policy"),
    )
    .expect("write trust policy");
    let stdout = run_json_error_output(&wasm_path);
    assert_first_error_code(stdout.as_slice(), "R014");
}

#[test]
fn shared_std_runtime_loader_rejects_unsupported_verified_abi_minor_range_with_r015() {
    let (_tmp, wasm_path) = setup_shared_std_runtime_loader_fixture(9, 9);
    let stdout = run_json_error_output(&wasm_path);
    assert_first_error_code(stdout.as_slice(), "R015");
    let value: Value = serde_json::from_slice(stdout.as_slice()).expect("json");
    let message = value["errors"][0]["message"]
        .as_str()
        .expect("runtime error message");
    assert!(
        message.contains("runtime shared std ABI mismatch"),
        "unexpected message: {message}"
    );
}

#[test]
fn shared_std_runtime_loader_tamper_missing_artifact_reports_r012() {
    let (tmp, wasm_path) = setup_shared_std_runtime_loader_fixture(0, 0);
    let root = tmp.path();
    fs::remove_file(root.join("std-packages").join("std-text-1.2.0.wasm"))
        .expect("remove shared std artifact");
    let stdout = run_json_error_output(&wasm_path);
    assert_first_error_code(stdout.as_slice(), "R012");
}

#[test]
fn shared_std_runtime_loader_tamper_untrusted_signer_reports_r014() {
    let (tmp, wasm_path) = setup_shared_std_runtime_loader_fixture(0, 0);
    let root = tmp.path();
    let trust_path = root.join("clg.trust-policy.json");
    let mut trust: Value =
        serde_json::from_slice(fs::read(&trust_path).expect("read trust policy").as_slice())
            .expect("parse trust policy");
    trust["revoked_key_ids"] = serde_json::json!(["k1"]);
    fs::write(
        &trust_path,
        serde_json::to_vec_pretty(&trust).expect("serialize trust policy"),
    )
    .expect("write trust policy");
    let stdout = run_json_error_output(&wasm_path);
    assert_first_error_code(stdout.as_slice(), "R014");
}

#[test]
fn runtime_loader_tamper_runtime_link_hash_mismatch_reports_r017() {
    let (tmp, wasm_path) = setup_runtime_loader_fixture();
    let root = tmp.path();
    fs::write(
        root.join("clg.runtime-link.sha256"),
        format!("{}\n", "a".repeat(64)),
    )
    .expect("tamper runtime link hash");
    let stdout = run_json_error_output(&wasm_path);
    assert_first_error_code(stdout.as_slice(), "R017");
}

#[test]
fn runtime_loader_replay_missing_artifact_json_is_deterministic() {
    let (tmp, wasm_path) = setup_runtime_loader_fixture();
    let root = tmp.path();
    fs::remove_file(root.join("store").join("hello.wasm")).expect("remove artifact");
    let run_a = run_json_error_output(&wasm_path);
    let run_b = run_json_error_output(&wasm_path);
    assert_eq!(
        run_a, run_b,
        "runtime loader JSON output should be replay-stable"
    );
    assert_first_error_code(run_a.as_slice(), "R012");
}

#[test]
fn runtime_loader_replay_digest_mismatch_json_is_deterministic() {
    let (tmp, wasm_path) = setup_runtime_loader_fixture();
    let root = tmp.path();
    fs::write(root.join("store").join("hello.wasm"), b"tampered-bytes").expect("tamper artifact");
    let run_a = run_json_error_output(&wasm_path);
    let run_b = run_json_error_output(&wasm_path);
    assert_eq!(
        run_a, run_b,
        "runtime loader JSON output should be replay-stable"
    );
    assert_first_error_code(run_a.as_slice(), "R013");
}

#[test]
fn runtime_loader_replay_untrusted_signer_json_is_deterministic() {
    let (tmp, wasm_path) = setup_runtime_loader_fixture();
    let root = tmp.path();
    let trust_path = root.join("clg.trust-policy.json");
    let mut trust: Value =
        serde_json::from_slice(fs::read(&trust_path).expect("read trust policy").as_slice())
            .expect("parse trust policy");
    trust["revoked_key_ids"] = serde_json::json!(["k1"]);
    fs::write(
        &trust_path,
        serde_json::to_vec_pretty(&trust).expect("serialize trust policy"),
    )
    .expect("write trust policy");

    let run_a = run_json_error_output(&wasm_path);
    let run_b = run_json_error_output(&wasm_path);
    assert_eq!(
        run_a, run_b,
        "runtime loader JSON output should be replay-stable"
    );
    assert_first_error_code(run_a.as_slice(), "R014");
}

#[test]
fn runtime_loader_replay_runtime_link_hash_mismatch_json_is_deterministic() {
    let (tmp, wasm_path) = setup_runtime_loader_fixture();
    let root = tmp.path();
    fs::write(
        root.join("clg.runtime-link.sha256"),
        format!("{}\n", "a".repeat(64)),
    )
    .expect("tamper runtime link hash");
    let run_a = run_json_error_output(&wasm_path);
    let run_b = run_json_error_output(&wasm_path);
    assert_eq!(
        run_a, run_b,
        "runtime loader JSON output should be replay-stable for runtime-link hash mismatch"
    );
    assert_first_error_code(run_a.as_slice(), "R017");
}

#[test]
fn runtime_loader_production_profile_without_runtime_link_fails_closed() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path();
    let wasm_path = root.join("out.wasm");

    Command::cargo_bin("clg")
        .expect("bin")
        .args(["emit-hello", "-o"])
        .arg(&wasm_path)
        .assert()
        .success();

    fs::write(
        root.join("clg.host-profile.json"),
        r#"{
  "schema_version": 0,
  "profile": "contract_static",
  "capabilities": ["std::wasi::print"]
}"#,
    )
    .expect("write host profile");

    let stdout = run_json_error_output(&wasm_path);
    assert_first_error_code(stdout.as_slice(), "R012");
}

