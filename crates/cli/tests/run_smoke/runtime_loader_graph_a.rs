#[test]
fn runtime_loader_rejects_provider_package_missing_host_capability_with_r016() {
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
    let provider_src = root.join("provider.clear");
    fs::write(
        &provider_src,
        r#"
            io function main() -> Int {
                std::env::time()
            }
        "#
        .trim(),
    )
    .expect("write provider source");
    let provider_wasm = store_dir.join("provider.wasm");
    Command::cargo_bin("clg")
        .expect("bin")
        .args(["build"])
        .arg(&provider_src)
        .args(["-o"])
        .arg(&provider_wasm)
        .assert()
        .success();

    let provider_bytes = fs::read(&provider_wasm).expect("read provider artifact");
    let provider_digest = format!("sha256:{}", sha256_hex(provider_bytes.as_slice()));
    let package_id = "pkg::provider@1.0.0";
    let package_name = "pkg::provider";
    let package_version = "1.0.0";
    let abi_id = "abi:pkg::provider:1.0.0";
    let runtime_link = serde_json::json!({
        "schema_version": 0,
        "resolver_version": 1,
        "packages": [
            {
                "id": package_id,
                "digest": provider_digest,
                "artifact_path": "store/provider.wasm",
                "abi_id": abi_id
            }
        ],
        "bindings": []
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
                {
                    "id": package_id,
                    "digest": provider_digest,
                    "path": "store/provider.wasm"
                }
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
                    "id": package_id,
                    "name": package_name,
                    "version": package_version,
                    "digest": provider_digest,
                    "abi_id": abi_id,
                    "dependencies": []
                }
            ]
        }))
        .expect("serialize lockfile"),
    )
    .expect("write lockfile");

    let signing = SigningKey::from_bytes(&[11u8; 32]);
    let signed_at = "2026-06-01T00:00:00Z";
    let payload = format!(
        "clg-package-signature-v0\n{package_name}\n{package_version}\n{provider_digest}\n{signed_at}\n"
    );
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
                    "name": package_name,
                    "version": package_version,
                    "digest": provider_digest,
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

    fs::write(
        root.join("clg.host-profile.json"),
        r#"{
  "schema_version": 0,
  "profile": "contract_static",
  "capabilities": ["std::wasi::print"]
}"#,
    )
    .expect("write host profile");

    let stdout = run_json_error_output(&app_wasm_path);
    let v: Value = serde_json::from_slice(&stdout).expect("json");
    let errors = v
        .get("errors")
        .and_then(|e| e.as_array())
        .expect("errors array");
    let first = errors.first().expect("first error");
    assert_eq!(first.get("code").and_then(|s| s.as_str()), Some("R016"));
    assert_eq!(first.get("stage").and_then(|s| s.as_str()), Some("runtime"));
    let msg = first
        .get("message")
        .and_then(|s| s.as_str())
        .expect("error message");
    assert!(
        msg.contains(package_id),
        "expected provider package id in deterministic R016 message"
    );
    assert!(
        msg.contains("std::env::time"),
        "expected missing provider capability in deterministic R016 message"
    );
}

#[test]
fn runtime_loader_links_provider_dependencies_using_topological_order() {
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

    let provider_a_wasm = store_dir.join("pkg-a.wasm");
    let provider_b_wasm = store_dir.join("pkg-b.wasm");
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
          (func (export "foo") (result i32)
            i32.const 7
          )
        )
        "#,
    )
    .expect("provider b wat");
    fs::write(&provider_a_wasm, &provider_a_bytes).expect("write provider a");
    fs::write(&provider_b_wasm, &provider_b_bytes).expect("write provider b");

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
                {
                    "id": provider_a_id,
                    "digest": provider_a_digest,
                    "path": "store/pkg-a.wasm"
                },
                {
                    "id": provider_b_id,
                    "digest": provider_b_digest,
                    "path": "store/pkg-b.wasm"
                }
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
                    "dependencies": []
                }
            ]
        }))
        .expect("serialize lockfile"),
    )
    .expect("write lockfile");

    let signing = SigningKey::from_bytes(&[13u8; 32]);
    let signed_at = "2026-06-01T00:00:00Z";
    let sign_entry = |name: &str, version: &str, digest: &str| {
        let payload =
            format!("clg-package-signature-v0\n{name}\n{version}\n{digest}\n{signed_at}\n");
        hex::encode(signing.sign(payload.as_bytes()).to_bytes())
    };
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
                    "name": "pkg::a",
                    "version": "1.0.0",
                    "digest": provider_a_digest,
                    "key_id": "k1",
                    "signed_at": signed_at,
                    "signature_format": "ed25519",
                    "signature": sign_entry("pkg::a", "1.0.0", provider_a_digest.as_str())
                },
                {
                    "name": "pkg::b",
                    "version": "1.0.0",
                    "digest": provider_b_digest,
                    "key_id": "k1",
                    "signed_at": signed_at,
                    "signature_format": "ed25519",
                    "signature": sign_entry("pkg::b", "1.0.0", provider_b_digest.as_str())
                }
            ]
        }))
        .expect("serialize signature file"),
    )
    .expect("write signature file");
    fs::write(
        root.join("clg.host-profile.json"),
        r#"{
  "schema_version": 0,
  "profile": "contract_static",
  "capabilities": ["std::wasi::print"]
}"#,
    )
    .expect("write host profile");

    Command::cargo_bin("clg")
        .expect("bin")
        .args(["run"])
        .arg(&app_wasm_path)
        .assert()
        .success()
        .stdout(predicates::str::contains("42\n"));
}

