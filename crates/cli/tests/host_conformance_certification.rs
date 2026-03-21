use assert_cmd::prelude::*;
use ed25519_dalek::{Signer, SigningKey};
use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::tempdir;

fn write_signed_strict_dependency_fixture(
    root: &Path,
    profile: &str,
    capabilities: &[&str],
    imports_json: &str,
) {
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

    let capabilities_json = capabilities
        .iter()
        .map(|capability| format!("\"{capability}\""))
        .collect::<Vec<_>>()
        .join(", ");
    fs::write(
        root.join("clg.host-profile.json"),
        format!(
            r#"{{
  "schema_version": 0,
  "profile": "{profile}",
  "capabilities": [{capabilities_json}]
}}"#
        ),
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

fn json_error(output: &[u8]) -> Value {
    let v: Value = serde_json::from_slice(output).expect("json");
    assert!(!v.get("ok").and_then(|b| b.as_bool()).unwrap_or(true));
    let errs = v.get("errors").and_then(|e| e.as_array()).expect("errors");
    assert_eq!(errs.len(), 1, "expected a single deterministic diagnostic");
    errs[0].clone()
}

#[test]
fn host_conformance_strict_profile_matrix_enforces_deterministic_env_policy() {
    for profile in ["contract_static", "shared_app"] {
        let tmp = tempdir().expect("tempdir");
        let src_path = tmp.path().join("strict_env_capability.clear");
        let wasm_path = tmp.path().join("out.wasm");
        let vcs_path = tmp.path().join("out.vc.json");
        let src = r#"
            function main() -> Int { std::str::len("abc") }
        "#;
        fs::write(&src_path, src.trim()).expect("write source");
        write_signed_strict_dependency_fixture(
            tmp.path(),
            profile,
            &["std::env::time"],
            r#"[{
  "symbol": "std::str::len",
  "effect": "pure",
  "params": ["String"],
  "ret": "Int",
  "capability": "std::env::time"
}]"#,
        );

        let output = Command::cargo_bin("clg")
            .unwrap()
            .args(["--json-errors", "build"])
            .arg(&src_path)
            .args(["-o"])
            .arg(&wasm_path)
            .args(["--emit-vcs"])
            .arg(&vcs_path)
            .args(["--compiler-mode", "strict"])
            .args(["--std-core-link-mode", "precompiled"])
            .assert()
            .failure()
            .get_output()
            .stdout
            .clone();
        let err = json_error(&output);
        assert_eq!(err.get("code").and_then(|s| s.as_str()), Some("C106"));
        assert_eq!(err.get("stage").and_then(|s| s.as_str()), Some("build"));
        assert!(
            err.get("message")
                .and_then(|s| s.as_str())
                .unwrap_or("")
                .contains("denied by strict deterministic policy"),
            "expected strict deterministic env deny policy for profile `{profile}`"
        );
    }
}

#[test]
fn host_conformance_strict_profile_matrix_allows_crypto_capability() {
    for profile in ["contract_static", "shared_app"] {
        let tmp = tempdir().expect("tempdir");
        let src_path = tmp.path().join("strict_crypto_capability.clear");
        let wasm_path = tmp.path().join("out.wasm");
        let vcs_path = tmp.path().join("out.vc.json");
        let src = r#"
            function main() -> Int { std::str::len("abc") }
        "#;
        fs::write(&src_path, src.trim()).expect("write source");
        write_signed_strict_dependency_fixture(
            tmp.path(),
            profile,
            &["std::crypto::hash"],
            r#"[{
  "symbol": "std::str::len",
  "effect": "pure",
  "params": ["String"],
  "ret": "Int",
  "capability": "std::crypto::hash"
}]"#,
        );

        Command::cargo_bin("clg")
            .unwrap()
            .args(["build"])
            .arg(&src_path)
            .args(["-o"])
            .arg(&wasm_path)
            .args(["--emit-vcs"])
            .arg(&vcs_path)
            .args(["--compiler-mode", "strict"])
            .args(["--std-core-link-mode", "precompiled"])
            .assert()
            .success();
    }
}

#[test]
fn host_conformance_runtime_production_profiles_require_runtime_link() {
    for profile in ["contract_static", "shared_app"] {
        let tmp = tempdir().expect("tempdir");
        let src_path = tmp.path().join("main.clear");
        let wasm_path = tmp.path().join("out.wasm");
        let src = r#"
            function main() -> Int { 0 }
        "#;
        fs::write(&src_path, src.trim()).expect("write source");
        fs::write(
            tmp.path().join("clg.host-profile.json"),
            format!(
                r#"{{
  "schema_version": 0,
  "profile": "{profile}",
  "capabilities": []
}}"#
            ),
        )
        .expect("write host profile");

        Command::cargo_bin("clg")
            .unwrap()
            .args(["build"])
            .arg(&src_path)
            .args(["-o"])
            .arg(&wasm_path)
            .assert()
            .success();

        let output = Command::cargo_bin("clg")
            .unwrap()
            .args(["--json-errors", "run"])
            .arg(&wasm_path)
            .assert()
            .failure()
            .get_output()
            .stdout
            .clone();
        let err = json_error(&output);
        assert_eq!(err.get("code").and_then(|s| s.as_str()), Some("R012"));
        assert_eq!(err.get("stage").and_then(|s| s.as_str()), Some("runtime"));
    }
}

#[test]
fn host_conformance_runtime_rejects_unsupported_profile() {
    let tmp = tempdir().expect("tempdir");
    let src_path = tmp.path().join("main.clear");
    let wasm_path = tmp.path().join("out.wasm");
    fs::write(&src_path, "function main() -> Int { 0 }").expect("write source");
    fs::write(
        tmp.path().join("clg.host-profile.json"),
        r#"{
  "schema_version": 0,
  "profile": "dev_local",
  "capabilities": []
}"#,
    )
    .expect("write unsupported profile");

    Command::cargo_bin("clg")
        .unwrap()
        .args(["build"])
        .arg(&src_path)
        .args(["-o"])
        .arg(&wasm_path)
        .assert()
        .success();

    let output = Command::cargo_bin("clg")
        .unwrap()
        .args(["--json-errors", "run"])
        .arg(&wasm_path)
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let err = json_error(&output);
    assert_eq!(err.get("code").and_then(|s| s.as_str()), Some("R016"));
    assert_eq!(err.get("stage").and_then(|s| s.as_str()), Some("runtime"));
}
