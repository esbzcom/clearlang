#[cfg(feature = "rust-z3-lib")]
use assert_cmd::prelude::*;
#[cfg(feature = "rust-z3-lib")]
use ed25519_dalek::{Signer, SigningKey};
#[cfg(feature = "rust-z3-lib")]
use serde_json::Value;
#[cfg(feature = "rust-z3-lib")]
use sha2::{Digest, Sha256};
#[cfg(feature = "rust-z3-lib")]
use std::fs;
#[cfg(feature = "rust-z3-lib")]
use std::path::{Path, PathBuf};
#[cfg(feature = "rust-z3-lib")]
use std::process::Command;
#[cfg(feature = "rust-z3-lib")]
use tempfile::tempdir;

#[cfg(feature = "rust-z3-lib")]
const SAMPLE_SOURCE: &str = r#"
pure function inc(x: Int) -> Int
    require { x >= 0 }
    ensure { result > x }
{ x + 1 }
function main() -> Int { inc(1) }
"#;

#[cfg(feature = "rust-z3-lib")]
fn write_fake_solver_to(path: &Path) -> PathBuf {
    let source = path.with_extension("rs");
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create fake solver dir");
    }
    let solver = path.to_path_buf();
    let fake_solver_src = r#"
use std::io::{self, Read};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|arg| arg == "--version" || arg == "-version") {
        println!("Z3 version 4.16.0 - fake");
        return;
    }
    let mut stdin = Vec::new();
    let _ = io::stdin().read_to_end(&mut stdin);
    println!("unsat");
}
"#;
    fs::write(&source, fake_solver_src).expect("write fake solver source");
    let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_string());
    let output = Command::new(rustc)
        .arg(&source)
        .arg("-O")
        .arg("-o")
        .arg(&solver)
        .output()
        .expect("run rustc for fake solver");
    assert!(
        output.status.success(),
        "fake solver compile failed: {}",
        String::from_utf8_lossy(output.stderr.as_slice())
    );
    solver
}

#[cfg(feature = "rust-z3-lib")]
fn write_solver_integrity_sidecars(solver: &Path) {
    let solver_bytes = fs::read(solver).expect("read solver bytes");
    let checksum = format!("sha256:{}", hex::encode(Sha256::digest(&solver_bytes)));
    let checksum_path = solver.with_file_name(format!(
        "{}.sha256",
        solver
            .file_name()
            .expect("solver filename")
            .to_string_lossy()
    ));
    fs::write(&checksum_path, format!("{checksum}\n")).expect("write checksum sidecar");

    let signing = SigningKey::from_bytes(&[7u8; 32]);
    let signature = hex::encode(signing.sign(checksum.as_bytes()).to_bytes());
    let signature_path = solver.with_file_name(format!(
        "{}.sig",
        solver
            .file_name()
            .expect("solver filename")
            .to_string_lossy()
    ));
    fs::write(
        &signature_path,
        serde_json::to_vec_pretty(&serde_json::json!({
            "schema_version": 1,
            "key_id": "z3-vendor-k7-2026q2",
            "scheme": "ed25519",
            "signed_payload": checksum,
            "signature": signature
        }))
        .expect("serialize signature sidecar"),
    )
    .expect("write signature sidecar");
}

#[cfg(feature = "rust-z3-lib")]
fn vc_statuses(path: &Path) -> Vec<String> {
    let vcs_json: Value =
        serde_json::from_slice(&fs::read(path).expect("read vcs")).expect("parse vcs json");
    vcs_json
        .as_array()
        .expect("vcs array")
        .iter()
        .map(|vc| {
            vc.get("status")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string()
        })
        .collect()
}

#[cfg(feature = "rust-z3-lib")]
fn sha256_prefixed_hex(bytes: &[u8]) -> String {
    format!("sha256:{}", hex::encode(Sha256::digest(bytes)))
}

#[cfg(feature = "rust-z3-lib")]
#[test]
fn backend_parity_gate_keeps_vc_statuses_and_proof_hash_identical() {
    let tmp = tempdir().expect("tempdir");
    let source = tmp.path().join("main.clear");
    let external_wasm = tmp.path().join("external.wasm");
    let external_vcs = tmp.path().join("external.vc.json");
    let external_proof = tmp.path().join("external.proof.json");
    let rust_wasm = tmp.path().join("rust.wasm");
    let rust_vcs = tmp.path().join("rust.vc.json");
    let rust_proof = tmp.path().join("rust.proof.json");
    fs::write(&source, SAMPLE_SOURCE).expect("write source");

    let external_solver = if cfg!(windows) {
        tmp.path().join("fake-z3.exe")
    } else {
        tmp.path().join("fake-z3")
    };
    write_fake_solver_to(&external_solver);
    write_solver_integrity_sidecars(&external_solver);

    Command::cargo_bin("clg")
        .expect("cargo_bin clg")
        .env("CLG_SOLVER_BACKEND", "external-z3-cli")
        .env("CLG_SOLVER_BIN", &external_solver)
        .arg("build")
        .arg(&source)
        .arg("-o")
        .arg(&external_wasm)
        .arg("--emit-vcs")
        .arg(&external_vcs)
        .arg("--emit-proof")
        .arg(&external_proof)
        .assert()
        .success();

    Command::cargo_bin("clg")
        .expect("cargo_bin clg")
        .env("CLG_SOLVER_BACKEND", "rust-z3-lib")
        .env("CLG_SOLVER_RUST_Z3_CUTOVER", "1")
        .env_remove("CLG_SOLVER_BIN")
        .arg("build")
        .arg(&source)
        .arg("-o")
        .arg(&rust_wasm)
        .arg("--emit-vcs")
        .arg(&rust_vcs)
        .arg("--emit-proof")
        .arg(&rust_proof)
        .assert()
        .success();

    assert_eq!(
        vc_statuses(&external_vcs),
        vc_statuses(&rust_vcs),
        "external-z3-cli and rust-z3-lib backends must produce identical VC status vectors for identical strict inputs"
    );

    let external_proof_hash = sha256_prefixed_hex(&fs::read(&external_proof).expect("read proof"));
    let rust_proof_hash = sha256_prefixed_hex(&fs::read(&rust_proof).expect("read proof"));
    assert_eq!(
        external_proof_hash, rust_proof_hash,
        "external-z3-cli and rust-z3-lib backends must produce identical proof artifact hash for identical strict inputs"
    );
}
