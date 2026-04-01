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
const PROVED_SOURCE: &str = r#"
function main() -> Int
    ensure { result > 0 }
{ 1 }
"#;

#[cfg(feature = "rust-z3-lib")]
const FAILED_SOURCE: &str = r#"
function main() -> Int
    ensure { result < 0 }
{ 1 }
"#;

#[cfg(feature = "rust-z3-lib")]
struct ParityFixture {
    name: &'static str,
    source: &'static str,
    external_solver_result: &'static str,
    expected_status: &'static str,
}

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
        let version = std::env::var("CLG_FAKE_Z3_VERSION").unwrap_or_else(|_| "4.16.0".to_string());
        println!("Z3 version {} - fake", version);
        return;
    }
    let mut stdin = Vec::new();
    let _ = io::stdin().read_to_end(&mut stdin);
    match std::env::var("CLG_FAKE_Z3_RESULT").as_deref() {
        Ok("sat") => println!("sat"),
        Ok("unknown") => {
            println!("unknown");
            println!("(:reason-unknown \"incomplete\")");
        }
        Ok("timeout") => {
            println!("unknown");
            println!("(:reason-unknown \"timeout\")");
        }
        _ => println!("unsat"),
    }
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
fn run_build_with_backend(
    source: &Path,
    wasm: &Path,
    vcs: &Path,
    proof: &Path,
    configure: impl FnOnce(&mut Command),
) {
    let mut command = Command::cargo_bin("clg").expect("cargo_bin clg");
    configure(&mut command);
    command
        .arg("build")
        .arg(source)
        .arg("-o")
        .arg(wasm)
        .arg("--emit-vcs")
        .arg(vcs)
        .arg("--emit-proof")
        .arg(proof)
        .assert()
        .success();
}

#[cfg(feature = "rust-z3-lib")]
#[test]
fn backend_parity_gate_keeps_status_vectors_and_proof_hashes_identical_across_fixture_matrix() {
    let tmp = tempdir().expect("tempdir");
    let external_solver = if cfg!(windows) {
        tmp.path().join("fake-z3.exe")
    } else {
        tmp.path().join("fake-z3")
    };
    write_fake_solver_to(&external_solver);
    write_solver_integrity_sidecars(&external_solver);

    let fixtures = [
        ParityFixture {
            name: "proved",
            source: PROVED_SOURCE,
            external_solver_result: "unsat",
            expected_status: "proved",
        },
        ParityFixture {
            name: "failed",
            source: FAILED_SOURCE,
            external_solver_result: "sat",
            expected_status: "failed",
        },
    ];

    for fixture in fixtures {
        let source = tmp.path().join(format!("{}-main.clear", fixture.name));
        fs::write(&source, fixture.source).expect("write source");

        let external_wasm = tmp.path().join(format!("{}-external.wasm", fixture.name));
        let external_vcs = tmp
            .path()
            .join(format!("{}-external.vc.json", fixture.name));
        let external_proof = tmp
            .path()
            .join(format!("{}-external.proof.json", fixture.name));

        run_build_with_backend(
            &source,
            &external_wasm,
            &external_vcs,
            &external_proof,
            |command| {
                command
                    .env("CLG_SOLVER_BACKEND", "external-z3-cli")
                    .env("CLG_SOLVER_BIN", &external_solver)
                    .env("CLG_FAKE_Z3_RESULT", fixture.external_solver_result);
            },
        );

        let rust_wasm = tmp.path().join(format!("{}-rust.wasm", fixture.name));
        let rust_vcs = tmp.path().join(format!("{}-rust.vc.json", fixture.name));
        let rust_proof = tmp.path().join(format!("{}-rust.proof.json", fixture.name));

        run_build_with_backend(&source, &rust_wasm, &rust_vcs, &rust_proof, |command| {
            command
                .env("CLG_SOLVER_BACKEND", "rust-z3-lib")
                .env("CLG_SOLVER_RUST_Z3_CUTOVER", "1")
                .env_remove("CLG_SOLVER_BIN")
                .env_remove("CLG_FAKE_Z3_RESULT");
        });

        let external_statuses = vc_statuses(&external_vcs);
        let rust_statuses = vc_statuses(&rust_vcs);
        assert!(
            external_statuses
                .iter()
                .all(|status| status == fixture.expected_status),
            "external fixture `{}` should produce only `{}` VC statuses, got {:?}",
            fixture.name,
            fixture.expected_status,
            external_statuses
        );
        assert_eq!(
            external_statuses, rust_statuses,
            "external-z3-cli and rust-z3-lib backends must produce identical VC status vectors for fixture `{}`",
            fixture.name
        );

        let external_proof_hash =
            sha256_prefixed_hex(&fs::read(&external_proof).expect("read proof"));
        let rust_proof_hash = sha256_prefixed_hex(&fs::read(&rust_proof).expect("read proof"));
        assert_eq!(
            external_proof_hash, rust_proof_hash,
            "external-z3-cli and rust-z3-lib backends must produce identical proof artifact hash for fixture `{}`",
            fixture.name
        );
    }
}
