#[test]
fn strict_compiler_mode_requires_package_metadata_with_c104() {
    let src = r#"
        function main() -> Int { 0 }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("strict_mode_no_pkg_metadata.clear");
    fs::write(&file, src).expect("write");
    write_minimal_strict_lockfile(tmp.path());
    write_minimal_strict_trust_policy(tmp.path());
    write_minimal_strict_host_profile(tmp.path());
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
    assert_single_json_error(&v, "C104", "build");
}

#[test]
fn strict_compiler_mode_allows_signed_dependency_with_empty_abi_imports() {
    let src = r#"
        function main() -> Int { 0 }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("strict_mode_signed_dep_ok.clear");
    fs::write(&file, src).expect("write");
    write_signed_strict_dependency_fixture(tmp.path(), "[]");
    let out = tmp.path().join("out.wasm");
    let vcs = tmp.path().join("out.vc.json");

    let mut cmd = Command::cargo_bin("clg").unwrap();
    cmd.args(["build"])
        .arg(&file)
        .args(["-o"])
        .arg(&out)
        .args(["--emit-vcs"])
        .arg(&vcs)
        .args(["--compiler-mode", "strict"])
        .args(["--std-core-link-mode", "precompiled"]);
    cmd.assert().success();
}

#[test]
fn strict_compiler_mode_allows_linked_abi_symbol_from_strict_contract() {
    let src = r#"
        function main() -> Int { std::str::len("abc") }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("strict_mode_linked_abi_symbol.clear");
    fs::write(&file, src).expect("write");
    write_signed_strict_dependency_fixture(
        tmp.path(),
        r#"[
        {
          "symbol": "std::str::len",
          "effect": "pure",
          "params": ["String"],
          "ret": "Int",
          "capability": null
        }
      ]"#,
    );
    let out = tmp.path().join("out.wasm");
    let vcs = tmp.path().join("out.vc.json");

    let mut cmd = Command::cargo_bin("clg").unwrap();
    cmd.args(["build"])
        .arg(&file)
        .args(["-o"])
        .arg(&out)
        .args(["--emit-vcs"])
        .arg(&vcs)
        .args(["--compiler-mode", "strict"])
        .args(["--std-core-link-mode", "precompiled"]);
    cmd.assert().success();
}

#[test]
fn strict_compiler_mode_rejects_package_abi_mismatch_with_c105() {
    let src = r#"
        function main() -> Int { 0 }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("strict_mode_bad_pkg_abi_match.clear");
    fs::write(&file, src).expect("write");
    write_minimal_strict_lockfile(tmp.path());
    write_minimal_strict_trust_policy(tmp.path());
    write_minimal_strict_host_profile(tmp.path());
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
  "contracts": []
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
    assert_single_json_error(&v, "C105", "build");
}

#[test]
fn strict_compiler_mode_rejects_proof_strict_false_override_with_c030() {
    let src = r#"
        function main() -> Int { 0 }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("strict_mode_override_false.clear");
    fs::write(&file, src).expect("write");
    write_minimal_strict_preflight_files(tmp.path());
    let out = tmp.path().join("out.wasm");
    let vcs = tmp.path().join("out.vc.json");

    let mut cmd = Command::cargo_bin("clg").unwrap();
    cmd.args(["--json-errors", "build"])
        .arg(&file)
        .args(["-o"])
        .arg(&out)
        .args(["--emit-vcs"])
        .arg(&vcs)
        .args(["--compiler-mode", "strict", "--proof-strict=false"]);
    let output = cmd.assert().failure().get_output().stdout.clone();
    let v: Value = serde_json::from_slice(&output).expect("json");
    assert_single_json_error(&v, "C030", "build");
}

#[test]
fn strict_compiler_mode_rejects_assumed_surfaces_with_c033() {
    let src = r#"
        pure function check(a: Bytes, b: Bytes, x: U64, y: U64) -> Bool
            ensure { result == std::bytes::eq_ct(a, b) }
            ensure { (x & y) == x }
        {
            std::bytes::eq_ct(a, b)
        }
        function main() -> Int { 0 }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("strict_mode_assumed_surface.clear");
    fs::write(&file, src).expect("write");
    write_minimal_strict_preflight_files(tmp.path());
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
    assert_single_json_error(&v, "C033", "build");
}

