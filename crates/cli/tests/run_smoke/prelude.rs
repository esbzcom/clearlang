use assert_cmd::Command;
use ed25519_dalek::{Signer, SigningKey};
use predicates::prelude::PredicateBooleanExt;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use wat::parse_str as wat_parse_str;

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

fn write_runtime_trust_inputs(root: &Path, entries: &[(&str, &str, &str)]) {
    let signing = SigningKey::from_bytes(&[17u8; 32]);
    let signed_at = "2026-06-01T00:00:00Z";
    let signatures = entries
        .iter()
        .map(|(name, version, digest)| {
            let payload =
                format!("clg-package-signature-v0\n{name}\n{version}\n{digest}\n{signed_at}\n");
            let signature = hex::encode(signing.sign(payload.as_bytes()).to_bytes());
            serde_json::json!({
                "name": name,
                "version": version,
                "digest": digest,
                "key_id": "k1",
                "signed_at": signed_at,
                "signature_format": "ed25519",
                "signature": signature
            })
        })
        .collect::<Vec<_>>();
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
            "signatures": signatures
        }))
        .expect("serialize signature file"),
    )
    .expect("write signature file");
}

fn write_empty_runtime_link(root: &Path) {
    let runtime_link = serde_json::json!({
        "schema_version": 0,
        "resolver_version": 1,
        "packages": [],
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

fn write_shared_std_runtime_loader_gate_artifacts(
    root: &Path,
    package_id: &str,
    version: &str,
    artifact_text: &str,
    verified_std_abi_minor_min: u32,
    verified_std_abi_minor_max: u32,
) {
    let store_dir = root.join("std-packages");
    fs::create_dir_all(&store_dir).expect("create std-packages");
    let artifact_stem = format!("{}-{}", package_id.replace("::", "-"), version);
    let artifact_name = format!("{artifact_stem}.wasm");
    let artifact_path = store_dir.join(&artifact_name);
    fs::write(&artifact_path, artifact_text.as_bytes()).expect("write shared std artifact");
    let digest = format!("sha256:{}", sha256_hex(artifact_text.as_bytes()));
    let package_release_id = format!("{package_id}@{version}");

    let runtime_link = serde_json::json!({
        "schema_version": 0,
        "resolver_version": 1,
        "packages": [
            {
                "id": package_release_id,
                "digest": digest,
                "artifact_path": format!("std-packages/{artifact_name}"),
                "abi_id": format!("abi:{package_id}:{version}")
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
                    "id": package_release_id,
                    "digest": digest,
                    "path": format!("std-packages/{artifact_name}")
                }
            ]
        }))
        .expect("serialize store index"),
    )
    .expect("write store index");

    fs::write(
        root.join("clg.lock.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "schema_version": 2,
            "resolver_version": 1,
            "roots": [],
            "packages": [],
            "std": {
                "delivery": "shared",
                "packages": [
                    {
                        "package_id": package_id,
                        "version": version,
                        "verified_std_abi": {
                            "major": 1,
                            "minor_min": verified_std_abi_minor_min,
                            "minor_max": verified_std_abi_minor_max
                        },
                        "artifact": {
                            "format": "wasm",
                            "path": format!("std-packages/{artifact_name}"),
                            "digest": digest,
                            "size_bytes": artifact_text.len()
                        },
                        "signature": {
                            "key_id": "k1",
                            "algorithm": "ed25519",
                            "signed_at": "2026-06-01T00:00:00Z",
                            "signature": "placeholder"
                        },
                        "provenance": {
                            "statement_digest": "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
                            "statement_format": "in-toto-v1"
                        },
                        "symbols": match package_id {
                            "std::eth" => serde_json::json!(["std::eth::from_array", "std::eth::from_bytes"]),
                            "std::solana" => serde_json::json!(["std::solana::from_array", "std::solana::from_bytes"]),
                            _ => serde_json::json!(["std::bytes", "std::str", "std::str_pattern"]),
                        },
                        "dependencies": []
                    }
                ]
            }
        }))
        .expect("serialize lockfile"),
    )
    .expect("write lockfile");

    let signing = SigningKey::from_bytes(&[7u8; 32]);
    let signed_at = "2026-06-01T00:00:00Z";
    let payload =
        format!("clg-package-signature-v0\n{package_id}\n{version}\n{digest}\n{signed_at}\n");
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
                    "name": package_id,
                    "version": version,
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

fn setup_shared_std_runtime_loader_fixture(
    package_id: &str,
    version: &str,
    verified_std_abi_minor_min: u32,
    verified_std_abi_minor_max: u32,
) -> (tempfile::TempDir, PathBuf) {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path();
    let wasm_path = root.join("out.wasm");

    Command::cargo_bin("clg")
        .expect("bin")
        .args(["emit-hello", "-o"])
        .arg(&wasm_path)
        .assert()
        .success();

    write_shared_std_runtime_loader_gate_artifacts(
        root,
        package_id,
        version,
        "shared-std-artifact",
        verified_std_abi_minor_min,
        verified_std_abi_minor_max,
    );

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

