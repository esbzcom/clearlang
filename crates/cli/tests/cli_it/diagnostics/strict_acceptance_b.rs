#[test]
fn strict_acceptance_import_map_artifact_is_deterministic_across_identical_runs() {
    let src = r#"
        function main() -> Int {
            std::str::len("abc") + std::bytes::len(std::bytes::from_string("xy"))
        }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp
        .path()
        .join("strict_acceptance_import_map_determinism.clear");
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
        },
        {
          "symbol": "std::bytes::len",
          "effect": "pure",
          "params": ["Bytes"],
          "ret": "Int",
          "capability": null
        }
      ]"#,
    );

    let out1 = tmp.path().join("run1.wasm");
    let vcs1 = tmp.path().join("run1.vc.json");
    Command::cargo_bin("clg")
        .unwrap()
        .args(["build"])
        .arg(&file)
        .args(["-o"])
        .arg(&out1)
        .args(["--emit-vcs"])
        .arg(&vcs1)
        .args(["--compiler-mode", "strict"])
        .args(["--std-core-link-mode", "precompiled"])
        .assert()
        .success();

    let artifact1 = strict_import_map_artifact_path(&out1);
    let bytes1 = fs::read(&artifact1).expect("read run1 strict import-map artifact");
    let hash1 = hex::encode(Sha256::digest(bytes1.as_slice()));

    let out2 = tmp.path().join("run2.wasm");
    let vcs2 = tmp.path().join("run2.vc.json");
    Command::cargo_bin("clg")
        .unwrap()
        .args(["build"])
        .arg(&file)
        .args(["-o"])
        .arg(&out2)
        .args(["--emit-vcs"])
        .arg(&vcs2)
        .args(["--compiler-mode", "strict"])
        .args(["--std-core-link-mode", "precompiled"])
        .assert()
        .success();

    let artifact2 = strict_import_map_artifact_path(&out2);
    let bytes2 = fs::read(&artifact2).expect("read run2 strict import-map artifact");
    let hash2 = hex::encode(Sha256::digest(bytes2.as_slice()));

    assert_eq!(
        bytes1, bytes2,
        "canonical import-map artifact bytes must match"
    );
    assert_eq!(
        hash1, hash2,
        "canonical import-map artifact hash must match"
    );

    let artifact_json: Value = serde_json::from_slice(&bytes1).expect("artifact json");
    assert_eq!(
        artifact_json
            .get("kind")
            .and_then(|value| value.as_str())
            .unwrap_or_default(),
        "clg.strict_direct_dependency_import_map.v0"
    );
    let imports = artifact_json
        .get("imports")
        .and_then(|value| value.as_array())
        .expect("artifact imports");
    assert_eq!(imports.len(), 2);
    assert_eq!(
        imports[0]
            .get("symbol")
            .and_then(|value| value.as_str())
            .unwrap_or_default(),
        "std::bytes::len"
    );
    assert_eq!(
        imports[1]
            .get("symbol")
            .and_then(|value| value.as_str())
            .unwrap_or_default(),
        "std::str::len"
    );
}

#[test]
fn strict_acceptance_diagnostics_order_is_deterministic_across_identical_runs() {
    let src = r#"
        function main() -> Int {
            std::str::len("abc") + std::bytes::len(std::bytes::from_string("xy"))
        }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp
        .path()
        .join("strict_acceptance_diagnostics_order_determinism.clear");
    fs::write(&file, src).expect("write");
    write_signed_strict_dependency_fixture(
        tmp.path(),
        r#"[
        {
          "symbol": "std::str::len",
          "effect": "pure",
          "params": ["String"],
          "ret": "Int",
          "capability": "std::crypto::hash"
        },
        {
          "symbol": "std::bytes::len",
          "effect": "pure",
          "params": ["Bytes"],
          "ret": "Int",
          "capability": "std::crypto::hash"
        }
      ]"#,
    );

    let run1 = run_strict_build_json_failure(tmp.path(), &file);
    let errs1 = assert_json_error_codes(&run1, "build", &["C106", "C106"]);
    let messages1 = errs1
        .iter()
        .map(|err| {
            err.get("message")
                .and_then(|value| value.as_str())
                .unwrap_or_default()
                .to_owned()
        })
        .collect::<Vec<_>>();

    let run2 = run_strict_build_json_failure(tmp.path(), &file);
    let errs2 = assert_json_error_codes(&run2, "build", &["C106", "C106"]);
    let messages2 = errs2
        .iter()
        .map(|err| {
            err.get("message")
                .and_then(|value| value.as_str())
                .unwrap_or_default()
                .to_owned()
        })
        .collect::<Vec<_>>();

    assert_eq!(
        messages1, messages2,
        "strict preflight diagnostics ordering must be deterministic across identical replays"
    );
}

