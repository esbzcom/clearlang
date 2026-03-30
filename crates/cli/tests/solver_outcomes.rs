use assert_cmd::prelude::*;
use serde_json::Value;
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
        let version = std::env::var("CLG_FAKE_Z3_VERSION").unwrap_or_else(|_| "4.13.4".to_string());
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
