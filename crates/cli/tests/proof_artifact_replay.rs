use assert_cmd::prelude::*;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::tempdir;

fn write_sample_source(path: &Path) {
    let src = r#"
        pure function inc(x: Int) -> Int
            require { 0 <= x }
            ensure { result > x }
        { x + 1 }

        function main() -> Int { inc(1) }
    "#;
    fs::write(path, src).expect("write sample source");
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

#[test]
fn emit_proof_replay_with_identical_inputs_is_byte_identical() {
    let tmp = tempdir().expect("tempdir");
    let root = tmp.path();
    let src = root.join("app.clear");
    write_sample_source(&src);

    let wasm_1 = root.join("run1.wasm");
    let vcs_1 = root.join("run1.vc.json");
    let proof_1 = root.join("run1.proof.json");
    let wasm_2 = root.join("run2.wasm");
    let vcs_2 = root.join("run2.vc.json");
    let proof_2 = root.join("run2.proof.json");

    let mut first = Command::cargo_bin("clg").expect("clg binary");
    first
        .current_dir(root)
        .args(["build"])
        .arg(&src)
        .args(["-o"])
        .arg(&wasm_1)
        .args(["--compiler-mode", "standard"])
        .arg("--emit-vcs")
        .arg(&vcs_1)
        .arg("--emit-proof")
        .arg(&proof_1)
        .assert()
        .success();

    let mut second = Command::cargo_bin("clg").expect("clg binary");
    second
        .current_dir(root)
        .args(["build"])
        .arg(&src)
        .args(["-o"])
        .arg(&wasm_2)
        .args(["--compiler-mode", "standard"])
        .arg("--emit-vcs")
        .arg(&vcs_2)
        .arg("--emit-proof")
        .arg(&proof_2)
        .assert()
        .success();

    let proof_bytes_1 = fs::read(&proof_1).expect("read run1 proof artifact");
    let proof_bytes_2 = fs::read(&proof_2).expect("read run2 proof artifact");
    assert_eq!(
        proof_bytes_1, proof_bytes_2,
        "proof artifact bytes must be deterministic across identical runs"
    );

    let hash_1 = sha256_hex(&proof_bytes_1);
    let hash_2 = sha256_hex(&proof_bytes_2);
    assert_eq!(hash_1, hash_2, "proof artifact hash must be deterministic");
}
