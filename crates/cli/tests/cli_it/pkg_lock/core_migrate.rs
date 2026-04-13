#[test]
fn pkg_migrate_manifest_generates_project_manifest_from_existing_lock_roots() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    fs::write(
        root.join("clg.package-metadata.json"),
        r#"{
  "schema_version": 1,
  "packages": [
    {
      "name": "std::core",
      "version": "1.0.0",
      "digest": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "artifact": { "format": "wasm", "path": "store/std-core.wasm" },
      "abi_id": "abi:std::core:1.0.0"
    }
  ]
}"#,
    )
    .expect("write metadata");
    fs::write(
        root.join("clg.lock.json"),
        r#"{
  "schema_version": 1,
  "resolver_version": 1,
  "roots": [
    {
      "name": "example-app",
      "dependencies": [
        { "name": "std::core", "requirement": "^1.0.0" }
      ]
    }
  ],
  "packages": []
}"#,
    )
    .expect("write lockfile");

    Command::cargo_bin("clg")
        .unwrap()
        .args(["pkg", "migrate-manifest", "--root"])
        .arg(root)
        .assert()
        .success();

    let manifest_bytes = fs::read(root.join("clg.project.json")).expect("read project manifest");
    let manifest: Value = serde_json::from_slice(&manifest_bytes).expect("project manifest json");
    assert_eq!(manifest["schema_version"], Value::from(1));
    assert_eq!(manifest["project"]["name"], Value::from("example-app"));
    assert_eq!(
        manifest["project"]["clg_version"],
        Value::from(current_clg_version_requirement())
    );
    assert_eq!(manifest["project"]["entry"], Value::from("main.clear"));
    assert_eq!(manifest["dependencies"][0]["name"], Value::from("std::core"));
    assert_eq!(
        manifest["dependencies"][0]["requirement"],
        Value::from("^1.0.0")
    );
    assert_eq!(
        manifest["release_defaults"]["advisory_as_of"],
        Value::from("REQUIRED_RFC3339_UTC")
    );
    assert_eq!(
        manifest["release_defaults"]["key_id"],
        Value::from("REQUIRED_KEY_ID")
    );
}

#[test]
fn pkg_migrate_manifest_reports_c109_when_manifest_already_exists() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    fs::write(
        root.join("clg.package-metadata.json"),
        r#"{"schema_version":1,"packages":[]}"#,
    )
    .expect("write metadata");
    fs::write(
        root.join("clg.project.json"),
        r#"{"schema_version":1,"project":{"name":"existing"},"dependencies":[],"release_defaults":{"advisory_as_of":"x","key_id":"y","out_dir":"out","trust_policy":"trust-policy.json"}}"#,
    )
    .expect("write existing manifest");

    let output = Command::cargo_bin("clg")
        .unwrap()
        .args(["--json-errors", "pkg", "migrate-manifest", "--root"])
        .arg(root)
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let v: Value = serde_json::from_slice(&output).expect("json");
    assert_eq!(v.get("ok").and_then(|b| b.as_bool()), Some(false));
    let errs = v
        .get("errors")
        .and_then(|e| e.as_array())
        .expect("errors array");
    assert_eq!(errs.len(), 1);
    let e0 = &errs[0];
    assert_eq!(e0.get("code").and_then(|s| s.as_str()), Some("C109"));
    assert_eq!(e0.get("stage").and_then(|s| s.as_str()), Some("build"));
}
