use assert_cmd::prelude::*;
use ed25519_dalek::{Signer, SigningKey};
use serde_json::json;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::tempdir;

const SAMPLE_SOURCE: &str = r#"
pure function inc(x: Int) -> Int
    require { x >= 0 }
    ensure { result > x }
{ x + 1 }
function main() -> Int { inc(1) }
"#;

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
        serde_json::to_vec_pretty(&json!({
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

fn write_fake_solver(dir: &Path) -> PathBuf {
    let solver = if cfg!(windows) {
        dir.join("fake-z3.exe")
    } else {
        dir.join("fake-z3")
    };
    write_fake_solver_to(&solver)
}

fn bundled_solver_path(bundle_root: &Path) -> PathBuf {
    if cfg!(windows) {
        bundle_root.join("windows").join("z3.exe")
    } else if cfg!(target_os = "macos") {
        bundle_root.join("macos").join("z3")
    } else {
        bundle_root.join("linux").join("z3")
    }
}

fn build_with_solver_status(fake_status: &str) -> Value {
    let tmp = tempdir().expect("tempdir");
    let source = tmp.path().join("main.clear");
    let wasm = tmp.path().join("out.wasm");
    let vcs = tmp.path().join("out.vc.json");
    fs::write(&source, SAMPLE_SOURCE).expect("write source");
    let solver = write_fake_solver(tmp.path());
    write_solver_integrity_sidecars(&solver);
    Command::cargo_bin("clg")
        .expect("cargo_bin clg")
        .env("CLG_SOLVER_BIN", solver)
        .env("CLG_FAKE_Z3_RESULT", fake_status)
        .arg("build")
        .arg(&source)
        .arg("-o")
        .arg(&wasm)
        .arg("--emit-vcs")
        .arg(&vcs)
        .assert()
        .success();
    serde_json::from_slice(&fs::read(&vcs).expect("read vcs")).expect("parse vcs json")
}

#[test]
fn configured_solver_maps_outcomes_to_vc_status() {
    let cases = [
        ("unsat", "proved"),
        ("sat", "failed"),
        ("unknown", "unknown"),
        ("timeout", "timeout"),
    ];
    for (fake, expected_status) in cases {
        let vcs = build_with_solver_status(fake);
        let first = vcs
            .as_array()
            .and_then(|items| items.first())
            .expect("first vc");
        assert_eq!(
            first.get("status").and_then(|value| value.as_str()),
            Some(expected_status),
            "expected fake solver result `{fake}` to map to vc status `{expected_status}`"
        );
    }
}

#[test]
fn configured_solver_statuses_flow_into_emit_proof_summary() {
    let tmp = tempdir().expect("tempdir");
    let source = tmp.path().join("main.clear");
    let wasm = tmp.path().join("out.wasm");
    let vcs = tmp.path().join("out.vc.json");
    let proof = tmp.path().join("out.proof.json");
    fs::write(&source, SAMPLE_SOURCE).expect("write source");
    let solver = write_fake_solver(tmp.path());
    write_solver_integrity_sidecars(&solver);
    Command::cargo_bin("clg")
        .expect("cargo_bin clg")
        .env("CLG_SOLVER_BIN", solver)
        .env("CLG_FAKE_Z3_RESULT", "sat")
        .arg("build")
        .arg(&source)
        .arg("-o")
        .arg(&wasm)
        .arg("--emit-vcs")
        .arg(&vcs)
        .arg("--emit-proof")
        .arg(&proof)
        .assert()
        .success();
    let artifact: Value =
        serde_json::from_slice(&fs::read(&proof).expect("read proof artifact")).expect("json");
    let summary = artifact
        .get("summary")
        .and_then(|value| value.as_object())
        .expect("summary");
    assert_eq!(
        summary.get("failed_count").and_then(|v| v.as_u64()),
        Some(1)
    );
    assert_eq!(
        summary.get("proved_count").and_then(|v| v.as_u64()),
        Some(0)
    );
    assert_eq!(
        summary.get("unknown_count").and_then(|v| v.as_u64()),
        Some(0)
    );
    assert_eq!(
        summary.get("timeout_count").and_then(|v| v.as_u64()),
        Some(0)
    );
    assert_eq!(
        summary.get("generated_count").and_then(|v| v.as_u64()),
        Some(0),
        "configured solver should replace generated status with deterministic solver status"
    );
}

#[test]
fn missing_solver_binary_keeps_generated_status() {
    let tmp = tempdir().expect("tempdir");
    let source = tmp.path().join("main.clear");
    let wasm = tmp.path().join("out.wasm");
    let vcs = tmp.path().join("out.vc.json");
    fs::write(&source, SAMPLE_SOURCE).expect("write source");
    let missing_solver = tmp.path().join("missing-z3");
    Command::cargo_bin("clg")
        .expect("cargo_bin clg")
        .env("CLG_SOLVER_BIN", missing_solver)
        .arg("build")
        .arg(&source)
        .arg("-o")
        .arg(&wasm)
        .arg("--emit-vcs")
        .arg(&vcs)
        .assert()
        .success();
    let vcs_json: Value =
        serde_json::from_slice(&fs::read(&vcs).expect("read vcs json")).expect("parse vcs");
    let first = vcs_json
        .as_array()
        .and_then(|items| items.first())
        .expect("first vc");
    assert_eq!(
        first.get("status").and_then(|v| v.as_str()),
        Some("generated")
    );
}

#[test]
fn configured_solver_version_mismatch_fails_build() {
    let tmp = tempdir().expect("tempdir");
    let source = tmp.path().join("main.clear");
    let wasm = tmp.path().join("out.wasm");
    let vcs = tmp.path().join("out.vc.json");
    fs::write(&source, SAMPLE_SOURCE).expect("write source");
    let solver = write_fake_solver(tmp.path());
    write_solver_integrity_sidecars(&solver);
    Command::cargo_bin("clg")
        .expect("cargo_bin clg")
        .env("CLG_SOLVER_BIN", solver)
        .env("CLG_FAKE_Z3_RESULT", "sat")
        .env("CLG_FAKE_Z3_VERSION", "9.9.9")
        .arg("build")
        .arg(&source)
        .arg("-o")
        .arg(&wasm)
        .arg("--emit-vcs")
        .arg(&vcs)
        .assert()
        .failure();
}

#[test]
fn bundled_solver_root_is_used_when_solver_env_not_set() {
    let tmp = tempdir().expect("tempdir");
    let source = tmp.path().join("main.clear");
    let wasm = tmp.path().join("out.wasm");
    let vcs = tmp.path().join("out.vc.json");
    fs::write(&source, SAMPLE_SOURCE).expect("write source");
    let bundle_root = tmp.path().join("solver-bundle");
    let bundled_solver = bundled_solver_path(&bundle_root);
    write_fake_solver_to(&bundled_solver);
    write_solver_integrity_sidecars(&bundled_solver);
    Command::cargo_bin("clg")
        .expect("cargo_bin clg")
        .env("CLG_SOLVER_BUNDLE_ROOT", bundle_root)
        .env("CLG_FAKE_Z3_RESULT", "sat")
        .arg("build")
        .arg(&source)
        .arg("-o")
        .arg(&wasm)
        .arg("--emit-vcs")
        .arg(&vcs)
        .assert()
        .success();
    let vcs_json: Value =
        serde_json::from_slice(&fs::read(&vcs).expect("read vcs json")).expect("parse vcs");
    let first = vcs_json
        .as_array()
        .and_then(|items| items.first())
        .expect("first vc");
    assert_eq!(
        first.get("status").and_then(|v| v.as_str()),
        Some("failed"),
        "bundled solver should execute and map sat to failed status"
    );
}

#[test]
fn extracted_solver_bundle_layout_is_used_when_solver_env_not_set() {
    let tmp = tempdir().expect("tempdir");
    let source = tmp.path().join("main.clear");
    let wasm = tmp.path().join("out.wasm");
    let vcs = tmp.path().join("out.vc.json");
    fs::write(&source, SAMPLE_SOURCE).expect("write source");

    let extracted_solver = if cfg!(windows) {
        tmp.path()
            .join("tools")
            .join("proof")
            .join("z3")
            .join("z3-extract")
            .join("z3-4.16.0-x64-win")
            .join("bin")
            .join("z3.exe")
    } else if cfg!(target_os = "macos") {
        tmp.path()
            .join("tools")
            .join("proof")
            .join("z3")
            .join("z3-extract")
            .join("z3-4.16.0-x64-osx")
            .join("bin")
            .join("z3")
    } else {
        tmp.path()
            .join("tools")
            .join("proof")
            .join("z3")
            .join("z3-extract")
            .join("z3-4.16.0-x64-glibc-2.39")
            .join("bin")
            .join("z3")
    };
    write_fake_solver_to(&extracted_solver);
    write_solver_integrity_sidecars(&extracted_solver);

    Command::cargo_bin("clg")
        .expect("cargo_bin clg")
        .current_dir(tmp.path())
        .env("CLG_FAKE_Z3_RESULT", "sat")
        .arg("build")
        .arg("main.clear")
        .arg("-o")
        .arg("out.wasm")
        .arg("--emit-vcs")
        .arg("out.vc.json")
        .assert()
        .success();

    let vcs_json: Value =
        serde_json::from_slice(&fs::read(&vcs).expect("read vcs json")).expect("parse vcs");
    let first = vcs_json
        .as_array()
        .and_then(|items| items.first())
        .expect("first vc");
    assert_eq!(
        first.get("status").and_then(|v| v.as_str()),
        Some("failed"),
        "extracted solver bundle fallback should execute and map sat to failed status"
    );
    assert!(wasm.exists(), "build output wasm should be generated");
}

#[test]
fn bundled_solver_without_integrity_sidecars_keeps_generated_status() {
    let tmp = tempdir().expect("tempdir");
    let source = tmp.path().join("main.clear");
    let wasm = tmp.path().join("out.wasm");
    let vcs = tmp.path().join("out.vc.json");
    fs::write(&source, SAMPLE_SOURCE).expect("write source");

    let bundle_root = tmp.path().join("solver-bundle");
    let bundled_solver = bundled_solver_path(&bundle_root);
    write_fake_solver_to(&bundled_solver);

    Command::cargo_bin("clg")
        .expect("cargo_bin clg")
        .env("CLG_SOLVER_BUNDLE_ROOT", bundle_root)
        .env("CLG_FAKE_Z3_RESULT", "sat")
        .arg("build")
        .arg(&source)
        .arg("-o")
        .arg(&wasm)
        .arg("--emit-vcs")
        .arg(&vcs)
        .assert()
        .success();
    let vcs_json: Value =
        serde_json::from_slice(&fs::read(&vcs).expect("read vcs json")).expect("parse vcs");
    let first = vcs_json
        .as_array()
        .and_then(|items| items.first())
        .expect("first vc");
    assert_eq!(
        first.get("status").and_then(|v| v.as_str()),
        Some("generated"),
        "solver bundle without required signature sidecars must be rejected and keep generated status"
    );
}

#[test]
fn explicit_solver_env_path_overrides_bundle_root() {
    let tmp = tempdir().expect("tempdir");
    let source = tmp.path().join("main.clear");
    let wasm = tmp.path().join("out.wasm");
    let vcs = tmp.path().join("out.vc.json");
    fs::write(&source, SAMPLE_SOURCE).expect("write source");
    let bundle_root = tmp.path().join("solver-bundle");
    let bundled_solver = bundled_solver_path(&bundle_root);
    write_fake_solver_to(&bundled_solver);
    let missing_solver = tmp.path().join("missing-z3.exe");
    Command::cargo_bin("clg")
        .expect("cargo_bin clg")
        .env("CLG_SOLVER_BIN", missing_solver)
        .env("CLG_SOLVER_BUNDLE_ROOT", bundle_root)
        .env("CLG_FAKE_Z3_RESULT", "sat")
        .arg("build")
        .arg(&source)
        .arg("-o")
        .arg(&wasm)
        .arg("--emit-vcs")
        .arg(&vcs)
        .assert()
        .success();
    let vcs_json: Value =
        serde_json::from_slice(&fs::read(&vcs).expect("read vcs json")).expect("parse vcs");
    let first = vcs_json
        .as_array()
        .and_then(|items| items.first())
        .expect("first vc");
    assert_eq!(
        first.get("status").and_then(|v| v.as_str()),
        Some("generated"),
        "explicit CLG_SOLVER_BIN should take precedence over bundle-root fallback"
    );
}

#[test]
fn configured_solver_without_integrity_sidecars_keeps_generated_status() {
    let tmp = tempdir().expect("tempdir");
    let source = tmp.path().join("main.clear");
    let wasm = tmp.path().join("out.wasm");
    let vcs = tmp.path().join("out.vc.json");
    fs::write(&source, SAMPLE_SOURCE).expect("write source");
    let solver = write_fake_solver(tmp.path());

    Command::cargo_bin("clg")
        .expect("cargo_bin clg")
        .env("CLG_SOLVER_BIN", solver)
        .env("CLG_FAKE_Z3_RESULT", "sat")
        .arg("build")
        .arg(&source)
        .arg("-o")
        .arg(&wasm)
        .arg("--emit-vcs")
        .arg(&vcs)
        .assert()
        .success();

    let vcs_json: Value =
        serde_json::from_slice(&fs::read(&vcs).expect("read vcs json")).expect("parse vcs");
    let first = vcs_json
        .as_array()
        .and_then(|items| items.first())
        .expect("first vc");
    assert_eq!(
        first.get("status").and_then(|v| v.as_str()),
        Some("generated"),
        "configured solver without required signature sidecars must be rejected and keep generated status"
    );
}

#[test]
fn configured_solver_with_corrupt_checksum_sidecar_keeps_generated_status() {
    let tmp = tempdir().expect("tempdir");
    let source = tmp.path().join("main.clear");
    let wasm = tmp.path().join("out.wasm");
    let vcs = tmp.path().join("out.vc.json");
    fs::write(&source, SAMPLE_SOURCE).expect("write source");
    let solver = write_fake_solver(tmp.path());
    write_solver_integrity_sidecars(&solver);
    let checksum_path = solver.with_file_name(format!(
        "{}.sha256",
        solver
            .file_name()
            .expect("solver filename")
            .to_string_lossy()
    ));
    fs::write(
        &checksum_path,
        "sha256:0000000000000000000000000000000000000000000000000000000000000000\n",
    )
    .expect("write corrupt checksum sidecar");

    Command::cargo_bin("clg")
        .expect("cargo_bin clg")
        .env("CLG_SOLVER_BIN", solver)
        .env("CLG_FAKE_Z3_RESULT", "sat")
        .arg("build")
        .arg(&source)
        .arg("-o")
        .arg(&wasm)
        .arg("--emit-vcs")
        .arg(&vcs)
        .assert()
        .success();

    let vcs_json: Value =
        serde_json::from_slice(&fs::read(&vcs).expect("read vcs json")).expect("parse vcs");
    let first = vcs_json
        .as_array()
        .and_then(|items| items.first())
        .expect("first vc");
    assert_eq!(
        first.get("status").and_then(|v| v.as_str()),
        Some("generated"),
        "solver with corrupt checksum sidecar must be rejected and keep generated status"
    );
}

#[test]
fn configured_solver_with_corrupt_signature_sidecar_keeps_generated_status() {
    let tmp = tempdir().expect("tempdir");
    let source = tmp.path().join("main.clear");
    let wasm = tmp.path().join("out.wasm");
    let vcs = tmp.path().join("out.vc.json");
    fs::write(&source, SAMPLE_SOURCE).expect("write source");
    let solver = write_fake_solver(tmp.path());
    write_solver_integrity_sidecars(&solver);
    let signature_path = solver.with_file_name(format!(
        "{}.sig",
        solver
            .file_name()
            .expect("solver filename")
            .to_string_lossy()
    ));
    let mut signature_json: Value =
        serde_json::from_slice(&fs::read(&signature_path).expect("read signature sidecar"))
            .expect("parse signature sidecar");
    signature_json["signature"] =
        Value::String("ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff".to_string());
    fs::write(
        &signature_path,
        serde_json::to_vec_pretty(&signature_json).expect("serialize signature sidecar"),
    )
    .expect("write corrupt signature sidecar");

    Command::cargo_bin("clg")
        .expect("cargo_bin clg")
        .env("CLG_SOLVER_BIN", solver)
        .env("CLG_FAKE_Z3_RESULT", "sat")
        .arg("build")
        .arg(&source)
        .arg("-o")
        .arg(&wasm)
        .arg("--emit-vcs")
        .arg(&vcs)
        .assert()
        .success();

    let vcs_json: Value =
        serde_json::from_slice(&fs::read(&vcs).expect("read vcs json")).expect("parse vcs");
    let first = vcs_json
        .as_array()
        .and_then(|items| items.first())
        .expect("first vc");
    assert_eq!(
        first.get("status").and_then(|v| v.as_str()),
        Some("generated"),
        "solver with corrupt signature sidecar must be rejected and keep generated status"
    );
}
