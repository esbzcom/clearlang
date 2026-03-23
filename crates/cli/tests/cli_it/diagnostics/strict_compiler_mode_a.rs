#[test]
fn strict_compiler_mode_requires_lockfile_with_c101() {
    let src = r#"
        function main() -> Int { 0 }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("strict_mode_no_lock.clear");
    fs::write(&file, src).expect("write");
    let out = tmp.path().join("out.wasm");
    let vcs = tmp.path().join("out.vc.json");

    let mut cmd = Command::cargo_bin("clg").unwrap();
    cmd.args(["--json-errors", "build"])
        .arg(&file)
        .args(["-o"])
        .arg(&out)
        .args(["--emit-vcs"])
        .arg(&vcs)
        .args(["--compiler-mode", "strict"]);
    let output = cmd.assert().failure().get_output().stdout.clone();
    let v: Value = serde_json::from_slice(&output).expect("json");
    assert_single_json_error(&v, "C101", "build");
}

#[test]
fn strict_compiler_mode_rejects_legacy_package_metadata_source_with_c101() {
    let src = r#"
        function main() -> Int { 0 }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("strict_mode_legacy_package_source.clear");
    fs::write(&file, src).expect("write");
    write_minimal_strict_preflight_files(tmp.path());
    fs::write(
        tmp.path().join("clg-packages.json"),
        r#"{
  "schema_version": 1,
  "packages": []
}"#,
    )
    .expect("write legacy package metadata source");
    let out = tmp.path().join("out.wasm");
    let vcs = tmp.path().join("out.vc.json");

    let mut cmd = Command::cargo_bin("clg").unwrap();
    cmd.args(["--json-errors", "build"])
        .arg(&file)
        .args(["-o"])
        .arg(&out)
        .args(["--emit-vcs"])
        .arg(&vcs)
        .args(["--compiler-mode", "strict"]);
    let output = cmd.assert().failure().get_output().stdout.clone();
    let v: Value = serde_json::from_slice(&output).expect("json");
    assert_single_json_error(&v, "C101", "build");
}

#[test]
fn strict_compiler_mode_preflight_errors_are_reported_before_typecheck() {
    let src = r#"
        function main() -> Int { unknown_fn(1) }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp
        .path()
        .join("strict_mode_preflight_before_typecheck.clear");
    fs::write(&file, src).expect("write");
    let out = tmp.path().join("out.wasm");
    let vcs = tmp.path().join("out.vc.json");

    let mut cmd = Command::cargo_bin("clg").unwrap();
    cmd.args(["--json-errors", "build"])
        .arg(&file)
        .args(["-o"])
        .arg(&out)
        .args(["--emit-vcs"])
        .arg(&vcs)
        .args(["--compiler-mode", "strict"]);
    let output = cmd.assert().failure().get_output().stdout.clone();
    let v: Value = serde_json::from_slice(&output).expect("json");
    assert_single_json_error(&v, "C101", "build");
}

#[test]
fn strict_compiler_mode_requires_trust_policy_with_c103() {
    let src = r#"
        function main() -> Int { 0 }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("strict_mode_no_trust_policy.clear");
    fs::write(&file, src).expect("write");
    write_minimal_strict_lockfile(tmp.path());
    write_minimal_strict_host_profile(tmp.path());
    write_minimal_strict_package_metadata(tmp.path());
    write_minimal_strict_package_abi(tmp.path());
    let out = tmp.path().join("out.wasm");
    let vcs = tmp.path().join("out.vc.json");

    let mut cmd = Command::cargo_bin("clg").unwrap();
    cmd.args(["--json-errors", "build"])
        .arg(&file)
        .args(["-o"])
        .arg(&out)
        .args(["--emit-vcs"])
        .arg(&vcs)
        .args(["--compiler-mode", "strict"]);
    let output = cmd.assert().failure().get_output().stdout.clone();
    let v: Value = serde_json::from_slice(&output).expect("json");
    assert_single_json_error(&v, "C103", "build");
}

#[test]
fn strict_compiler_mode_requires_host_profile_with_c106() {
    let src = r#"
        function main() -> Int { 0 }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("strict_mode_no_host_profile.clear");
    fs::write(&file, src).expect("write");
    write_minimal_strict_lockfile(tmp.path());
    write_minimal_strict_trust_policy(tmp.path());
    write_minimal_strict_package_metadata(tmp.path());
    write_minimal_strict_package_abi(tmp.path());
    let out = tmp.path().join("out.wasm");
    let vcs = tmp.path().join("out.vc.json");

    let mut cmd = Command::cargo_bin("clg").unwrap();
    cmd.args(["--json-errors", "build"])
        .arg(&file)
        .args(["-o"])
        .arg(&out)
        .args(["--emit-vcs"])
        .arg(&vcs)
        .args(["--compiler-mode", "strict"]);
    let output = cmd.assert().failure().get_output().stdout.clone();
    let v: Value = serde_json::from_slice(&output).expect("json");
    assert_single_json_error(&v, "C106", "build");
}

#[test]
fn strict_compiler_mode_rejects_invalid_trust_policy_schema_with_c103() {
    let src = r#"
        function main() -> Int { 0 }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("strict_mode_bad_trust_schema.clear");
    fs::write(&file, src).expect("write");
    write_minimal_strict_lockfile(tmp.path());
    fs::write(
        tmp.path().join("clg.trust-policy.json"),
        r#"{"schema_version":1,"trusted_signers":[],"revoked_key_ids":[]}"#,
    )
    .expect("write invalid strict trust policy");
    write_minimal_strict_host_profile(tmp.path());
    write_minimal_strict_package_metadata(tmp.path());
    write_minimal_strict_package_abi(tmp.path());
    let out = tmp.path().join("out.wasm");
    let vcs = tmp.path().join("out.vc.json");

    let mut cmd = Command::cargo_bin("clg").unwrap();
    cmd.args(["--json-errors", "build"])
        .arg(&file)
        .args(["-o"])
        .arg(&out)
        .args(["--emit-vcs"])
        .arg(&vcs)
        .args(["--compiler-mode", "strict"]);
    let output = cmd.assert().failure().get_output().stdout.clone();
    let v: Value = serde_json::from_slice(&output).expect("json");
    assert_single_json_error(&v, "C103", "build");
}

#[test]
fn strict_compiler_mode_rejects_invalid_host_profile_schema_with_c106() {
    let src = r#"
        function main() -> Int { 0 }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("strict_mode_bad_host_schema.clear");
    fs::write(&file, src).expect("write");
    write_minimal_strict_lockfile(tmp.path());
    write_minimal_strict_trust_policy(tmp.path());
    write_minimal_strict_package_metadata(tmp.path());
    write_minimal_strict_package_abi(tmp.path());
    fs::write(
        tmp.path().join("clg.host-profile.json"),
        r#"{"schema_version":1,"profile":"contract_static","capabilities":[]}"#,
    )
    .expect("write invalid strict host profile");
    let out = tmp.path().join("out.wasm");
    let vcs = tmp.path().join("out.vc.json");

    let mut cmd = Command::cargo_bin("clg").unwrap();
    cmd.args(["--json-errors", "build"])
        .arg(&file)
        .args(["-o"])
        .arg(&out)
        .args(["--emit-vcs"])
        .arg(&vcs)
        .args(["--compiler-mode", "strict"]);
    let output = cmd.assert().failure().get_output().stdout.clone();
    let v: Value = serde_json::from_slice(&output).expect("json");
    assert_single_json_error(&v, "C106", "build");
}

#[test]
fn strict_compiler_mode_rejects_invalid_lockfile_schema_with_c104() {
    let src = r#"
        function main() -> Int { 0 }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("strict_mode_lock_schema.clear");
    fs::write(&file, src).expect("write");
    write_minimal_strict_trust_policy(tmp.path());
    write_minimal_strict_host_profile(tmp.path());
    write_minimal_strict_package_metadata(tmp.path());
    write_minimal_strict_package_abi(tmp.path());
    fs::write(
        tmp.path().join("clg.lock.json"),
        r#"{"schema_version":2,"dependencies":[]}"#,
    )
    .expect("write lockfile");
    let out = tmp.path().join("out.wasm");
    let vcs = tmp.path().join("out.vc.json");

    let mut cmd = Command::cargo_bin("clg").unwrap();
    cmd.args(["--json-errors", "build"])
        .arg(&file)
        .args(["-o"])
        .arg(&out)
        .args(["--emit-vcs"])
        .arg(&vcs)
        .args(["--compiler-mode", "strict"]);
    let output = cmd.assert().failure().get_output().stdout.clone();
    let v: Value = serde_json::from_slice(&output).expect("json");
    assert_single_json_error(&v, "C104", "build");
}

#[test]
fn strict_compiler_mode_rejects_invalid_package_metadata_schema_with_c104() {
    let src = r#"
        function main() -> Int { 0 }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("strict_mode_pkg_metadata_schema.clear");
    fs::write(&file, src).expect("write");
    write_minimal_strict_lockfile(tmp.path());
    write_minimal_strict_trust_policy(tmp.path());
    write_minimal_strict_host_profile(tmp.path());
    write_minimal_strict_package_abi(tmp.path());
    fs::write(
        tmp.path().join("clg.package-metadata.json"),
        r#"{"schema_version":2,"packages":[]}"#,
    )
    .expect("write strict package metadata");
    let out = tmp.path().join("out.wasm");
    let vcs = tmp.path().join("out.vc.json");

    let mut cmd = Command::cargo_bin("clg").unwrap();
    cmd.args(["--json-errors", "build"])
        .arg(&file)
        .args(["-o"])
        .arg(&out)
        .args(["--emit-vcs"])
        .arg(&vcs)
        .args(["--compiler-mode", "strict"]);
    let output = cmd.assert().failure().get_output().stdout.clone();
    let v: Value = serde_json::from_slice(&output).expect("json");
    assert_single_json_error(&v, "C104", "build");
}

#[test]
fn strict_compiler_mode_rejects_invalid_package_abi_schema_with_c104() {
    let src = r#"
        function main() -> Int { 0 }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("strict_mode_pkg_abi_schema.clear");
    fs::write(&file, src).expect("write");
    write_minimal_strict_lockfile(tmp.path());
    write_minimal_strict_trust_policy(tmp.path());
    write_minimal_strict_host_profile(tmp.path());
    write_minimal_strict_package_metadata(tmp.path());
    fs::write(
        tmp.path().join("clg.package-abi.json"),
        r#"{"schema_version":1,"contracts":[]}"#,
    )
    .expect("write strict package ABI");
    let out = tmp.path().join("out.wasm");
    let vcs = tmp.path().join("out.vc.json");

    let mut cmd = Command::cargo_bin("clg").unwrap();
    cmd.args(["--json-errors", "build"])
        .arg(&file)
        .args(["-o"])
        .arg(&out)
        .args(["--emit-vcs"])
        .arg(&vcs)
        .args(["--compiler-mode", "strict"]);
    let output = cmd.assert().failure().get_output().stdout.clone();
    let v: Value = serde_json::from_slice(&output).expect("json");
    assert_single_json_error(&v, "C104", "build");
}

#[test]
fn strict_compiler_mode_rejects_invalid_lockfile_digest_with_c102() {
    let src = r#"
        function main() -> Int { 0 }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("strict_mode_lock_digest.clear");
    fs::write(&file, src).expect("write");
    write_minimal_strict_trust_policy(tmp.path());
    write_minimal_strict_host_profile(tmp.path());
    write_minimal_strict_package_metadata(tmp.path());
    write_minimal_strict_package_abi(tmp.path());
    fs::write(
        tmp.path().join("clg.lock.json"),
        r#"{
  "schema_version": 0,
  "dependencies": [
    { "name": "std::core", "version": "1.0.0", "digest": "sha256:ABCDEF" }
  ]
}"#,
    )
    .expect("write lockfile");
    let out = tmp.path().join("out.wasm");
    let vcs = tmp.path().join("out.vc.json");

    let mut cmd = Command::cargo_bin("clg").unwrap();
    cmd.args(["--json-errors", "build"])
        .arg(&file)
        .args(["-o"])
        .arg(&out)
        .args(["--emit-vcs"])
        .arg(&vcs)
        .args(["--compiler-mode", "strict"]);
    let output = cmd.assert().failure().get_output().stdout.clone();
    let v: Value = serde_json::from_slice(&output).expect("json");
    assert_single_json_error(&v, "C102", "build");
}

#[test]
fn strict_compiler_mode_rejects_lockfile_package_digest_mismatch_with_c102() {
    let src = r#"
        function main() -> Int { 0 }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp
        .path()
        .join("strict_mode_lockfile_package_digest_mismatch.clear");
    fs::write(&file, src).expect("write");
    write_minimal_strict_trust_policy(tmp.path());
    write_minimal_strict_host_profile(tmp.path());
    fs::write(
        tmp.path().join("clg.lock.json"),
        r#"{
  "schema_version": 0,
  "dependencies": [
    {
      "name": "std::core",
      "version": "1.0.0",
      "digest": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
    }
  ]
}"#,
    )
    .expect("write lockfile");
    fs::write(
        tmp.path().join("clg.package-metadata.json"),
        r#"{
  "schema_version": 0,
  "packages": [
    {
      "name": "std::core",
      "version": "1.0.0",
      "digest": "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
      "artifact": { "format": "wasm", "path": "store/std-core-1.0.0.wasm" },
      "abi_id": "abi:std::core:1.0.0"
    }
  ]
}"#,
    )
    .expect("write strict package metadata");
    fs::write(
        tmp.path().join("clg.package-abi.json"),
        r#"{
  "schema_version": 0,
  "contracts": [
    {
      "abi_id": "abi:std::core:1.0.0",
      "package": "std::core",
      "version": "1.0.0",
      "imports": []
    }
  ]
}"#,
    )
    .expect("write strict package abi");
    let out = tmp.path().join("out.wasm");
    let vcs = tmp.path().join("out.vc.json");

    let mut cmd = Command::cargo_bin("clg").unwrap();
    cmd.args(["--json-errors", "build"])
        .arg(&file)
        .args(["-o"])
        .arg(&out)
        .args(["--emit-vcs"])
        .arg(&vcs)
        .args(["--compiler-mode", "strict"]);
    let output = cmd.assert().failure().get_output().stdout.clone();
    let v: Value = serde_json::from_slice(&output).expect("json");
    assert_single_json_error(&v, "C102", "build");
}

#[test]
fn strict_compiler_mode_requires_package_signatures_with_c103() {
    let src = r#"
        function main() -> Int { 0 }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp
        .path()
        .join("strict_mode_missing_package_signatures.clear");
    fs::write(&file, src).expect("write");
    write_minimal_strict_trust_policy(tmp.path());
    write_minimal_strict_host_profile(tmp.path());
    fs::write(
        tmp.path().join("clg.lock.json"),
        r#"{
  "schema_version": 0,
  "dependencies": [
    {
      "name": "std::core",
      "version": "1.0.0",
      "digest": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
    }
  ]
}"#,
    )
    .expect("write lockfile");
    fs::write(
        tmp.path().join("clg.package-metadata.json"),
        r#"{
  "schema_version": 0,
  "packages": [
    {
      "name": "std::core",
      "version": "1.0.0",
      "digest": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "artifact": { "format": "wasm", "path": "store/std-core-1.0.0.wasm" },
      "abi_id": "abi:std::core:1.0.0"
    }
  ]
}"#,
    )
    .expect("write strict package metadata");
    fs::write(
        tmp.path().join("clg.package-abi.json"),
        r#"{
  "schema_version": 0,
  "contracts": [
    {
      "abi_id": "abi:std::core:1.0.0",
      "package": "std::core",
      "version": "1.0.0",
      "imports": []
    }
  ]
}"#,
    )
    .expect("write strict package abi");
    let out = tmp.path().join("out.wasm");
    let vcs = tmp.path().join("out.vc.json");

    let mut cmd = Command::cargo_bin("clg").unwrap();
    cmd.args(["--json-errors", "build"])
        .arg(&file)
        .args(["-o"])
        .arg(&out)
        .args(["--emit-vcs"])
        .arg(&vcs)
        .args(["--compiler-mode", "strict"]);
    let output = cmd.assert().failure().get_output().stdout.clone();
    let v: Value = serde_json::from_slice(&output).expect("json");
    assert_single_json_error(&v, "C103", "build");
}

