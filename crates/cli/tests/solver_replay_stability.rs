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

fn write_fake_solver(dir: &Path) -> PathBuf {
    let source = dir.join("fake_z3.rs");
    let solver = if cfg!(windows) {
        dir.join("fake-z3.exe")
    } else {
        dir.join("fake-z3")
    };
    let fake_solver_src = r#"
use std::io::{self, Read};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|arg| arg == "--version" || arg == "-version") {
        println!("Z3 version 4.13.4 - fake");
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

#[test]
fn solver_enabled_replay_is_status_and_artifact_stable() {
    let tmp = tempdir().expect("tempdir");
    let source = tmp.path().join("main.clear");
    let wasm_1 = tmp.path().join("run1.wasm");
    let wasm_2 = tmp.path().join("run2.wasm");
    let vcs_1 = tmp.path().join("run1.vc.json");
    let vcs_2 = tmp.path().join("run2.vc.json");
    let proof_1 = tmp.path().join("run1.proof.json");
    let proof_2 = tmp.path().join("run2.proof.json");
    fs::write(&source, SAMPLE_SOURCE).expect("write source");
    let solver = write_fake_solver(tmp.path());

    for (wasm, vcs, proof) in [(&wasm_1, &vcs_1, &proof_1), (&wasm_2, &vcs_2, &proof_2)] {
        Command::cargo_bin("clg")
            .expect("cargo_bin clg")
            .env("CLG_SOLVER_BIN", &solver)
            .env("CLG_FAKE_Z3_RESULT", "sat")
            .arg("build")
            .arg(&source)
            .arg("-o")
            .arg(wasm)
            .arg("--emit-vcs")
            .arg(vcs)
            .arg("--emit-proof")
            .arg(proof)
            .assert()
            .success();
    }

    let vcs_json_1: Value =
        serde_json::from_slice(&fs::read(&vcs_1).expect("read run1 vcs")).expect("parse run1 vcs");
    let vcs_json_2: Value =
        serde_json::from_slice(&fs::read(&vcs_2).expect("read run2 vcs")).expect("parse run2 vcs");
    assert_eq!(
        vcs_json_1, vcs_json_2,
        "solver replay VC JSON should be stable"
    );

    let proof_bytes_1 = fs::read(&proof_1).expect("read run1 proof");
    let proof_bytes_2 = fs::read(&proof_2).expect("read run2 proof");
    assert_eq!(
        proof_bytes_1, proof_bytes_2,
        "solver replay proof artifact bytes should be stable"
    );
}
