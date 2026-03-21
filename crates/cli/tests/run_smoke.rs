use assert_cmd::Command;
use ed25519_dalek::{Signer, SigningKey};
use predicates::prelude::PredicateBooleanExt;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;
use std::path::PathBuf;

fn sample(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../clearlang-tests")
        .join(name)
}

fn example_project(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/projects")
        .join(name)
}

fn canonicalize_json_value(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut keys: Vec<_> = map.keys().cloned().collect();
            keys.sort();
            let mut out = serde_json::Map::new();
            for key in keys {
                out.insert(
                    key.clone(),
                    canonicalize_json_value(map.get(key.as_str()).expect("key exists")),
                );
            }
            Value::Object(out)
        }
        Value::Array(items) => Value::Array(items.iter().map(canonicalize_json_value).collect()),
        _ => value.clone(),
    }
}

fn canonical_json_bytes(value: &Value) -> Vec<u8> {
    serde_json::to_vec(&canonicalize_json_value(value)).expect("canonical json")
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn write_runtime_loader_gate_artifacts(root: &Path, digest: &str, include_store_index: bool) {
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
                "provider_symbol": "main"
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

    if include_store_index {
        fs::write(
            root.join("clg.package-store-index.json"),
            serde_json::to_vec_pretty(&serde_json::json!({
                "schema_version": 0,
                "artifacts": [
                    {
                        "id": "app::hello@1.0.0",
                        "digest": digest,
                        "path": "store/hello.wasm"
                    }
                ]
            }))
            .expect("serialize store index"),
        )
        .expect("write store index");
    }

    fs::write(
        root.join("clg.lock.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "schema_version": 1,
            "resolver_version": 1,
            "roots": [],
            "packages": [
                {
                    "id": "app::hello@1.0.0",
                    "name": "app::hello",
                    "version": "1.0.0",
                    "digest": digest,
                    "abi_id": "abi:app::hello:1.0.0",
                    "dependencies": []
                }
            ]
        }))
        .expect("serialize lockfile"),
    )
    .expect("write lockfile");

    let signing = SigningKey::from_bytes(&[11u8; 32]);
    let signed_at = "2026-06-01T00:00:00Z";
    let payload = format!("clg-package-signature-v0\napp::hello\n1.0.0\n{digest}\n{signed_at}\n");
    let signature = hex::encode(signing.sign(payload.as_bytes()).to_bytes());
    fs::write(
        root.join("clg.trust-policy.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "schema_version": 0,
            "trusted_signers": [
                {
                    "key_id": "k1",
                    "scheme": "ed25519",
                    "public_key": format!("hex:{}", hex::encode(signing.verifying_key().to_bytes())),
                    "not_before": "2026-01-01T00:00:00Z",
                    "not_after": "2027-01-01T00:00:00Z"
                }
            ],
            "revoked_key_ids": []
        }))
        .expect("serialize trust policy"),
    )
    .expect("write trust policy");
    fs::write(
        root.join("clg.package-signatures.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "schema_version": 0,
            "signatures": [
                {
                    "name": "app::hello",
                    "version": "1.0.0",
                    "digest": digest,
                    "key_id": "k1",
                    "signed_at": signed_at,
                    "signature_format": "ed25519",
                    "signature": signature
                }
            ]
        }))
        .expect("serialize signature file"),
    )
    .expect("write signature file");
}

fn setup_runtime_loader_fixture() -> (tempfile::TempDir, PathBuf) {
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

    (tmp, wasm_path)
}

fn run_json_error_output(file: &Path) -> Vec<u8> {
    let output = Command::cargo_bin("clg")
        .expect("bin")
        .args(["--json-errors", "run"])
        .arg(file)
        .output()
        .expect("run command");
    assert!(!output.status.success(), "run should fail");
    output.stdout
}

fn assert_first_error_code(stdout: &[u8], expected_code: &str) {
    let v: Value = serde_json::from_slice(stdout).expect("json");
    assert_eq!(v.get("ok").and_then(|b| b.as_bool()), Some(false));
    let errors = v
        .get("errors")
        .and_then(|e| e.as_array())
        .expect("errors array");
    assert!(!errors.is_empty());
    let first = &errors[0];
    assert_eq!(
        first.get("code").and_then(|s| s.as_str()),
        Some(expected_code)
    );
    assert_eq!(first.get("stage").and_then(|s| s.as_str()), Some("runtime"));
}

#[test]
fn emit_hello_and_run() {
    let out = tempfile::Builder::new()
        .prefix("emit_hello_")
        .suffix(".wasm")
        .tempfile()
        .unwrap();
    let out_path = out.path().to_path_buf();

    Command::cargo_bin("clg")
        .expect("bin")
        .args(["emit-hello", "-o"])
        .arg(&out_path)
        .assert()
        .success();

    Command::cargo_bin("clg")
        .expect("bin")
        .args(["run"])
        .arg(&out_path)
        .assert()
        .success()
        .stdout(predicates::str::contains("42\n"));
}

#[test]
fn build_and_run_samples() {
    // 01_hello
    let out1 = tempfile::tempdir().unwrap().path().join("hello.wasm");
    Command::cargo_bin("clg")
        .unwrap()
        .args(["build"])
        .arg(sample("01_hello.clear"))
        .args(["-o"])
        .arg(&out1)
        .assert()
        .success();
    Command::cargo_bin("clg")
        .unwrap()
        .args(["run"])
        .arg(&out1)
        .assert()
        .success()
        .stdout(predicates::str::contains("42\n"));

    // 12_zero_arg_fn
    let out2 = tempfile::tempdir().unwrap().path().join("zfn.wasm");
    Command::cargo_bin("clg")
        .unwrap()
        .args(["build"])
        .arg(sample("12_zero_arg_fn.clear"))
        .args(["-o"])
        .arg(&out2)
        .assert()
        .success();
    Command::cargo_bin("clg")
        .unwrap()
        .args(["run"])
        .arg(&out2)
        .assert()
        .success()
        .stdout(predicates::str::contains("7\n"));
}

#[test]
fn run_can_execute_clear_source_directly() {
    Command::cargo_bin("clg")
        .unwrap()
        .args(["run"])
        .arg(sample("02_arith.clear"))
        .assert()
        .success()
        .stdout(predicates::str::contains("15\n"));
}

#[test]
fn run_supports_comments_and_numeric_separators() {
    Command::cargo_bin("clg")
        .unwrap()
        .args(["run"])
        .arg(sample("19_comments_numeric_separator.clear"))
        .assert()
        .success()
        .stdout(predicates::str::contains("1000\n"));
}

#[test]
fn run_surfaces_build_type_errors_for_clear_source_json() {
    let output = Command::cargo_bin("clg")
        .unwrap()
        .args(["--json-errors", "run"])
        .arg(sample("14_arity_mismatch.clear"))
        .output()
        .expect("run command");
    assert!(!output.status.success(), "run should fail");
    let v: Value = serde_json::from_slice(&output.stdout).expect("json parse");
    assert_eq!(v.get("ok").and_then(|b| b.as_bool()), Some(false));
    let errors = v
        .get("errors")
        .and_then(|e| e.as_array())
        .expect("errors array");
    assert!(!errors.is_empty());
    let first = &errors[0];
    assert_eq!(first.get("code").and_then(|s| s.as_str()), Some("T002"));
    assert_eq!(first.get("stage").and_then(|s| s.as_str()), Some("type"));
}

#[test]
fn parse_command_prints_ast() {
    let sample = sample("01_hello.clear");
    Command::cargo_bin("clg")
        .unwrap()
        .args(["parse"])
        .arg(&sample)
        .assert()
        .success()
        .stdout(predicates::str::contains("add2").and(predicates::str::contains("main")));
}

#[test]
fn build_fails_on_parse_error() {
    let out = tempfile::tempdir().unwrap().path().join("bad.wasm");
    Command::cargo_bin("clg")
        .unwrap()
        .args(["build"])
        .arg(sample("06_trailing_call_comma.clear"))
        .args(["-o"])
        .arg(&out)
        .assert()
        .failure();
}

#[test]
fn build_fails_on_type_error() {
    let out = tempfile::tempdir().unwrap().path().join("type_err.wasm");
    Command::cargo_bin("clg")
        .unwrap()
        .args(["build"])
        .arg(sample("14_arity_mismatch.clear"))
        .args(["-o"])
        .arg(&out)
        .assert()
        .failure();
}

#[test]
fn parse_json_errors_are_structured() {
    let output = Command::cargo_bin("clg")
        .unwrap()
        .args(["--json-errors", "parse"])
        .arg(sample("06_trailing_call_comma.clear"))
        .output()
        .expect("run parse command");
    assert!(!output.status.success(), "parse should fail");
    let v: Value = serde_json::from_slice(&output.stdout).expect("json parse");
    assert_eq!(v.get("ok").and_then(|b| b.as_bool()), Some(false));
    let errors = v
        .get("errors")
        .and_then(|e| e.as_array())
        .expect("errors array");
    assert!(!errors.is_empty());
    let first = &errors[0];
    assert_eq!(first.get("code").and_then(|s| s.as_str()), Some("P001"));
    assert_eq!(first.get("stage").and_then(|s| s.as_str()), Some("parse"));
}

#[test]
fn build_json_errors_surface_type_failures() {
    let tmp = tempfile::tempdir().unwrap();
    let out_path = tmp.path().join("type_err.wasm");
    let output = Command::cargo_bin("clg")
        .unwrap()
        .args(["--json-errors", "build"])
        .arg(sample("14_arity_mismatch.clear"))
        .args(["-o"])
        .arg(&out_path)
        .output()
        .expect("run build command");
    assert!(!output.status.success(), "build should fail");
    let v: Value = serde_json::from_slice(&output.stdout).expect("json parse");
    assert_eq!(v.get("ok").and_then(|b| b.as_bool()), Some(false));
    let errors = v
        .get("errors")
        .and_then(|e| e.as_array())
        .expect("errors array");
    assert!(!errors.is_empty());
    let first = &errors[0];
    assert_eq!(first.get("code").and_then(|s| s.as_str()), Some("T002"));
    assert_eq!(first.get("stage").and_then(|s| s.as_str()), Some("type"));
}

#[test]
fn parse_json_errors_suggest_underscore_for_comma_grouped_number() {
    let output = Command::cargo_bin("clg")
        .unwrap()
        .args(["--json-errors", "parse"])
        .arg(sample("20_comma_numeric_separator_invalid.clear"))
        .output()
        .expect("run parse command");
    assert!(!output.status.success(), "parse should fail");
    let v: Value = serde_json::from_slice(&output.stdout).expect("json parse");
    assert_eq!(v.get("ok").and_then(|b| b.as_bool()), Some(false));
    let errors = v
        .get("errors")
        .and_then(|e| e.as_array())
        .expect("errors array");
    assert!(
        errors.iter().any(|e| {
            e.get("message")
                .and_then(|m| m.as_str())
                .map(|m| m.contains("use `_`"))
                .unwrap_or(false)
        }),
        "expected underscore separator suggestion in parse diagnostics"
    );
}

#[test]
fn run_generic_project_fixture() {
    Command::cargo_bin("clg")
        .unwrap()
        .args(["run"])
        .arg(example_project("generic/main.clear"))
        .assert()
        .success()
        .stdout(predicates::str::contains("generic: checkout pipeline"))
        .stdout(predicates::str::contains("900"));
}

#[test]
fn run_crypto_project_fixture() {
    Command::cargo_bin("clg")
        .unwrap()
        .args(["run"])
        .arg(example_project("crypto/main.clear"))
        .assert()
        .success()
        .stdout(predicates::str::contains("crypto: auth pipeline"))
        .stdout(predicates::str::contains("1"));
}

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
fn run_wasm_with_runtime_link_missing_provider_symbol_fails_with_r015() {
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
    assert_eq!(first.get("code").and_then(|s| s.as_str()), Some("R015"));
    assert_eq!(first.get("stage").and_then(|s| s.as_str()), Some("runtime"));
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
