use assert_cmd::prelude::*;
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::tempdir;

fn run_build_and_load_vcs(root: &Path, file_name: &str) -> Value {
    let source = r#"
        pure function check(a: Bytes, b: Bytes, x: U64, y: U64) -> Bool
            ensure { result == std::bytes::eq_ct(a, b) }
            ensure { (x & y) == x }
        {
            std::bytes::eq_ct(a, b)
        }
        function main() -> Int { 0 }
    "#;
    let src_path = root.join(format!("{file_name}.clear"));
    let wasm_path = root.join(format!("{file_name}.wasm"));
    let vcs_path = root.join(format!("{file_name}.vc.json"));
    fs::write(&src_path, source).expect("write source");

    Command::cargo_bin("clg")
        .expect("clg binary")
        .args(["build"])
        .arg(&src_path)
        .args(["-o"])
        .arg(&wasm_path)
        .args(["--emit-vcs"])
        .arg(&vcs_path)
        .assert()
        .success();

    serde_json::from_slice(&fs::read(&vcs_path).expect("read vcs json")).expect("parse vcs json")
}

fn parity_summary(vcs: &Value) -> Value {
    let entries = vcs.as_array().expect("vc output should be an array");
    let mut status_counts: BTreeMap<String, u64> = BTreeMap::new();
    let mut proof_status_counts: BTreeMap<String, u64> = BTreeMap::new();
    let mut assumption_boundaries: BTreeSet<String> = BTreeSet::new();

    for entry in entries {
        let obj = entry.as_object().expect("vc entry object");
        let status = obj
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string();
        *status_counts.entry(status).or_insert(0) += 1;

        if let Some(proof_status) = obj.get("proof_status").and_then(Value::as_str) {
            *proof_status_counts
                .entry(proof_status.to_string())
                .or_insert(0) += 1;
        }

        if let Some(items) = obj
            .get("assumptions")
            .and_then(|v| v.get("items"))
            .and_then(Value::as_array)
        {
            for item in items {
                if let Some(id) = item.get("id").and_then(Value::as_str) {
                    if !id.trim().is_empty() {
                        assumption_boundaries.insert(id.to_string());
                    }
                }
            }
        }
    }

    json!({
        "schema_version": 1,
        "vc_count": entries.len(),
        "status_counts": status_counts,
        "proof_status_counts": proof_status_counts,
        "assumption_boundaries": assumption_boundaries.into_iter().collect::<Vec<_>>(),
    })
}

fn write_summary(out_path: &Path, summary: &Value) {
    if let Some(parent) = out_path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).expect("create parity artifact dir");
        }
    }
    fs::write(
        out_path,
        serde_json::to_vec_pretty(summary).expect("serialize parity summary"),
    )
    .expect("write parity summary");
}

#[test]
fn milestone3_proof_parity_summary_is_deterministic() {
    let tmp = tempdir().expect("tempdir");
    let vcs_run1 = run_build_and_load_vcs(tmp.path(), "run1");
    let vcs_run2 = run_build_and_load_vcs(tmp.path(), "run2");

    let summary1 = parity_summary(&vcs_run1);
    let summary2 = parity_summary(&vcs_run2);
    assert_eq!(
        summary1, summary2,
        "proof parity summary should be deterministic for identical inputs"
    );

    if let Ok(out_path) = std::env::var("CLG_PROOF_PARITY_OUT") {
        write_summary(Path::new(&out_path), &summary1);
    }
}
