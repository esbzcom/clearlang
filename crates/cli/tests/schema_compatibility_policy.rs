use assert_cmd::prelude::*;
use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::tempdir;

fn write_source(root: &Path) -> std::path::PathBuf {
    let src = r#"
        function main() -> Int { 0 }
    "#;
    let file = root.join("main.clear");
    fs::write(&file, src).expect("write source");
    file
}

fn write_lockfile_v0(root: &Path) {
    fs::write(
        root.join("clg.lock.json"),
        r#"{"schema_version":0,"dependencies":[]}"#,
    )
    .expect("write lockfile");
}

fn write_trust_policy_v0(root: &Path) {
    fs::write(
        root.join("clg.trust-policy.json"),
        r#"{
  "schema_version": 0,
  "trusted_signers": [],
  "revoked_key_ids": []
}"#,
    )
    .expect("write trust policy");
}

fn write_host_profile_v0(root: &Path) {
    fs::write(
        root.join("clg.host-profile.json"),
        r#"{
  "schema_version": 0,
  "profile": "contract_static",
  "capabilities": []
}"#,
    )
    .expect("write host profile");
}

fn write_package_abi(root: &Path, schema_version: u32) {
    let value = serde_json::json!({
        "schema_version": schema_version,
        "contracts": [],
    });
    fs::write(
        root.join("clg.package-abi.json"),
        serde_json::to_vec_pretty(&value).expect("serialize abi"),
    )
    .expect("write package abi");
}

fn write_package_metadata(root: &Path, schema_version: u32) {
    let value = serde_json::json!({
        "schema_version": schema_version,
        "packages": [],
    });
    fs::write(
        root.join("clg.package-metadata.json"),
        serde_json::to_vec_pretty(&value).expect("serialize package metadata"),
    )
    .expect("write package metadata");
}

fn run_strict_build_json_failure(root: &Path, file: &Path) -> Value {
    let out = root.join("out.wasm");
    let vcs = root.join("out.vc.json");
    let output = Command::cargo_bin("clg")
        .unwrap()
        .args(["--json-errors", "build"])
        .arg(file)
        .args(["-o"])
        .arg(&out)
        .args(["--emit-vcs"])
        .arg(&vcs)
        .args(["--compiler-mode", "strict"])
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    serde_json::from_slice(&output).expect("json errors")
}

fn assert_single_json_error(v: &Value, code: &str, stage: &str) {
    assert_eq!(v.get("ok").and_then(|b| b.as_bool()), Some(false));
    let errs = v
        .get("errors")
        .and_then(|e| e.as_array())
        .expect("errors array");
    assert_eq!(errs.len(), 1, "expected exactly one error");
    let e0 = &errs[0];
    assert_eq!(e0.get("code").and_then(|s| s.as_str()), Some(code));
    assert_eq!(e0.get("stage").and_then(|s| s.as_str()), Some(stage));
}

#[test]
fn strict_schema_compat_accepts_package_metadata_v0() {
    let tmp = tempdir().unwrap();
    let file = write_source(tmp.path());
    write_lockfile_v0(tmp.path());
    write_trust_policy_v0(tmp.path());
    write_host_profile_v0(tmp.path());
    write_package_metadata(tmp.path(), 0);
    write_package_abi(tmp.path(), 0);

    let out = tmp.path().join("out.wasm");
    let vcs = tmp.path().join("out.vc.json");
    Command::cargo_bin("clg")
        .unwrap()
        .args(["build"])
        .arg(&file)
        .args(["-o"])
        .arg(&out)
        .args(["--emit-vcs"])
        .arg(&vcs)
        .args(["--compiler-mode", "strict"])
        .assert()
        .success();
}

#[test]
fn strict_schema_compat_accepts_package_metadata_v1() {
    let tmp = tempdir().unwrap();
    let file = write_source(tmp.path());
    write_lockfile_v0(tmp.path());
    write_trust_policy_v0(tmp.path());
    write_host_profile_v0(tmp.path());
    write_package_metadata(tmp.path(), 1);
    write_package_abi(tmp.path(), 0);

    let out = tmp.path().join("out.wasm");
    let vcs = tmp.path().join("out.vc.json");
    Command::cargo_bin("clg")
        .unwrap()
        .args(["build"])
        .arg(&file)
        .args(["-o"])
        .arg(&out)
        .args(["--emit-vcs"])
        .arg(&vcs)
        .args(["--compiler-mode", "strict"])
        .assert()
        .success();
}

#[test]
fn strict_schema_compat_rejects_package_metadata_v2_with_c104() {
    let tmp = tempdir().unwrap();
    let file = write_source(tmp.path());
    write_lockfile_v0(tmp.path());
    write_trust_policy_v0(tmp.path());
    write_host_profile_v0(tmp.path());
    write_package_metadata(tmp.path(), 2);
    write_package_abi(tmp.path(), 0);

    let v = run_strict_build_json_failure(tmp.path(), &file);
    assert_single_json_error(&v, "C104", "build");
}

#[test]
fn strict_schema_compat_rejects_package_abi_v1_with_c104() {
    let tmp = tempdir().unwrap();
    let file = write_source(tmp.path());
    write_lockfile_v0(tmp.path());
    write_trust_policy_v0(tmp.path());
    write_host_profile_v0(tmp.path());
    write_package_metadata(tmp.path(), 0);
    write_package_abi(tmp.path(), 1);

    let v = run_strict_build_json_failure(tmp.path(), &file);
    assert_single_json_error(&v, "C104", "build");
}
