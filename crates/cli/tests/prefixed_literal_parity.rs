use assert_cmd::prelude::*;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::tempdir;

const DECIMAL_SOURCE: &str = r#"
pure function mask(x: U64) -> U64
    ensure { result == (x & 255) }
{ x & 170 }

function main() -> Int { 0 }
"#;

const PREFIXED_SOURCE: &str = r#"
pure function mask(x: U64) -> U64
    ensure { result == (x & 0xFF) }
{ x & 0b1010_1010 }

function main() -> Int { 0 }
"#;

fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

fn write_source(path: &Path, source: &str) {
    fs::write(path, source).expect("write source");
}

fn build_with_outputs(
    root: &Path,
    src: &Path,
    wasm: &Path,
    vcs: &Path,
    proof: &Path,
    missing_solver: &Path,
) {
    let mut cmd = Command::cargo_bin("clg").expect("clg binary");
    cmd.current_dir(root)
        .env("CLG_SOLVER_BIN", missing_solver)
        .args(["build"])
        .arg(src)
        .args(["-o"])
        .arg(wasm)
        .args(["--compiler-mode", "standard"])
        .arg("--emit-vcs")
        .arg(vcs)
        .arg("--emit-proof")
        .arg(proof)
        .assert()
        .success();
}

fn vc_semantic_fingerprint(vcs_path: &Path) -> Value {
    let raw: Value =
        serde_json::from_slice(&fs::read(vcs_path).expect("read vcs json")).expect("parse vcs");
    let arr = raw.as_array().expect("vcs should be an array");
    let mut out = Vec::with_capacity(arr.len());
    for vc in arr {
        let pre_smt2 = vc
            .get("pre")
            .and_then(|v| v.get("smt2"))
            .and_then(Value::as_str)
            .expect("pre.smt2")
            .to_string();
        let post_smt2 = vc
            .get("post")
            .and_then(|v| v.get("smt2"))
            .and_then(Value::as_str)
            .expect("post.smt2")
            .to_string();
        let vc_smt2 = vc
            .get("vc")
            .and_then(|v| v.get("smt2"))
            .and_then(Value::as_str)
            .expect("vc.smt2")
            .to_string();
        let assumptions = vc
            .get("diagnostics")
            .and_then(|d| d.get("proof_context"))
            .and_then(|p| p.get("assumptions"))
            .and_then(|a| a.get("items"))
            .cloned()
            .unwrap_or_else(|| json!([]));

        out.push(json!({
            "function": vc.get("function").cloned().unwrap_or(Value::Null),
            "vc_id": vc.get("vc_id").cloned().unwrap_or(Value::Null),
            "status": vc.get("status").cloned().unwrap_or(Value::Null),
            "assurance_tier": vc.get("assurance").and_then(|a| a.get("tier")).cloned().unwrap_or(Value::Null),
            "pre_smt2": pre_smt2,
            "post_smt2": post_smt2,
            "vc_smt2": vc_smt2,
            "assumption_items": assumptions,
        }));
    }
    Value::Array(out)
}

#[test]
fn prefixed_literals_preserve_proof_and_vc_determinism_against_decimal_equivalents() {
    let tmp = tempdir().expect("tempdir");
    let root = tmp.path();
    let src = root.join("main.clear");
    let missing_solver = if cfg!(windows) {
        root.join("missing-z3.exe")
    } else {
        root.join("missing-z3")
    };

    let decimal_wasm = root.join("decimal.wasm");
    let decimal_vcs = root.join("decimal.vc.json");
    let decimal_proof = root.join("decimal.proof.json");
    write_source(&src, DECIMAL_SOURCE);
    build_with_outputs(
        root,
        &src,
        &decimal_wasm,
        &decimal_vcs,
        &decimal_proof,
        &missing_solver,
    );

    let prefixed_wasm = root.join("prefixed.wasm");
    let prefixed_vcs = root.join("prefixed.vc.json");
    let prefixed_proof = root.join("prefixed.proof.json");
    write_source(&src, PREFIXED_SOURCE);
    build_with_outputs(
        root,
        &src,
        &prefixed_wasm,
        &prefixed_vcs,
        &prefixed_proof,
        &missing_solver,
    );

    let decimal_vc_fp = vc_semantic_fingerprint(&decimal_vcs);
    let prefixed_vc_fp = vc_semantic_fingerprint(&prefixed_vcs);
    assert_eq!(
        decimal_vc_fp, prefixed_vc_fp,
        "decimal and prefixed sources must emit equivalent VC semantic payloads"
    );

    let decimal_proof_bytes = fs::read(&decimal_proof).expect("read decimal proof");
    let prefixed_proof_bytes = fs::read(&prefixed_proof).expect("read prefixed proof");
    assert_eq!(
        decimal_proof_bytes, prefixed_proof_bytes,
        "decimal and prefixed sources must emit byte-identical proof artifacts under identical deterministic inputs"
    );
    assert_eq!(
        sha256_hex(&decimal_proof_bytes),
        sha256_hex(&prefixed_proof_bytes),
        "decimal and prefixed sources must emit identical proof artifact hashes"
    );
}
