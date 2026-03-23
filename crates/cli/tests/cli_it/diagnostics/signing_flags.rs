#[test]
fn strict_compiler_mode_allows_program_without_assumptions() {
    let src = r#"
        pure function id(x: Int) -> Int { x }
        function main() -> Int { id(1) }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("strict_mode_no_assumptions.clear");
    fs::write(&file, src).expect("write");
    write_minimal_strict_preflight_files(tmp.path());
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
fn trust_anchor_checker_versions_require_sign_with_c032() {
    let src = r#"
        function main() -> Int { 0 }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("trust_versions_require_sign.clear");
    fs::write(&file, src).expect("write");
    let out = tmp.path().join("out.wasm");

    let mut cmd = Command::cargo_bin("clg").unwrap();
    cmd.args(["--json-errors", "build"])
        .arg(&file)
        .args(["-o"])
        .arg(&out)
        .args([
            "--lean-checker-version",
            "4.14.0",
            "--coq-checker-version",
            "8.19.2",
        ]);
    let output = cmd.assert().failure().get_output().stdout.clone();
    let v: Value = serde_json::from_slice(&output).expect("json");
    assert_single_json_error(&v, "C032", "build");
}

#[test]
fn assurance_manifest_out_requires_sign_with_c034() {
    let src = r#"
        function main() -> Int { 0 }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("manifest_requires_sign.clear");
    fs::write(&file, src).expect("write");
    let out = tmp.path().join("out.wasm");
    let manifest = tmp.path().join("assurance.json");

    let mut cmd = Command::cargo_bin("clg").unwrap();
    cmd.args(["--json-errors", "build"])
        .arg(&file)
        .args(["-o"])
        .arg(&out)
        .args(["--assurance-manifest-out"])
        .arg(&manifest);
    let output = cmd.assert().failure().get_output().stdout.clone();
    let v: Value = serde_json::from_slice(&output).expect("json");
    assert_single_json_error(&v, "C034", "build");
}

#[test]
fn trust_anchor_checker_versions_must_be_paired_with_c032() {
    let src = r#"
        function main() -> Int { 0 }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("trust_versions_pair_required.clear");
    fs::write(&file, src).expect("write");
    let out = tmp.path().join("out.wasm");
    let vcs = tmp.path().join("out.vc.json");
    let sig = tmp.path().join("out.sig.json");
    let key = tmp.path().join("unused.json");

    let mut cmd = Command::cargo_bin("clg").unwrap();
    cmd.args(["--json-errors", "build"])
        .arg(&file)
        .args(["-o"])
        .arg(&out)
        .args(["--emit-vcs"])
        .arg(&vcs)
        .args(["--sign", "--key-id", "demo", "--scope", "both"])
        .args(["--key"])
        .arg(&key)
        .args(["--sig-out"])
        .arg(&sig)
        .args(["--lean-checker-version", "4.14.0"]);
    let output = cmd.assert().failure().get_output().stdout.clone();
    let v: Value = serde_json::from_slice(&output).expect("json");
    assert_single_json_error(&v, "C032", "build");
}

#[test]
fn trust_anchor_checker_versions_must_be_non_empty_with_c032() {
    let src = r#"
        function main() -> Int { 0 }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("trust_versions_non_empty_required.clear");
    fs::write(&file, src).expect("write");
    let out = tmp.path().join("out.wasm");
    let vcs = tmp.path().join("out.vc.json");
    let sig = tmp.path().join("out.sig.json");
    let key = tmp.path().join("unused.json");

    let mut cmd = Command::cargo_bin("clg").unwrap();
    cmd.args(["--json-errors", "build"])
        .arg(&file)
        .args(["-o"])
        .arg(&out)
        .args(["--emit-vcs"])
        .arg(&vcs)
        .args(["--sign", "--key-id", "demo", "--scope", "both"])
        .args(["--key"])
        .arg(&key)
        .args(["--sig-out"])
        .arg(&sig)
        .args([
            "--lean-checker-version",
            "",
            "--coq-checker-version",
            "8.19.2",
        ]);
    let output = cmd.assert().failure().get_output().stdout.clone();
    let v: Value = serde_json::from_slice(&output).expect("json");
    assert_single_json_error(&v, "C032", "build");
}
