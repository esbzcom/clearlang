use super::*;

#[test]
fn pkg_lock_generate_writes_sorted_pins_from_metadata() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    fs::write(
        root.join("clg.package-metadata.json"),
        r#"{
  "schema_version": 1,
  "packages": [
    {
      "name": "std::host",
      "version": "1.0.0",
      "digest": "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
      "artifact": { "format": "wasm", "path": "store/std-host.wasm" },
      "abi_id": "abi:std::host:1.0.0"
    },
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

    Command::cargo_bin("clg")
        .unwrap()
        .args(["pkg", "lock", "--generate", "--root"])
        .arg(root)
        .assert()
        .success();

    let lock_bytes = fs::read(root.join("clg.lock.json")).expect("read lockfile");
    let lock_text = String::from_utf8(lock_bytes.clone()).expect("utf8");
    assert!(lock_text.starts_with("{\"packages\":"));
    assert!(lock_text.ends_with("}\n"));
    let v: Value = serde_json::from_slice(&lock_bytes).expect("lockfile json");
    assert_eq!(v["schema_version"], Value::from(1));
    assert_eq!(v["resolver_version"], Value::from(1));
    let roots = v["roots"].as_array().expect("roots array");
    assert_eq!(roots.len(), 1);
    let root_deps = roots[0]["dependencies"]
        .as_array()
        .expect("root dependencies");
    assert_eq!(root_deps.len(), 2);
    assert_eq!(root_deps[0]["name"], Value::String("std::core".to_string()));
    assert_eq!(root_deps[1]["name"], Value::String("std::host".to_string()));
    let packages = v["packages"].as_array().expect("packages array");
    assert_eq!(packages.len(), 2);
    assert_eq!(
        packages[0]["id"],
        Value::String("std::core@1.0.0".to_string())
    );
    assert_eq!(
        packages[1]["id"],
        Value::String("std::host@1.0.0".to_string())
    );
}

#[test]
fn pkg_lock_generate_prints_canonical_hash() {
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
      "abi_id": "abi:std::core:1.0.0"
    }
  ]
}"#,
    )
    .expect("write metadata");

    Command::cargo_bin("clg")
        .unwrap()
        .args(["pkg", "lock", "--generate", "--root"])
        .arg(root)
        .assert()
        .success()
        .stdout(predicate::str::contains("[sha256:"));
}

#[test]
fn pkg_lock_generate_fails_if_lock_already_exists() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    fs::write(
        root.join("clg.package-metadata.json"),
        r#"{"schema_version":1,"packages":[]}"#,
    )
    .expect("write metadata");
    fs::write(
        root.join("clg.lock.json"),
        r#"{"schema_version":0,"dependencies":[]}"#,
    )
    .expect("write lockfile");

    let output = Command::cargo_bin("clg")
        .unwrap()
        .args(["--json-errors", "pkg", "lock", "--generate", "--root"])
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
    assert_eq!(e0.get("code").and_then(|s| s.as_str()), Some("C027"));
    assert_eq!(e0.get("stage").and_then(|s| s.as_str()), Some("build"));
}

#[test]
fn pkg_lock_update_fails_if_lock_missing() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    fs::write(
        root.join("clg.package-metadata.json"),
        r#"{"schema_version":1,"packages":[]}"#,
    )
    .expect("write metadata");

    let output = Command::cargo_bin("clg")
        .unwrap()
        .args(["--json-errors", "pkg", "lock", "--update", "--root"])
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
    assert_eq!(e0.get("code").and_then(|s| s.as_str()), Some("C027"));
    assert_eq!(e0.get("stage").and_then(|s| s.as_str()), Some("build"));
}

#[test]
fn pkg_lock_replay_with_identical_inputs_is_byte_identical() {
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
      "abi_id": "abi:std::core:1.0.0"
    },
    {
      "name": "std::host",
      "version": "1.0.0",
      "digest": "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
      "abi_id": "abi:std::host:1.0.0"
    }
  ]
}"#,
    )
    .expect("write metadata");

    Command::cargo_bin("clg")
        .unwrap()
        .args(["pkg", "lock", "--generate", "--root"])
        .arg(root)
        .assert()
        .success();
    let first = fs::read(root.join("clg.lock.json")).expect("read first lockfile bytes");

    Command::cargo_bin("clg")
        .unwrap()
        .args(["pkg", "lock", "--update", "--root"])
        .arg(root)
        .assert()
        .success();
    let second = fs::read(root.join("clg.lock.json")).expect("read second lockfile bytes");

    assert_eq!(first, second, "lockfile replay bytes must be identical");
}

#[test]
fn pkg_lock_generate_derives_roots_from_unreferenced_packages() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    fs::write(
        root.join("clg.package-metadata.json"),
        r#"{
  "schema_version": 1,
  "packages": [
    {
      "name": "app::entry",
      "version": "1.0.0",
      "digest": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "abi_id": "abi:app::entry:1.0.0",
      "dependencies": [
        { "name": "lib::core", "requirement": "^1.0.0" }
      ]
    },
    {
      "name": "lib::core",
      "version": "1.0.0",
      "digest": "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
      "abi_id": "abi:lib::core:1.0.0"
    }
  ]
}"#,
    )
    .expect("write metadata");

    Command::cargo_bin("clg")
        .unwrap()
        .args(["pkg", "lock", "--generate", "--root"])
        .arg(root)
        .assert()
        .success();

    let lock_bytes = fs::read(root.join("clg.lock.json")).expect("read lockfile");
    let v: Value = serde_json::from_slice(&lock_bytes).expect("lockfile json");
    let roots = v["roots"].as_array().expect("roots array");
    assert_eq!(roots.len(), 1);
    let deps = roots[0]["dependencies"]
        .as_array()
        .expect("root dependencies");
    assert_eq!(deps.len(), 1);
    assert_eq!(deps[0]["name"], Value::String("app::entry".to_string()));
}

#[test]
fn pkg_lock_generate_reports_c112_for_transitive_cycle() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    fs::write(
        root.join("clg.package-metadata.json"),
        r#"{
  "schema_version": 1,
  "packages": [
    {
      "name": "z::pkg",
      "version": "1.0.0",
      "digest": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "abi_id": "abi:z::pkg:1.0.0",
      "dependencies": [{ "name": "y::pkg", "requirement": "^1.0.0" }]
    },
    {
      "name": "y::pkg",
      "version": "1.0.0",
      "digest": "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
      "abi_id": "abi:y::pkg:1.0.0",
      "dependencies": [{ "name": "x::pkg", "requirement": "^1.0.0" }]
    },
    {
      "name": "x::pkg",
      "version": "1.0.0",
      "digest": "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
      "abi_id": "abi:x::pkg:1.0.0",
      "dependencies": [{ "name": "z::pkg", "requirement": "^1.0.0" }]
    }
  ]
}"#,
    )
    .expect("write metadata");

    let output = Command::cargo_bin("clg")
        .unwrap()
        .args(["--json-errors", "pkg", "lock", "--generate", "--root"])
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
    assert_eq!(e0.get("code").and_then(|s| s.as_str()), Some("C112"));
    assert_eq!(e0.get("stage").and_then(|s| s.as_str()), Some("build"));
}

#[test]
fn pkg_lock_generate_reports_c113_for_unsatisfiable_semver_constraints() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    fs::write(
        root.join("clg.package-metadata.json"),
        r#"{
  "schema_version": 1,
  "packages": [
    {
      "name": "app::entry",
      "version": "1.0.0",
      "digest": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "abi_id": "abi:app::entry:1.0.0",
      "dependencies": [{ "name": "lib::core", "requirement": "^2.0.0" }]
    },
    {
      "name": "lib::core",
      "version": "1.0.0",
      "digest": "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
      "abi_id": "abi:lib::core:1.0.0"
    }
  ]
}"#,
    )
    .expect("write metadata");

    let output = Command::cargo_bin("clg")
        .unwrap()
        .args(["--json-errors", "pkg", "lock", "--generate", "--root"])
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
    assert_eq!(e0.get("code").and_then(|s| s.as_str()), Some("C113"));
    assert_eq!(e0.get("stage").and_then(|s| s.as_str()), Some("build"));
}

#[test]
fn pkg_lock_generate_reports_c115_for_deny_advisory() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    fs::write(
        root.join("clg.package-metadata.json"),
        r#"{
  "schema_version": 1,
  "packages": [
    {
      "name": "app::entry",
      "version": "1.0.0",
      "digest": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "abi_id": "abi:app::entry:1.0.0",
      "dependencies": [{ "name": "lib::core", "requirement": "^1.0.0" }]
    },
    {
      "name": "lib::core",
      "version": "1.1.0",
      "digest": "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
      "abi_id": "abi:lib::core:1.1.0"
    }
  ]
}"#,
    )
    .expect("write metadata");
    fs::write(
        root.join("clg.advisories.json"),
        r#"{
  "schema_version": 1,
  "advisories": [
    {
      "id": "ADV-100",
      "package": "lib::core",
      "affected": "^1.0.0",
      "severity": "high",
      "action": "deny",
      "issued_at": "2026-01-01T00:00:00Z",
      "expires_at": "2027-01-01T00:00:00Z"
    }
  ]
}"#,
    )
    .expect("write advisories");

    let output = Command::cargo_bin("clg")
        .unwrap()
        .args(["--json-errors", "pkg", "lock", "--generate", "--root"])
        .arg(root)
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let v: Value = serde_json::from_slice(&output).expect("json");
    let errs = v
        .get("errors")
        .and_then(|e| e.as_array())
        .expect("errors array");
    assert_eq!(errs.len(), 1);
    let e0 = &errs[0];
    assert_eq!(e0.get("code").and_then(|s| s.as_str()), Some("C115"));
}

#[test]
fn pkg_lock_generate_reports_c116_for_force_upgrade_without_safe_version() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    fs::write(
        root.join("clg.package-metadata.json"),
        r#"{
  "schema_version": 1,
  "packages": [
    {
      "name": "app::entry",
      "version": "1.0.0",
      "digest": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "abi_id": "abi:app::entry:1.0.0",
      "dependencies": [{ "name": "lib::core", "requirement": "^1.0.0" }]
    },
    {
      "name": "lib::core",
      "version": "1.1.0",
      "digest": "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
      "abi_id": "abi:lib::core:1.1.0"
    }
  ]
}"#,
    )
    .expect("write metadata");
    fs::write(
        root.join("clg.advisories.json"),
        r#"{
  "schema_version": 1,
  "advisories": [
    {
      "id": "ADV-200",
      "package": "lib::core",
      "affected": "^1.0.0",
      "severity": "critical",
      "action": "force_upgrade",
      "minimum_safe_version": "2.0.0",
      "issued_at": "2026-01-01T00:00:00Z",
      "expires_at": "2027-01-01T00:00:00Z"
    }
  ]
}"#,
    )
    .expect("write advisories");

    let output = Command::cargo_bin("clg")
        .unwrap()
        .args(["--json-errors", "pkg", "lock", "--generate", "--root"])
        .arg(root)
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let v: Value = serde_json::from_slice(&output).expect("json");
    let errs = v
        .get("errors")
        .and_then(|e| e.as_array())
        .expect("errors array");
    assert_eq!(errs.len(), 1);
    let e0 = &errs[0];
    assert_eq!(e0.get("code").and_then(|s| s.as_str()), Some("C116"));
}

#[test]
fn pkg_lock_generate_reports_c117_for_malformed_advisory_input() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    fs::write(
        root.join("clg.package-metadata.json"),
        r#"{
  "schema_version": 1,
  "packages": [
    {
      "name": "app::entry",
      "version": "1.0.0",
      "digest": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "abi_id": "abi:app::entry:1.0.0"
    }
  ]
}"#,
    )
    .expect("write metadata");
    fs::write(
        root.join("clg.advisories.json"),
        r#"{"schema_version":1,"advisories":[{"id":"ADV-BAD"}]}"#,
    )
    .expect("write advisories");

    let output = Command::cargo_bin("clg")
        .unwrap()
        .args(["--json-errors", "pkg", "lock", "--generate", "--root"])
        .arg(root)
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let v: Value = serde_json::from_slice(&output).expect("json");
    let errs = v
        .get("errors")
        .and_then(|e| e.as_array())
        .expect("errors array");
    assert_eq!(errs.len(), 1);
    let e0 = &errs[0];
    assert_eq!(e0.get("code").and_then(|s| s.as_str()), Some("C117"));
}
