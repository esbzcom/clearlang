use assert_cmd::prelude::*;
use ed25519_dalek::SigningKey;
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::tempdir;

fn write_key_material(dir: &Path) -> (PathBuf, PathBuf) {
    let signing = SigningKey::from_bytes(&[11u8; 32]);
    let verifying = signing.verifying_key();
    let private_key = hex::encode(signing.to_bytes());
    let public_key = hex::encode(verifying.to_bytes());

    let key_path = dir.join("key.json");
    let pub_path = dir.join("pub.json");

    let key_json = json!({
        "scheme": "ed25519",
        "private_key": private_key,
        "public_key": public_key,
    });
    fs::write(
        &key_path,
        serde_json::to_vec_pretty(&key_json).expect("serialize key"),
    )
    .expect("write key");

    let pub_json = json!({
        "scheme": "ed25519",
        "public_key": public_key,
    });
    fs::write(
        &pub_path,
        serde_json::to_vec_pretty(&pub_json).expect("serialize pubkey"),
    )
    .expect("write pubkey");

    (key_path, pub_path)
}

#[test]
fn milestone2_readiness_gate_exercises_signed_artifact_roundtrip() {
    let tmp = tempdir().expect("tempdir");
    let src_path = tmp.path().join("readiness_smoke.clear");
    let wasm_path = tmp.path().join("readiness.wasm");
    let vcs_path = tmp.path().join("readiness.vc.json");
    let sig_path = tmp.path().join("readiness.sig.json");
    let manifest_path = tmp.path().join("readiness.assurance.json");
    let (key_path, pub_path) = write_key_material(tmp.path());

    fs::write(
        &src_path,
        r#"
            pure function inc(x: Int) -> Int
                require { 0 <= x }
                ensure { result > x }
            { x + 1 }
            function main() -> Int { inc(1) }
        "#,
    )
    .expect("write source");

    let mut build = Command::cargo_bin("clg").expect("bin");
    build
        .current_dir(tmp.path())
        .args(["build"])
        .arg(&src_path)
        .args(["-o"])
        .arg(&wasm_path)
        .arg("--emit-vcs")
        .arg(&vcs_path)
        .arg("--sign")
        .arg("--key")
        .arg(&key_path)
        .arg("--key-id")
        .arg("milestone2-readiness")
        .arg("--scope")
        .arg("both")
        .arg("--sig-out")
        .arg(&sig_path)
        .arg("--assurance-manifest-out")
        .arg(&manifest_path);
    build.assert().success();

    assert!(wasm_path.exists(), "expected signed module wasm output");
    assert!(vcs_path.exists(), "expected vc output");
    assert!(sig_path.exists(), "expected signature output");
    assert!(manifest_path.exists(), "expected assurance manifest output");

    let sig_json: serde_json::Value =
        serde_json::from_slice(&fs::read(&sig_path).expect("read signature")).expect("json");
    assert_eq!(
        sig_json.get("key_id").and_then(|v| v.as_str()),
        Some("milestone2-readiness")
    );
    assert!(
        sig_json
            .get("signature")
            .and_then(|v| v.as_str())
            .map(|s| !s.is_empty())
            .unwrap_or(false),
        "signature payload should include a non-empty signature field"
    );
    assert!(
        sig_json
            .get("payload")
            .and_then(|v| v.get("module_hash"))
            .and_then(|v| v.as_str())
            .map(|s| !s.is_empty())
            .unwrap_or(false),
        "signature payload should include a non-empty module hash"
    );

    let mut verify = Command::cargo_bin("clg").expect("bin");
    verify
        .args(["verify"])
        .arg("--module")
        .arg(&wasm_path)
        .arg("--sig")
        .arg(&sig_path)
        .arg("--pubkey")
        .arg(&pub_path);
    verify.assert().success();
}
