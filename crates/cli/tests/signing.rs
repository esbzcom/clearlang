use assert_cmd::prelude::*;
use ed25519_dalek::SigningKey;
use predicates::prelude::predicate;
use predicates::prelude::PredicateBooleanExt;
use serde_json::json;
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
    let src_path = tmp.join("contract.clear");
    fs::write(&src_path, sample_source()).unwrap();

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
    cmd.assert().success();

    (wasm_path, sig_path, pub_path)
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
