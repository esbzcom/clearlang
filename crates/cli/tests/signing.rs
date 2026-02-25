use assert_cmd::prelude::*;
use ed25519_dalek::SigningKey;
use predicates::prelude::predicate;
use predicates::prelude::PredicateBooleanExt;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::tempdir;
use wasmparser::{Parser, Payload};

fn sample_source() -> &'static str {
    r#"
        pure function inc(x: Int) -> Int
            require { 0 <= x }
            ensure { result > x }
        { x + 1 }
        function main() -> Int { inc(1) }
    "#
}

fn sample_source_with_assumptions() -> &'static str {
    r#"
        pure function check(a: Bytes, b: Bytes) -> Bool
            ensure { result == std::bytes::eq_ct(a, b) }
        { std::bytes::eq_ct(a, b) }
        function main() -> Int { if check(std::bytes::from_string("a"), std::bytes::from_string("b")) { 1 } else { 0 } }
    "#
}

fn write_key_material(dir: &Path) -> (PathBuf, PathBuf) {
    let signing = SigningKey::from_bytes(&[7u8; 32]);
    let verifying = signing.verifying_key();
    let priv_hex = hex::encode(signing.to_bytes());
    let pub_hex = hex::encode(verifying.to_bytes());

    let key_path = dir.join("key.json");
    let pub_path = dir.join("pub.json");

    let key_json = json!({
        "scheme": "ed25519",
        "private_key": priv_hex,
        "public_key": pub_hex,
    });
    fs::write(&key_path, serde_json::to_vec_pretty(&key_json).unwrap()).unwrap();

    let pub_json = json!({
        "scheme": "ed25519",
        "public_key": pub_hex,
    });
    fs::write(&pub_path, serde_json::to_vec_pretty(&pub_json).unwrap()).unwrap();

    (key_path, pub_path)
}

fn build_signed_module(tmp: &Path) -> (PathBuf, PathBuf, PathBuf) {
    build_signed_module_from_source_with_manifest_and_trust_anchors(
        tmp,
        sample_source(),
        None,
        None,
        None,
    )
}

fn build_signed_module_with_trust_anchors(
    tmp: &Path,
    lean_checker_version: Option<&str>,
    coq_checker_version: Option<&str>,
) -> (PathBuf, PathBuf, PathBuf) {
    build_signed_module_from_source_with_manifest_and_trust_anchors(
        tmp,
        sample_source(),
        None,
        lean_checker_version,
        coq_checker_version,
    )
}

fn build_signed_module_with_manifest_and_trust_anchors(
    tmp: &Path,
    manifest_path: Option<&Path>,
    lean_checker_version: Option<&str>,
    coq_checker_version: Option<&str>,
) -> (PathBuf, PathBuf, PathBuf) {
    build_signed_module_from_source_with_manifest_and_trust_anchors(
        tmp,
        sample_source(),
        manifest_path,
        lean_checker_version,
        coq_checker_version,
    )
}

fn build_signed_module_from_source_with_manifest_and_trust_anchors(
    tmp: &Path,
    source: &str,
    manifest_path: Option<&Path>,
    lean_checker_version: Option<&str>,
    coq_checker_version: Option<&str>,
) -> (PathBuf, PathBuf, PathBuf) {
    let src_path = tmp.join("contract.clear");
    fs::write(&src_path, source).unwrap();

    let wasm_path = tmp.join("out.wasm");
    let vcs_path = tmp.join("out.vc.json");
    let sig_path = tmp.join("out.sig.json");
    let (key_path, pub_path) = write_key_material(tmp);

    let mut cmd = Command::cargo_bin("clg").expect("bin");
    cmd.current_dir(tmp);
    cmd.args(["build"])
        .arg(&src_path)
        .args(["-o"])
        .arg(&wasm_path)
        .arg("--emit-vcs")
        .arg(&vcs_path)
        .arg("--sign")
        .arg("--key")
        .arg(&key_path)
        .arg("--key-id")
        .arg("test-key")
        .arg("--scope")
        .arg("both")
        .arg("--sig-out")
        .arg(&sig_path);
    if let Some(manifest_path) = manifest_path {
        cmd.arg("--assurance-manifest-out").arg(manifest_path);
    }
    if let (Some(lean), Some(coq)) = (lean_checker_version, coq_checker_version) {
        cmd.arg("--lean-checker-version")
            .arg(lean)
            .arg("--coq-checker-version")
            .arg(coq);
    }
    cmd.assert().success();

    (wasm_path, sig_path, pub_path)
}

fn canonicalize_value(value: &serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => {
            let mut keys: Vec<_> = map.keys().collect();
            keys.sort();
            let mut out = serde_json::Map::new();
            for key in keys {
                out.insert(key.clone(), canonicalize_value(&map[key]));
            }
            serde_json::Value::Object(out)
        }
        serde_json::Value::Array(items) => {
            serde_json::Value::Array(items.iter().map(canonicalize_value).collect())
        }
        _ => value.clone(),
    }
}

fn canonical_json_string(value: &serde_json::Value) -> String {
    serde_json::to_string(&canonicalize_value(value)).expect("serialize canonical json")
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

fn write_trust_policy(dir: &Path, lean_checker: &str, coq_checker: &str) -> PathBuf {
    let path = dir.join("trust-policy.json");
    let policy = json!({
        "schema_version": 1,
        "trust_anchors": {
            "lean_checker": lean_checker,
            "coq_checker": coq_checker,
        }
    });
    fs::write(&path, serde_json::to_vec_pretty(&policy).unwrap()).unwrap();
    path
}

fn write_release_policy(dir: &Path, minimum_assurance_tier: &str) -> PathBuf {
    let path = dir.join("release-policy.json");
    let policy = json!({
        "schema_version": 1,
        "minimum_assurance_tier": minimum_assurance_tier,
    });
    fs::write(&path, serde_json::to_vec_pretty(&policy).unwrap()).unwrap();
    path
}

fn tamper_proofs_hash(module: &Path) {
    #[derive(serde::Serialize, serde::Deserialize)]
    struct TamperSection {
        version: u32,
        generated_by: String,
        #[serde(with = "serde_bytes")]
        module_hash: Vec<u8>,
        #[serde(with = "serde_bytes")]
        proofs_hash: Vec<u8>,
        functions: serde_cbor::Value,
        assurance: Option<serde_cbor::Value>,
    }

    fn to_cbor_bytes<T: serde::Serialize>(value: &T) -> Vec<u8> {
        let mut buf = Vec::new();
        let mut ser = serde_cbor::ser::Serializer::new(&mut buf);
        value.serialize(&mut ser).unwrap();
        buf
    }

    let mut module_bytes = fs::read(module).expect("read wasm");
    let mut target: Option<(std::ops::Range<usize>, Vec<u8>)> = None;
    for payload in Parser::new(0).parse_all(&module_bytes) {
        let payload = payload.expect("payload");
        if let Payload::CustomSection(section) = payload {
            if section.name() == "clearlang.proof" {
                let range = section.range();
                target = Some((range, section.data().to_vec()));
                break;
            }
        }
    }
    let (range, data) = target.expect("proof section not found");
    let mut proof: TamperSection = serde_cbor::from_slice(&data).expect("decode proof");
    assert_eq!(proof.proofs_hash.len(), 32);
    proof.proofs_hash[0] ^= 0xFF;
    let new_bytes = to_cbor_bytes(&proof);
    assert_eq!(new_bytes.len(), data.len(), "proof section length changed");
    let data_start = range.end - data.len();
    module_bytes[data_start..range.end].copy_from_slice(&new_bytes);
    fs::write(module, module_bytes).expect("write tampered module");
}

fn tamper_signature(sig_path: &Path) {
    let sig_bytes = fs::read(sig_path).expect("read signature");
    let mut sig_value: serde_json::Value =
        serde_json::from_slice(&sig_bytes).expect("decode signature json");
    let sig = sig_value
        .get_mut("signature")
        .and_then(|value| value.as_str())
        .expect("signature field");
    let mut bytes = sig.as_bytes().to_vec();
    if let Some(first) = bytes.first_mut() {
        *first = if *first == b'0' { b'1' } else { b'0' };
    }
    let updated = String::from_utf8(bytes).expect("valid signature string");
    sig_value["signature"] = serde_json::Value::String(updated);
    fs::write(sig_path, serde_json::to_vec_pretty(&sig_value).unwrap()).expect("write signature");
}

fn strip_proof_section(module: &Path) {
    fn read_leb_u32(bytes: &[u8]) -> Option<(u32, usize)> {
        let mut value: u32 = 0;
        let mut shift = 0;
        for (idx, byte) in bytes.iter().enumerate() {
            let low = (byte & 0x7f) as u32;
            value |= low << shift;
            if byte & 0x80 == 0 {
                return Some((value, idx + 1));
            }
            shift += 7;
            if shift >= 32 {
                return None;
            }
        }
        None
    }

    let mut module_bytes = fs::read(module).expect("read wasm");
    let mut target: Option<std::ops::Range<usize>> = None;
    for payload in Parser::new(0).parse_all(&module_bytes) {
        let payload = payload.expect("payload");
        if let Payload::CustomSection(section) = payload {
            if section.name() == "clearlang.proof" {
                target = Some(section.range());
                break;
            }
        }
    }
    let range = target.expect("proof section not found");
    let payload = &module_bytes[range.start..range.end];
    let (name_len, name_len_bytes) = read_leb_u32(payload).expect("name length");
    let name_start = range.start + name_len_bytes;
    let name_end = name_start + name_len as usize;
    let name_bytes = &mut module_bytes[name_start..name_end];
    if let Some(last) = name_bytes.last_mut() {
        *last = if *last == b'X' { b'Y' } else { b'X' };
    }
    fs::write(module, module_bytes).expect("write stripped module");
}

#[test]
fn sign_and_verify_roundtrip() {
    let tmp = tempdir().unwrap();
    let (wasm_path, sig_path, pub_path) = build_signed_module(tmp.path());
    let sig_value: serde_json::Value =
        serde_json::from_slice(&fs::read(&sig_path).expect("read signature")).expect("json");
    let assurance = sig_value
        .get("payload")
        .and_then(|v| v.get("assurance"))
        .and_then(|v| v.as_object())
        .expect("payload assurance");
    assert_eq!(assurance.get("tier").and_then(|v| v.as_str()), Some("L1"));
    assert_eq!(
        assurance.get("label").and_then(|v| v.as_str()),
        Some("checked core")
    );
    assert_eq!(
        assurance
            .get("levels")
            .and_then(|v| v.get("L2"))
            .and_then(|v| v.as_str()),
        Some("verified module")
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

#[test]
fn verify_explain_emits_checked_core_summary() {
    let tmp = tempdir().unwrap();
    let (wasm_path, sig_path, pub_path) = build_signed_module(tmp.path());

    let mut verify = Command::cargo_bin("clg").expect("bin");
    verify
        .args(["verify", "--explain"])
        .arg("--module")
        .arg(&wasm_path)
        .arg("--sig")
        .arg(&sig_path)
        .arg("--pubkey")
        .arg(&pub_path);
    let stdout = verify.assert().success().get_output().stdout.clone();
    let text = String::from_utf8(stdout).expect("utf8 stdout");
    assert!(text.contains("Verification explanation"));
    assert!(text.contains("assurance: L1 (checked core)"));
    assert!(text.contains("assumed_boundaries: none"));
}

#[test]
fn verify_explain_includes_assumed_boundary_reasoning() {
    let tmp = tempdir().unwrap();
    let (wasm_path, sig_path, pub_path) =
        build_signed_module_from_source_with_manifest_and_trust_anchors(
            tmp.path(),
            sample_source_with_assumptions(),
            None,
            None,
            None,
        );

    let mut verify = Command::cargo_bin("clg").expect("bin");
    verify
        .args(["verify", "--explain"])
        .arg("--module")
        .arg(&wasm_path)
        .arg("--sig")
        .arg(&sig_path)
        .arg("--pubkey")
        .arg(&pub_path);
    let stdout = verify.assert().success().get_output().stdout.clone();
    let text = String::from_utf8(stdout).expect("utf8 stdout");
    assert!(text.contains("vcs_with_assumptions:"));
    assert!(text.contains("assumed_boundaries:"));
    assert!(text.contains("crypto.uninterpreted"));
    assert!(text.contains("why="));
}

#[test]
fn build_emits_signed_assurance_manifest() {
    let tmp = tempdir().unwrap();
    let manifest = tmp.path().join("out.assurance.json");
    let (_wasm_path, _sig_path, pub_path) = build_signed_module_with_manifest_and_trust_anchors(
        tmp.path(),
        Some(&manifest),
        None,
        None,
    );

    let manifest_value: serde_json::Value =
        serde_json::from_slice(&fs::read(&manifest).expect("read manifest")).expect("json");
    assert_eq!(
        manifest_value
            .get("schema_version")
            .and_then(|v| v.as_u64()),
        Some(1)
    );

    let payload = manifest_value.get("payload").expect("payload");
    assert_eq!(
        payload
            .get("format")
            .and_then(|v| v.as_str())
            .expect("format"),
        "clg.assurance_manifest.v1"
    );
    assert_eq!(
        payload
            .get("assurance")
            .and_then(|v| v.get("tier"))
            .and_then(|v| v.as_str()),
        Some("L1")
    );
    assert!(payload
        .get("assumptions")
        .and_then(|v| v.get("items"))
        .and_then(|v| v.as_array())
        .is_some());
    assert!(payload
        .get("dependency_trust_labels")
        .and_then(|v| v.as_array())
        .is_some());
    assert!(payload
        .get("toolchain")
        .and_then(|v| v.get("fingerprint_sha256"))
        .and_then(|v| v.as_str())
        .is_some());

    let payload_hash = manifest_value
        .get("signature")
        .and_then(|v| v.get("payload_hash"))
        .and_then(|v| v.as_str())
        .expect("payload_hash");
    let canonical_payload = canonical_json_string(payload);
    assert_eq!(payload_hash, sha256_hex(canonical_payload.as_bytes()));

    let signature_hex = manifest_value
        .get("signature")
        .and_then(|v| v.get("signature"))
        .and_then(|v| v.as_str())
        .expect("signature hex");
    let signature_bytes = hex::decode(signature_hex).expect("decode signature");
    let signature = ed25519_dalek::Signature::from_slice(&signature_bytes).expect("signature");

    let pub_json: serde_json::Value =
        serde_json::from_slice(&fs::read(pub_path).expect("read pubkey")).expect("pubkey json");
    let pub_hex = pub_json
        .get("public_key")
        .and_then(|v| v.as_str())
        .expect("public key");
    let pub_key_bytes: [u8; 32] = hex::decode(pub_hex)
        .expect("decode public key")
        .try_into()
        .expect("public key length");
    let verifying = ed25519_dalek::VerifyingKey::from_bytes(&pub_key_bytes).expect("verify key");
    verifying
        .verify_strict(canonical_payload.as_bytes(), &signature)
        .expect("manifest signature should verify");
}

#[test]
fn build_sign_defaults_assurance_manifest_path_from_sig_out() {
    let tmp = tempdir().unwrap();
    let (_wasm_path, sig_path, _pub_path) = build_signed_module(tmp.path());
    let manifest_path = sig_path.with_file_name("out.assurance.json");
    assert!(
        manifest_path.exists(),
        "expected default assurance manifest at {}",
        manifest_path.display()
    );
}

#[test]
fn verify_fails_when_proofs_hash_tampered() {
    let tmp = tempdir().unwrap();
    let (wasm_path, sig_path, pub_path) = build_signed_module(tmp.path());
    tamper_proofs_hash(&wasm_path);

    let mut verify = Command::cargo_bin("clg").expect("bin");
    verify
        .args(["--json-errors", "verify"])
        .arg("--module")
        .arg(&wasm_path)
        .arg("--sig")
        .arg(&sig_path)
        .arg("--pubkey")
        .arg(&pub_path);
    verify.assert().failure().stdout(
        predicate::str::contains("\"code\": \"V003\"")
            .and(predicate::str::contains("hash mismatch")),
    );
}

#[test]
fn verify_fails_when_proof_section_missing() {
    let tmp = tempdir().unwrap();
    let (wasm_path, sig_path, pub_path) = build_signed_module(tmp.path());
    strip_proof_section(&wasm_path);

    let mut verify = Command::cargo_bin("clg").expect("bin");
    verify
        .args(["--json-errors", "verify"])
        .arg("--module")
        .arg(&wasm_path)
        .arg("--sig")
        .arg(&sig_path)
        .arg("--pubkey")
        .arg(&pub_path);
    verify.assert().failure().stdout(
        predicate::str::contains("\"code\": \"V002\"")
            .and(predicate::str::contains("proof section not found")),
    );
}

#[test]
fn verify_fails_when_signature_is_invalid() {
    let tmp = tempdir().unwrap();
    let (wasm_path, sig_path, pub_path) = build_signed_module(tmp.path());
    tamper_signature(&sig_path);

    let mut verify = Command::cargo_bin("clg").expect("bin");
    verify
        .args(["--json-errors", "verify"])
        .arg("--module")
        .arg(&wasm_path)
        .arg("--sig")
        .arg(&sig_path)
        .arg("--pubkey")
        .arg(&pub_path);
    verify.assert().failure().stdout(
        predicate::str::contains("\"code\": \"V001\"")
            .and(predicate::str::contains("signature verification failed")),
    );
}

#[test]
fn verify_compile_time_requires_trust_policy() {
    let tmp = tempdir().unwrap();
    let (wasm_path, sig_path, pub_path) = build_signed_module(tmp.path());

    let mut verify = Command::cargo_bin("clg").expect("bin");
    verify
        .args(["--json-errors", "verify"])
        .arg("--module")
        .arg(&wasm_path)
        .arg("--sig")
        .arg(&sig_path)
        .arg("--pubkey")
        .arg(&pub_path)
        .args(["--verify-mode", "compile-time"]);
    verify.assert().failure().stdout(
        predicate::str::contains("\"code\": \"V004\"")
            .and(predicate::str::contains("requires `--trust-policy <FILE>`")),
    );
}

#[test]
fn verify_runtime_rejects_trust_policy() {
    let tmp = tempdir().unwrap();
    let (wasm_path, sig_path, pub_path) = build_signed_module(tmp.path());
    let trust_policy = write_trust_policy(tmp.path(), "4.14.0", "8.19.2");

    let mut verify = Command::cargo_bin("clg").expect("bin");
    verify
        .args(["--json-errors", "verify"])
        .arg("--module")
        .arg(&wasm_path)
        .arg("--sig")
        .arg(&sig_path)
        .arg("--pubkey")
        .arg(&pub_path)
        .arg("--trust-policy")
        .arg(&trust_policy);
    verify
        .assert()
        .failure()
        .stdout(
            predicate::str::contains("\"code\": \"V004\"").and(predicate::str::contains(
                "requires `--verify-mode compile-time`",
            )),
        );
}

#[test]
fn verify_compile_time_fails_when_signature_missing_trust_anchors() {
    let tmp = tempdir().unwrap();
    let (wasm_path, sig_path, pub_path) = build_signed_module(tmp.path());
    let trust_policy = write_trust_policy(tmp.path(), "4.14.0", "8.19.2");

    let mut verify = Command::cargo_bin("clg").expect("bin");
    verify
        .args(["--json-errors", "verify"])
        .arg("--module")
        .arg(&wasm_path)
        .arg("--sig")
        .arg(&sig_path)
        .arg("--pubkey")
        .arg(&pub_path)
        .args(["--verify-mode", "compile-time"])
        .arg("--trust-policy")
        .arg(&trust_policy);
    verify
        .assert()
        .failure()
        .stdout(
            predicate::str::contains("\"code\": \"V004\"").and(predicate::str::contains(
                "signature payload missing trust_anchors",
            )),
        );
}

#[test]
fn verify_compile_time_fails_when_trust_policy_mismatches_signature() {
    let tmp = tempdir().unwrap();
    let (wasm_path, sig_path, pub_path) =
        build_signed_module_with_trust_anchors(tmp.path(), Some("4.14.0"), Some("8.19.2"));
    let trust_policy = write_trust_policy(tmp.path(), "4.15.0", "8.19.2");

    let mut verify = Command::cargo_bin("clg").expect("bin");
    verify
        .args(["--json-errors", "verify"])
        .arg("--module")
        .arg(&wasm_path)
        .arg("--sig")
        .arg(&sig_path)
        .arg("--pubkey")
        .arg(&pub_path)
        .args(["--verify-mode", "compile-time"])
        .arg("--trust-policy")
        .arg(&trust_policy);
    verify
        .assert()
        .failure()
        .stdout(
            predicate::str::contains("\"code\": \"V004\"").and(predicate::str::contains(
                "trust-anchor checker version mismatch",
            )),
        );
}

#[test]
fn verify_compile_time_succeeds_when_trust_policy_matches_signature() {
    let tmp = tempdir().unwrap();
    let (wasm_path, sig_path, pub_path) =
        build_signed_module_with_trust_anchors(tmp.path(), Some("4.14.0"), Some("8.19.2"));
    let trust_policy = write_trust_policy(tmp.path(), "4.14.0", "8.19.2");

    let mut verify = Command::cargo_bin("clg").expect("bin");
    verify
        .args(["verify"])
        .arg("--module")
        .arg(&wasm_path)
        .arg("--sig")
        .arg(&sig_path)
        .arg("--pubkey")
        .arg(&pub_path)
        .args(["--verify-mode", "compile-time"])
        .arg("--trust-policy")
        .arg(&trust_policy);
    verify.assert().success();
}

#[test]
fn verify_release_policy_succeeds_when_manifest_meets_required_tier() {
    let tmp = tempdir().unwrap();
    let (wasm_path, sig_path, pub_path) = build_signed_module(tmp.path());
    let manifest = sig_path.with_file_name("out.assurance.json");
    let policy = write_release_policy(tmp.path(), "L1");

    let mut verify = Command::cargo_bin("clg").expect("bin");
    verify
        .args(["verify"])
        .arg("--module")
        .arg(&wasm_path)
        .arg("--sig")
        .arg(&sig_path)
        .arg("--pubkey")
        .arg(&pub_path)
        .arg("--assurance-manifest")
        .arg(&manifest)
        .arg("--release-policy")
        .arg(&policy);
    verify.assert().success();
}

#[test]
fn verify_release_policy_rejects_manifest_below_required_tier_with_v005() {
    let tmp = tempdir().unwrap();
    let (wasm_path, sig_path, pub_path) = build_signed_module(tmp.path());
    let manifest = sig_path.with_file_name("out.assurance.json");
    let policy = write_release_policy(tmp.path(), "L2");

    let mut verify = Command::cargo_bin("clg").expect("bin");
    verify
        .args(["--json-errors", "verify"])
        .arg("--module")
        .arg(&wasm_path)
        .arg("--sig")
        .arg(&sig_path)
        .arg("--pubkey")
        .arg(&pub_path)
        .arg("--assurance-manifest")
        .arg(&manifest)
        .arg("--release-policy")
        .arg(&policy);
    verify.assert().failure().stdout(
        predicate::str::contains("\"code\": \"V005\"")
            .and(predicate::str::contains("below required")),
    );
}

#[test]
fn verify_release_policy_rejects_manifest_hash_mismatch_with_v005() {
    let tmp_a = tempdir().unwrap();
    let (wasm_path, sig_path, pub_path) = build_signed_module(tmp_a.path());
    let policy = write_release_policy(tmp_a.path(), "L0");

    let tmp_b = tempdir().unwrap();
    let (_other_wasm, other_sig, _other_pub) =
        build_signed_module_from_source_with_manifest_and_trust_anchors(
            tmp_b.path(),
            sample_source_with_assumptions(),
            None,
            None,
            None,
        );
    let mismatched_manifest = other_sig.with_file_name("out.assurance.json");

    let mut verify = Command::cargo_bin("clg").expect("bin");
    verify
        .args(["--json-errors", "verify"])
        .arg("--module")
        .arg(&wasm_path)
        .arg("--sig")
        .arg(&sig_path)
        .arg("--pubkey")
        .arg(&pub_path)
        .arg("--assurance-manifest")
        .arg(&mismatched_manifest)
        .arg("--release-policy")
        .arg(&policy);
    verify
        .assert()
        .failure()
        .stdout(
            predicate::str::contains("\"code\": \"V005\"").and(predicate::str::contains(
                "does not match verified signature payload hashes",
            )),
        );
}
