#[test]
fn runtime_loader_rejects_ambiguous_provider_bindings_with_r015() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path();
    let app_wasm_path = root.join("out.wasm");

    Command::cargo_bin("clg")
        .expect("bin")
        .args(["emit-hello", "-o"])
        .arg(&app_wasm_path)
        .assert()
        .success();

    let store_dir = root.join("store");
    fs::create_dir_all(&store_dir).expect("create store");
    let provider_a_id = "pkg::a@1.0.0";
    let provider_b_id = "pkg::b@1.0.0";
    let provider_a_bytes = b"artifact-a".to_vec();
    let provider_b_bytes = b"artifact-b".to_vec();
    fs::write(store_dir.join("pkg-a.wasm"), &provider_a_bytes).expect("write provider a");
    fs::write(store_dir.join("pkg-b.wasm"), &provider_b_bytes).expect("write provider b");
    let provider_a_digest = format!("sha256:{}", sha256_hex(provider_a_bytes.as_slice()));
    let provider_b_digest = format!("sha256:{}", sha256_hex(provider_b_bytes.as_slice()));

    let runtime_link = serde_json::json!({
        "schema_version": 0,
        "resolver_version": 1,
        "packages": [
            {
                "id": provider_a_id,
                "digest": provider_a_digest,
                "artifact_path": "store/pkg-a.wasm",
                "abi_id": "abi:pkg::a:1.0.0"
            },
            {
                "id": provider_b_id,
                "digest": provider_b_digest,
                "artifact_path": "store/pkg-b.wasm",
                "abi_id": "abi:pkg::b:1.0.0"
            }
        ],
        "bindings": [
            {
                "import_module": "pkg::shared",
                "import_name": "foo",
                "provider_package_id": provider_a_id,
                "provider_symbol": "foo"
            },
            {
                "import_module": "pkg::shared",
                "import_name": "foo",
                "provider_package_id": provider_b_id,
                "provider_symbol": "foo"
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

    fs::write(
        root.join("clg.package-store-index.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "schema_version": 0,
            "artifacts": [
                { "id": provider_a_id, "digest": provider_a_digest, "path": "store/pkg-a.wasm" },
                { "id": provider_b_id, "digest": provider_b_digest, "path": "store/pkg-b.wasm" }
            ]
        }))
        .expect("serialize store index"),
    )
    .expect("write store index");

    fs::write(
        root.join("clg.lock.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "schema_version": 1,
            "resolver_version": 1,
            "roots": [],
            "packages": [
                {
                    "id": provider_a_id,
                    "name": "pkg::a",
                    "version": "1.0.0",
                    "digest": provider_a_digest,
                    "abi_id": "abi:pkg::a:1.0.0",
                    "dependencies": []
                },
                {
                    "id": provider_b_id,
                    "name": "pkg::b",
                    "version": "1.0.0",
                    "digest": provider_b_digest,
                    "abi_id": "abi:pkg::b:1.0.0",
                    "dependencies": []
                }
            ]
        }))
        .expect("serialize lockfile"),
    )
    .expect("write lockfile");

    write_runtime_trust_inputs(
        root,
        &[
            ("pkg::a", "1.0.0", provider_a_digest.as_str()),
            ("pkg::b", "1.0.0", provider_b_digest.as_str()),
        ],
    );

    let output = Command::cargo_bin("clg")
        .expect("bin")
        .args(["--json-errors", "run"])
        .arg(&app_wasm_path)
        .output()
        .expect("run command");
    assert!(!output.status.success(), "run should fail");
    let v: Value = serde_json::from_slice(&output.stdout).expect("json");
    let errors = v
        .get("errors")
        .and_then(|e| e.as_array())
        .expect("errors array");
    let first = errors.first().expect("first error");
    assert_eq!(first.get("code").and_then(|s| s.as_str()), Some("R015"));
    let msg = first
        .get("message")
        .and_then(|s| s.as_str())
        .expect("error message");
    assert!(msg.contains("ambiguous provider bindings"));
}

#[test]
fn runtime_loader_rejects_provider_dependency_cycle_with_r015() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path();
    let app_wasm_path = root.join("out.wasm");

    Command::cargo_bin("clg")
        .expect("bin")
        .args(["emit-hello", "-o"])
        .arg(&app_wasm_path)
        .assert()
        .success();

    let store_dir = root.join("store");
    fs::create_dir_all(&store_dir).expect("create store");

    let provider_a_id = "pkg::a@1.0.0";
    let provider_b_id = "pkg::b@1.0.0";

    let provider_a_bytes = wat_parse_str(
        r#"
        (module
          (import "pkg::b" "foo" (func $foo (result i32)))
          (func (export "foo") (result i32)
            call $foo
          )
        )
        "#,
    )
    .expect("provider a wat");
    let provider_b_bytes = wat_parse_str(
        r#"
        (module
          (import "pkg::a" "foo" (func $foo (result i32)))
          (func (export "foo") (result i32)
            call $foo
          )
        )
        "#,
    )
    .expect("provider b wat");
    fs::write(store_dir.join("pkg-a.wasm"), &provider_a_bytes).expect("write provider a");
    fs::write(store_dir.join("pkg-b.wasm"), &provider_b_bytes).expect("write provider b");
    let provider_a_digest = format!("sha256:{}", sha256_hex(provider_a_bytes.as_slice()));
    let provider_b_digest = format!("sha256:{}", sha256_hex(provider_b_bytes.as_slice()));

    let runtime_link = serde_json::json!({
        "schema_version": 0,
        "resolver_version": 1,
        "packages": [
            {
                "id": provider_a_id,
                "digest": provider_a_digest,
                "artifact_path": "store/pkg-a.wasm",
                "abi_id": "abi:pkg::a:1.0.0"
            },
            {
                "id": provider_b_id,
                "digest": provider_b_digest,
                "artifact_path": "store/pkg-b.wasm",
                "abi_id": "abi:pkg::b:1.0.0"
            }
        ],
        "bindings": [
            {
                "import_module": "pkg::a",
                "import_name": "foo",
                "provider_package_id": provider_a_id,
                "provider_symbol": "foo"
            },
            {
                "import_module": "pkg::b",
                "import_name": "foo",
                "provider_package_id": provider_b_id,
                "provider_symbol": "foo"
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

    fs::write(
        root.join("clg.package-store-index.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "schema_version": 0,
            "artifacts": [
                { "id": provider_a_id, "digest": provider_a_digest, "path": "store/pkg-a.wasm" },
                { "id": provider_b_id, "digest": provider_b_digest, "path": "store/pkg-b.wasm" }
            ]
        }))
        .expect("serialize store index"),
    )
    .expect("write store index");

    fs::write(
        root.join("clg.lock.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "schema_version": 1,
            "resolver_version": 1,
            "roots": [],
            "packages": [
                {
                    "id": provider_a_id,
                    "name": "pkg::a",
                    "version": "1.0.0",
                    "digest": provider_a_digest,
                    "abi_id": "abi:pkg::a:1.0.0",
                    "dependencies": [provider_b_id]
                },
                {
                    "id": provider_b_id,
                    "name": "pkg::b",
                    "version": "1.0.0",
                    "digest": provider_b_digest,
                    "abi_id": "abi:pkg::b:1.0.0",
                    "dependencies": [provider_a_id]
                }
            ]
        }))
        .expect("serialize lockfile"),
    )
    .expect("write lockfile");

    write_runtime_trust_inputs(
        root,
        &[
            ("pkg::a", "1.0.0", provider_a_digest.as_str()),
            ("pkg::b", "1.0.0", provider_b_digest.as_str()),
        ],
    );

    let output = Command::cargo_bin("clg")
        .expect("bin")
        .args(["--json-errors", "run"])
        .arg(&app_wasm_path)
        .output()
        .expect("run command");
    assert!(!output.status.success(), "run should fail");
    let v: Value = serde_json::from_slice(&output.stdout).expect("json");
    let errors = v
        .get("errors")
        .and_then(|e| e.as_array())
        .expect("errors array");
    let first = errors.first().expect("first error");
    assert_eq!(first.get("code").and_then(|s| s.as_str()), Some("R015"));
    let msg = first
        .get("message")
        .and_then(|s| s.as_str())
        .expect("error message");
    assert!(msg.contains("provider dependency cycle"));
}
