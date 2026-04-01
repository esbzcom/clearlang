#[cfg(feature = "rust-z3-lib")]
use assert_cmd::prelude::*;
#[cfg(feature = "rust-z3-lib")]
use serde_json::Value;
#[cfg(feature = "rust-z3-lib")]
use std::fs;
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
#[test]
fn rust_cutover_backend_has_no_runtime_dependency_on_external_solver_bundle() {
    let tmp = tempdir().expect("tempdir");
    let source = tmp.path().join("main.clear");
    let wasm = tmp.path().join("out.wasm");
    let vcs = tmp.path().join("out.vc.json");
    let proof = tmp.path().join("out.proof.json");
    fs::write(&source, SAMPLE_SOURCE).expect("write source");

    Command::cargo_bin("clg")
        .expect("cargo_bin clg")
        .current_dir(tmp.path())
        .env("CLG_SOLVER_BACKEND", "rust-z3-lib")
        .env("CLG_SOLVER_RUST_Z3_CUTOVER", "1")
        .env_remove("CLG_SOLVER_BIN")
        .env_remove("CLG_SOLVER_BUNDLE_ROOT")
        .arg("build")
        .arg("main.clear")
        .arg("-o")
        .arg("out.wasm")
        .arg("--emit-vcs")
        .arg("out.vc.json")
        .arg("--emit-proof")
        .arg("out.proof.json")
        .assert()
        .success();

    let vcs_json: Value =
        serde_json::from_slice(&fs::read(&vcs).expect("read vcs json")).expect("parse vcs");
    let first = vcs_json
        .as_array()
        .and_then(|items| items.first())
        .expect("first vc");
    assert_ne!(
        first.get("status").and_then(|v| v.as_str()),
        Some("generated"),
        "rust-z3-lib cutover must not depend on external solver bundle/runtime path"
    );

    let proof_json: Value =
        serde_json::from_slice(&fs::read(&proof).expect("read proof json")).expect("parse proof");
    let summary = proof_json
        .get("summary")
        .and_then(Value::as_object)
        .expect("proof summary");
    assert_eq!(
        summary.get("generated_count").and_then(Value::as_u64),
        Some(0),
        "rust-z3-lib cutover proof summary must not include generated placeholder outcomes"
    );

    assert!(wasm.exists(), "build output wasm should be emitted");
}
