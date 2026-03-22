use super::*;
use ed25519_dalek::{Signer, SigningKey};
use std::path::Path;

fn write_file(path: &Path, text: &str) {
    fs::write(path, text).expect("write file");
}

fn write_baseline_runtime_artifacts(root: &Path, artifact_text: &str) {
    let store_dir = root.join("store");
    fs::create_dir_all(&store_dir).expect("create store dir");
    write_file(store_dir.join("pkg-a.wasm").as_path(), artifact_text);

    let digest = format!("sha256:{}", sha256_hex(artifact_text.as_bytes()));
    let runtime_link = serde_json::json!({
        "schema_version": 0,
        "resolver_version": 1,
        "packages": [
            {
                "id": "pkg::a@1.0.0",
                "digest": digest,
                "artifact_path": "store/pkg-a.wasm",
                "abi_id": "abi:pkg::a:1.0.0"
            }
        ],
        "bindings": [
            {
                "import_module": "pkg::a",
                "import_name": "add",
                "provider_package_id": "pkg::a@1.0.0",
                "provider_symbol": "add"
            }
        ]
    });
    write_file(
        root.join(RUNTIME_LINK_FILE).as_path(),
        serde_json::to_string_pretty(&runtime_link)
            .expect("serialize runtime-link")
            .as_str(),
    );
    write_file(
        root.join(RUNTIME_LINK_HASH_FILE).as_path(),
        format!(
            "{}\n",
            sha256_hex(canonical_json_bytes(&runtime_link).as_slice())
        )
        .as_str(),
    );

    write_file(
        root.join(PACKAGE_STORE_INDEX_FILE).as_path(),
        serde_json::to_string_pretty(&serde_json::json!({
            "schema_version": 0,
            "artifacts": [
                {
                    "id": "pkg::a@1.0.0",
                    "digest": digest,
                    "path": "store/pkg-a.wasm"
                }
            ]
        }))
        .expect("serialize store index")
        .as_str(),
    );

    write_file(
        root.join(STRICT_LOCKFILE_FILE).as_path(),
        serde_json::to_string_pretty(&serde_json::json!({
            "schema_version": 1,
            "resolver_version": 1,
            "roots": [],
            "packages": [
                {
                    "id": "pkg::a@1.0.0",
                    "name": "pkg::a",
                    "version": "1.0.0",
                    "digest": digest,
                    "abi_id": "abi:pkg::a:1.0.0",
                    "dependencies": []
                }
            ]
        }))
        .expect("serialize lockfile")
        .as_str(),
    );

    let signing = SigningKey::from_bytes(&[7u8; 32]);
    let signed_at = "2026-06-01T00:00:00Z";
    let payload = canonical_signature_payload_v0("pkg::a", "1.0.0", digest.as_str(), signed_at);
    let signature = hex::encode(signing.sign(payload.as_bytes()).to_bytes());

    write_file(
            root.join("clg.trust-policy.json").as_path(),
            serde_json::to_string_pretty(&serde_json::json!({
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
            .expect("serialize trust policy")
            .as_str(),
        );

    write_file(
        root.join(STRICT_PACKAGE_SIGNATURES_FILE).as_path(),
        serde_json::to_string_pretty(&serde_json::json!({
            "schema_version": 0,
            "signatures": [
                {
                    "name": "pkg::a",
                    "version": "1.0.0",
                    "digest": digest,
                    "key_id": "k1",
                    "signed_at": signed_at,
                    "signature_format": "ed25519",
                    "signature": signature
                }
            ]
        }))
        .expect("serialize signature file")
        .as_str(),
    );
}

#[test]
fn returns_none_when_runtime_link_is_absent() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let result = load_runtime_packages_from_local_store_if_present(
        tmp.path(),
        RuntimeLoaderConfig::default(),
    )
    .expect("no runtime link should be accepted");
    assert!(result.is_none());
}

#[test]
fn production_host_profile_requires_runtime_link_artifact() {
    let tmp = tempfile::tempdir().expect("tempdir");
    write_file(
        tmp.path().join(HOST_PROFILE_FILE).as_path(),
        r#"{
  "schema_version": 0,
  "profile": "contract_static",
  "capabilities": ["std::wasi::print"]
}"#,
    );
    let err = load_runtime_packages_from_local_store_if_present(
        tmp.path(),
        RuntimeLoaderConfig::default(),
    )
    .expect_err("missing runtime-link should fail closed in production profile");
    assert_eq!(err.code(), "R012");
    assert!(err.message().contains("fail-closed"));
}

#[test]
fn runtime_host_profile_rejects_unsupported_capability_with_r016() {
    let tmp = tempfile::tempdir().expect("tempdir");
    write_file(
        tmp.path().join(HOST_PROFILE_FILE).as_path(),
        r#"{
  "schema_version": 0,
  "profile": "contract_static",
  "capabilities": ["std::env::unknown"]
}"#,
    );
    let err = load_runtime_packages_from_local_store_if_present(
        tmp.path(),
        RuntimeLoaderConfig::default(),
    )
    .expect_err("unsupported capability should fail");
    assert_eq!(err.code(), "R016");
    assert!(err.message().contains("unsupported capability"));
}

#[test]
fn runtime_host_profile_rejects_duplicate_capability_with_r016() {
    let tmp = tempfile::tempdir().expect("tempdir");
    write_file(
        tmp.path().join(HOST_PROFILE_FILE).as_path(),
        r#"{
  "schema_version": 0,
  "profile": "contract_static",
  "capabilities": ["std::crypto::hash", "std::crypto::hash", "std::wasi::print", "std::wasi::print"]
}"#,
    );
    let err = load_runtime_packages_from_local_store_if_present(
        tmp.path(),
        RuntimeLoaderConfig::default(),
    )
    .expect_err("duplicate capability should fail");
    assert_eq!(err.code(), "R016");
    assert!(err.message().contains("duplicate capability"));
    assert!(err.message().contains("std::crypto::hash"));
}

#[test]
fn runtime_host_profile_rejects_empty_capability_with_r016() {
    let tmp = tempfile::tempdir().expect("tempdir");
    write_file(
        tmp.path().join(HOST_PROFILE_FILE).as_path(),
        r#"{
  "schema_version": 0,
  "profile": "shared_app",
  "capabilities": ["", "std::wasi::print"]
}"#,
    );
    let err = load_runtime_packages_from_local_store_if_present(
        tmp.path(),
        RuntimeLoaderConfig::default(),
    )
    .expect_err("empty capability should fail");
    assert_eq!(err.code(), "R016");
    assert!(err.message().contains("empty capability id"));
}

#[test]
fn runtime_host_profile_missing_required_capability_with_r016() {
    let tmp = tempfile::tempdir().expect("tempdir");
    write_baseline_runtime_artifacts(tmp.path(), "artifact");
    write_file(
        tmp.path().join(HOST_PROFILE_FILE).as_path(),
        r#"{
  "schema_version": 0,
  "profile": "contract_static",
  "capabilities": ["std::wasi::print"]
}"#,
    );
    let err = load_runtime_packages_from_local_store_if_present(
        tmp.path(),
        RuntimeLoaderConfig {
            allow_remote_fetch: false,
            required_host_capabilities: vec!["std::env::time".to_string()],
        },
    )
    .expect_err("missing required capability should fail");
    assert_eq!(err.code(), "R016");
    assert!(err.message().contains("missing required capabilities"));
    assert!(err.message().contains("std::env::time"));
}

#[test]
fn rejects_remote_fetch_in_phase_23_0_1() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let err = load_runtime_packages_from_local_store_if_present(
        tmp.path(),
        RuntimeLoaderConfig {
            allow_remote_fetch: true,
            required_host_capabilities: Vec::new(),
        },
    )
    .expect_err("remote fetch should be rejected");
    assert_eq!(err.code(), "R012");
}

#[test]
fn loads_runtime_packages_from_local_store_index() {
    let tmp = tempfile::tempdir().expect("tempdir");
    write_baseline_runtime_artifacts(
        tmp.path(),
        "(module (func (export \"main\") (result i32) i32.const 0))",
    );
    let loaded = load_runtime_packages_from_local_store_if_present(
        tmp.path(),
        RuntimeLoaderConfig::default(),
    )
    .expect("load runtime packages")
    .expect("runtime-link present");
    assert_eq!(loaded.packages.len(), 1);
    assert_eq!(loaded.packages[0].id, "pkg::a@1.0.0");
}

#[test]
fn rejects_runtime_link_hash_mismatch_with_r017() {
    let tmp = tempfile::tempdir().expect("tempdir");
    write_baseline_runtime_artifacts(tmp.path(), "artifact");
    write_file(
        tmp.path().join(RUNTIME_LINK_HASH_FILE).as_path(),
        &"a".repeat(64),
    );

    let err = load_runtime_packages_from_local_store_if_present(
        tmp.path(),
        RuntimeLoaderConfig::default(),
    )
    .expect_err("expected hash mismatch");
    assert_eq!(err.code(), "R017");
}

#[test]
fn rejects_lockfile_digest_mismatch_with_r013() {
    let tmp = tempfile::tempdir().expect("tempdir");
    write_baseline_runtime_artifacts(tmp.path(), "artifact");
    write_file(
        tmp.path().join(STRICT_LOCKFILE_FILE).as_path(),
        r#"{
  "schema_version": 1,
  "resolver_version": 1,
  "roots": [],
  "packages": [
    {
      "id": "pkg::a@1.0.0",
      "name": "pkg::a",
      "version": "1.0.0",
      "digest": "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
      "abi_id": "abi:pkg::a:1.0.0",
      "dependencies": []
    }
  ]
}"#,
    );

    let err = load_runtime_packages_from_local_store_if_present(
        tmp.path(),
        RuntimeLoaderConfig::default(),
    )
    .expect_err("expected lock digest mismatch");
    assert_eq!(err.code(), "R013");
}

#[test]
fn rejects_lockfile_v1_invalid_root_requirement_with_r013() {
    let tmp = tempfile::tempdir().expect("tempdir");
    write_baseline_runtime_artifacts(tmp.path(), "artifact");
    write_file(
        tmp.path().join(STRICT_LOCKFILE_FILE).as_path(),
        r#"{
  "schema_version": 1,
  "resolver_version": 1,
  "roots": [
    {
      "name": "app",
      "dependencies": [{ "name": "pkg::a", "requirement": "latest" }]
    }
  ],
  "packages": [
    {
      "id": "pkg::a@1.0.0",
      "name": "pkg::a",
      "version": "1.0.0",
      "digest": "sha256:c7c5c1d70c5dec441d7d17042f8bbf5be4c4bf4a4c8f89f46b5f58211a711000",
      "abi_id": "abi:pkg::a:1.0.0",
      "dependencies": []
    }
  ]
}"#,
    );

    let err = load_runtime_packages_from_local_store_if_present(
        tmp.path(),
        RuntimeLoaderConfig::default(),
    )
    .expect_err("expected invalid root requirement");
    assert_eq!(err.code(), "R013");
    assert!(err.message().contains("invalid requirement"));
}

#[test]
fn rejects_lockfile_v1_invalid_dependency_id_with_r013() {
    let tmp = tempfile::tempdir().expect("tempdir");
    write_baseline_runtime_artifacts(tmp.path(), "artifact");
    write_file(
        tmp.path().join(STRICT_LOCKFILE_FILE).as_path(),
        r#"{
  "schema_version": 1,
  "resolver_version": 1,
  "roots": [],
  "packages": [
    {
      "id": "pkg::a@1.0.0",
      "name": "pkg::a",
      "version": "1.0.0",
      "digest": "sha256:c7c5c1d70c5dec441d7d17042f8bbf5be4c4bf4a4c8f89f46b5f58211a711000",
      "abi_id": "abi:pkg::a:1.0.0",
      "dependencies": ["not_a_package_id"]
    }
  ]
}"#,
    );

    let err = load_runtime_packages_from_local_store_if_present(
        tmp.path(),
        RuntimeLoaderConfig::default(),
    )
    .expect_err("expected invalid package dependency id");
    assert_eq!(err.code(), "R013");
    assert!(err.message().contains("invalid dependency id"));
}

#[test]
fn rejects_lockfile_v1_missing_package_dependency_reference_with_r013() {
    let tmp = tempfile::tempdir().expect("tempdir");
    write_baseline_runtime_artifacts(tmp.path(), "artifact");
    write_file(
        tmp.path().join(STRICT_LOCKFILE_FILE).as_path(),
        r#"{
  "schema_version": 1,
  "resolver_version": 1,
  "roots": [],
  "packages": [
    {
      "id": "pkg::a@1.0.0",
      "name": "pkg::a",
      "version": "1.0.0",
      "digest": "sha256:c7c5c1d70c5dec441d7d17042f8bbf5be4c4bf4a4c8f89f46b5f58211a711000",
      "abi_id": "abi:pkg::a:1.0.0",
      "dependencies": ["pkg::missing@1.0.0"]
    }
  ]
}"#,
    );

    let err = load_runtime_packages_from_local_store_if_present(
        tmp.path(),
        RuntimeLoaderConfig::default(),
    )
    .expect_err("expected missing package dependency reference");
    assert_eq!(err.code(), "R013");
    assert!(err.message().contains("missing from `packages[]`"));
}

#[test]
fn rejects_lockfile_v1_duplicate_package_dependency_id_with_r013() {
    let tmp = tempfile::tempdir().expect("tempdir");
    write_baseline_runtime_artifacts(tmp.path(), "artifact");
    write_file(
        tmp.path().join(STRICT_LOCKFILE_FILE).as_path(),
        r#"{
  "schema_version": 1,
  "resolver_version": 1,
  "roots": [],
  "packages": [
    {
      "id": "pkg::a@1.0.0",
      "name": "pkg::a",
      "version": "1.0.0",
      "digest": "sha256:c7c5c1d70c5dec441d7d17042f8bbf5be4c4bf4a4c8f89f46b5f58211a711000",
      "abi_id": "abi:pkg::a:1.0.0",
      "dependencies": ["pkg::a@1.0.0", "pkg::a@1.0.0"]
    }
  ]
}"#,
    );

    let err = load_runtime_packages_from_local_store_if_present(
        tmp.path(),
        RuntimeLoaderConfig::default(),
    )
    .expect_err("expected duplicate package dependency id");
    assert_eq!(err.code(), "R013");
    assert!(err.message().contains("duplicate dependency id"));
}

#[test]
fn rejects_lockfile_v1_root_requirement_without_matching_package_with_r013() {
    let tmp = tempfile::tempdir().expect("tempdir");
    write_baseline_runtime_artifacts(tmp.path(), "artifact");
    write_file(
        tmp.path().join(STRICT_LOCKFILE_FILE).as_path(),
        r#"{
  "schema_version": 1,
  "resolver_version": 1,
  "roots": [
    {
      "name": "app",
      "dependencies": [
        {
          "name": "pkg::a",
          "requirement": "^2.0.0"
        }
      ]
    }
  ],
  "packages": [
    {
      "id": "pkg::a@1.0.0",
      "name": "pkg::a",
      "version": "1.0.0",
      "digest": "sha256:c7c5c1d70c5dec441d7d17042f8bbf5be4c4bf4a4c8f89f46b5f58211a711000",
      "abi_id": "abi:pkg::a:1.0.0",
      "dependencies": []
    }
  ]
}"#,
    );

    let err = load_runtime_packages_from_local_store_if_present(
        tmp.path(),
        RuntimeLoaderConfig::default(),
    )
    .expect_err("expected unsatisfied root requirement");
    assert_eq!(err.code(), "R013");
    assert!(err.message().contains("is not satisfied by `packages[]`"));
}

#[test]
fn rejects_invalid_signature_with_r014() {
    let tmp = tempfile::tempdir().expect("tempdir");
    write_baseline_runtime_artifacts(tmp.path(), "artifact");
    write_file(
        tmp.path().join(STRICT_PACKAGE_SIGNATURES_FILE).as_path(),
        r#"{
  "schema_version": 0,
  "signatures": [
    {
      "name": "pkg::a",
      "version": "1.0.0",
      "digest": "sha256:c7c5c1d70c5dec441d7d17042f8bbf5be4c4bf4a4c8f89f46b5f58211a711000",
      "key_id": "k1",
      "signed_at": "2026-06-01T00:00:00Z",
      "signature_format": "ed25519",
      "signature": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
    }
  ]
}"#,
    );

    let err = load_runtime_packages_from_local_store_if_present(
        tmp.path(),
        RuntimeLoaderConfig::default(),
    )
    .expect_err("expected signature verification failure");
    assert_eq!(err.code(), "R014");
}

#[test]
fn rejects_missing_artifact_in_store_with_r012() {
    let tmp = tempfile::tempdir().expect("tempdir");
    write_baseline_runtime_artifacts(tmp.path(), "artifact");
    write_file(
        tmp.path().join(PACKAGE_STORE_INDEX_FILE).as_path(),
        r#"{"schema_version":0,"artifacts":[]}"#,
    );

    let err = load_runtime_packages_from_local_store_if_present(
        tmp.path(),
        RuntimeLoaderConfig::default(),
    )
    .expect_err("expected missing artifact");
    assert_eq!(err.code(), "R012");
}

#[test]
fn loads_from_mirror_when_primary_artifact_is_unavailable() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path();
    write_baseline_runtime_artifacts(root, "artifact");

    fs::remove_file(root.join("store").join("pkg-a.wasm")).expect("remove primary artifact");
    let mirror_store = root.join("mirror-a").join("store");
    fs::create_dir_all(&mirror_store).expect("create mirror store");
    write_file(mirror_store.join("pkg-a.wasm").as_path(), "artifact");
    write_file(
        root.join(RUNTIME_LOADER_POLICY_FILE).as_path(),
        r#"{
  "schema_version": 0,
  "offline_mode": true,
  "mirror_paths": ["mirror-a"],
  "artifact_read_retries": 2
}"#,
    );

    let loaded =
        load_runtime_packages_from_local_store_if_present(root, RuntimeLoaderConfig::default())
            .expect("load runtime packages")
            .expect("runtime-link present");
    assert_eq!(
        loaded.packages[0].resolved_path,
        root.join("mirror-a").join("store").join("pkg-a.wasm")
    );
}

#[test]
fn rejects_runtime_loader_policy_with_offline_mode_false() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path();
    write_baseline_runtime_artifacts(root, "artifact");
    write_file(
        root.join(RUNTIME_LOADER_POLICY_FILE).as_path(),
        r#"{
  "schema_version": 0,
  "offline_mode": false,
  "mirror_paths": [],
  "artifact_read_retries": 1
}"#,
    );

    let err =
        load_runtime_packages_from_local_store_if_present(root, RuntimeLoaderConfig::default())
            .expect_err("offline_mode=false should fail");
    assert_eq!(err.code(), "R012");
    assert!(err.message().contains("offline_mode=false"));
}

#[test]
fn rejects_unknown_binding_provider_package_with_r015() {
    let tmp = tempfile::tempdir().expect("tempdir");
    write_baseline_runtime_artifacts(tmp.path(), "artifact");
    let digest = format!("sha256:{}", sha256_hex("artifact".as_bytes()));
    let runtime_link = serde_json::json!({
        "schema_version": 0,
        "resolver_version": 1,
        "packages": [
            {
                "id": "pkg::a@1.0.0",
                "digest": digest,
                "artifact_path": "store/pkg-a.wasm",
                "abi_id": "abi:pkg::a:1.0.0"
            }
        ],
        "bindings": [
            {
                "import_module": "pkg::x",
                "import_name": "add",
                "provider_package_id": "pkg::x@1.0.0",
                "provider_symbol": "add"
            }
        ]
    });
    write_file(
        tmp.path().join(RUNTIME_LINK_FILE).as_path(),
        serde_json::to_string_pretty(&runtime_link)
            .expect("serialize runtime-link")
            .as_str(),
    );
    write_file(
        tmp.path().join(RUNTIME_LINK_HASH_FILE).as_path(),
        format!(
            "{}\n",
            sha256_hex(canonical_json_bytes(&runtime_link).as_slice())
        )
        .as_str(),
    );

    let err = load_runtime_packages_from_local_store_if_present(
        tmp.path(),
        RuntimeLoaderConfig::default(),
    )
    .expect_err("expected provider mismatch");
    assert_eq!(err.code(), "R015");
}
