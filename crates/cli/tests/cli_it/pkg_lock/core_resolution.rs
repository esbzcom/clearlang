#[test]
fn pkg_lock_and_build_accept_same_schema_v1_metadata_contract() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    fs::write(
        root.join("clg.package-metadata.json"),
        r#"{
  "schema_version": 1,
  "packages": [
    {
      "name": "extpkg",
      "version": "1.0.0",
      "digest": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "artifact": { "format": "wasm", "path": "store/extpkg.wasm" },
      "abi_id": "abi:extpkg:1.0.0"
    }
  ]
}"#,
    )
    .expect("write metadata");
    fs::write(
        root.join("clg.package-abi.json"),
        r#"{
  "schema_version": 0,
  "contracts": [
    {
      "abi_id": "abi:extpkg:1.0.0",
      "package": "extpkg",
      "version": "1.0.0",
      "imports": []
    }
  ]
}"#,
    )
    .expect("write abi");
    let main_path = root.join("main.clear");
    fs::write(&main_path, "function main() -> Int { 0 }").expect("write main");

    Command::cargo_bin("clg")
        .unwrap()
        .args(["pkg", "lock", "--generate", "--root"])
        .arg(root)
        .assert()
        .success();

    Command::cargo_bin("clg")
        .unwrap()
        .args(["build"])
        .arg(&main_path)
        .args(["-o"])
        .arg(root.join("out.wasm"))
        .assert()
        .success();
}

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
fn pkg_lock_generate_emits_exact_pins_and_matching_resolved_graph_identity() {
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
      "artifact": { "format": "wasm", "path": "store/app-entry.wasm" },
      "abi_id": "abi:app::entry:1.0.0",
      "dependencies": [{ "name": "lib::core", "requirement": "^1.0.0" }]
    },
    {
      "name": "lib::core",
      "version": "1.2.0",
      "digest": "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
      "artifact": { "format": "wasm", "path": "store/lib-core.wasm" },
      "abi_id": "abi:lib::core:1.2.0"
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
    let lock: Value = serde_json::from_slice(&lock_bytes).expect("lockfile json");
    assert_eq!(lock["schema_version"], Value::from(1));
    assert_eq!(lock["resolver_version"], Value::from(1));

    let lock_packages = lock["packages"].as_array().expect("lock packages");
    assert_eq!(lock_packages.len(), 2);
    assert!(
        lock_packages.iter().any(|pkg| {
            pkg["id"] == Value::String("app::entry@1.0.0".to_string())
                && pkg["name"] == Value::String("app::entry".to_string())
                && pkg["version"] == Value::String("1.0.0".to_string())
                && pkg["digest"]
                    == Value::String(
                        "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                            .to_string()
                    )
        }),
        "lockfile must pin exact app::entry identity (name/version/digest)"
    );
    assert!(
        lock_packages.iter().any(|pkg| {
            pkg["id"] == Value::String("lib::core@1.2.0".to_string())
                && pkg["name"] == Value::String("lib::core".to_string())
                && pkg["version"] == Value::String("1.2.0".to_string())
                && pkg["digest"]
                    == Value::String(
                        "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
                            .to_string()
                    )
        }),
        "lockfile must pin exact lib::core identity (name/version/digest)"
    );

    let mut lock_deps: std::collections::BTreeMap<String, Vec<String>> =
        std::collections::BTreeMap::new();
    for pkg in lock_packages {
        let id = pkg["id"].as_str().expect("lock id").to_string();
        let mut deps = pkg["dependencies"]
            .as_array()
            .expect("lock deps")
            .iter()
            .map(|dep| dep.as_str().expect("dep id").to_string())
            .collect::<Vec<_>>();
        deps.sort();
        lock_deps.insert(id, deps);
    }

    let graph_bytes = fs::read(root.join("clg.resolved-graph.json")).expect("read graph");
    let graph_hash =
        fs::read_to_string(root.join("clg.resolved-graph.sha256")).expect("read graph hash");
    let graph_canonical = &graph_bytes[..graph_bytes.len() - 1];
    assert_eq!(
        sha256_hex(graph_canonical),
        graph_hash.trim(),
        "resolved graph sidecar hash must match canonical graph bytes"
    );

    let graph: Value = serde_json::from_slice(&graph_bytes).expect("graph json");
    assert_eq!(graph["schema_version"], lock["schema_version"]);
    assert_eq!(graph["resolver_version"], lock["resolver_version"]);
    assert_eq!(
        graph["roots"], lock["roots"],
        "resolved graph roots must match lockfile roots"
    );

    let mut graph_deps: std::collections::BTreeMap<String, Vec<String>> =
        std::collections::BTreeMap::new();
    for pkg in graph["packages"].as_array().expect("graph packages") {
        let id = pkg["id"].as_str().expect("graph id").to_string();
        let mut deps = pkg["dependencies"]
            .as_array()
            .expect("graph deps")
            .iter()
            .map(|dep| dep.as_str().expect("dep id").to_string())
            .collect::<Vec<_>>();
        deps.sort();
        graph_deps.insert(id, deps);
    }
    assert_eq!(
        graph_deps, lock_deps,
        "resolved graph package/dependency identity must match lockfile identity"
    );
}

#[test]
fn pkg_lock_generate_prefers_project_manifest_dependencies_when_present() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    fs::write(
        root.join("clg.project.json"),
        with_current_clg_version_requirement(
            r#"{
  "schema_version": 1,
  "project": {
    "name": "example-app",
    "description": "Example project",
    "version": "1.0.0",
    "clg_version": "^0.1.0",
    "entry": "main.clear",
    "website": "https://example.com",
    "contact": {
      "name": "Example Maintainer",
      "email": "maintainer@example.com"
    }
  },
  "dependencies": [
    { "name": "app::entry", "requirement": "^1.0.0" }
  ],
  "release_defaults": {
    "advisory_as_of": "2026-03-31T00:00:00Z",
    "key_id": "release-2026q2",
    "out_dir": "out/release",
    "trust_policy": "trust-policy.json"
  }
}"#,
        ),
    )
    .expect("write project manifest");
    fs::write(
        root.join("clg.package-metadata.json"),
        r#"{
  "schema_version": 1,
  "packages": [
    {
      "name": "app::entry",
      "version": "1.0.0",
      "digest": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "artifact": { "format": "wasm", "path": "store/app-entry.wasm" },
      "abi_id": "abi:app::entry:1.0.0",
      "dependencies": [{ "name": "lib::core", "requirement": "^1.0.0" }]
    },
    {
      "name": "lib::core",
      "version": "1.2.0",
      "digest": "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
      "artifact": { "format": "wasm", "path": "store/lib-core.wasm" },
      "abi_id": "abi:lib::core:1.2.0"
    },
    {
      "name": "extra::pkg",
      "version": "1.0.0",
      "digest": "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
      "artifact": { "format": "wasm", "path": "store/extra.wasm" },
      "abi_id": "abi:extra::pkg:1.0.0"
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
    assert_eq!(roots[0]["name"], Value::String("example-app".to_string()));
    let deps = roots[0]["dependencies"]
        .as_array()
        .expect("root dependencies");
    assert_eq!(deps.len(), 1);
    assert_eq!(deps[0]["name"], Value::String("app::entry".to_string()));
    assert_eq!(deps[0]["requirement"], Value::String("^1.0.0".to_string()));

    let packages = v["packages"].as_array().expect("packages array");
    let mut ids: Vec<String> = packages
        .iter()
        .filter_map(|pkg| pkg.get("id").and_then(Value::as_str).map(ToOwned::to_owned))
        .collect();
    ids.sort();
    assert_eq!(ids, vec!["app::entry@1.0.0", "lib::core@1.2.0"]);
}

#[test]
fn pkg_lock_generate_rejects_invalid_project_manifest_dependencies_with_c027() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    fs::write(
        root.join("clg.project.json"),
        with_current_clg_version_requirement(
            r#"{
  "schema_version": 1,
  "project": {
    "name": "example-app",
    "description": "Example project",
    "version": "1.0.0",
    "clg_version": "^0.1.0",
    "entry": "main.clear",
    "website": "https://example.com",
    "contact": {
      "name": "Example Maintainer",
      "email": "maintainer@example.com"
    }
  },
  "dependencies": [
    { "name": "std::core", "requirement": "not-semver" }
  ],
  "release_defaults": {
    "advisory_as_of": "2026-03-31T00:00:00Z",
    "key_id": "release-2026q2",
    "out_dir": "out/release",
    "trust_policy": "trust-policy.json"
  }
}"#,
        ),
    )
    .expect("write project manifest");
    fs::write(
        root.join("clg.package-metadata.json"),
        r#"{"schema_version":1,"packages":[]}"#,
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
    assert_eq!(e0.get("code").and_then(|s| s.as_str()), Some("C027"));
    assert_eq!(e0.get("stage").and_then(|s| s.as_str()), Some("build"));
    assert!(
        e0.get("message")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .contains("invalid requirement"),
        "expected invalid requirement diagnostics"
    );
}

#[test]
fn pkg_lock_generate_rejects_incompatible_project_clg_version_with_c027() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    fs::write(
        root.join("clg.project.json"),
        with_current_clg_version_requirement(
            r#"{
  "schema_version": 1,
  "project": {
    "name": "example-app",
    "description": "Example project",
    "version": "1.0.0",
    "clg_version": "^999.0.0",
    "entry": "main.clear",
    "website": "https://example.com",
    "contact": {
      "name": "Example Maintainer",
      "email": "maintainer@example.com"
    }
  },
  "dependencies": [],
  "release_defaults": {
    "advisory_as_of": "2026-03-31T00:00:00Z",
    "key_id": "release-2026q2",
    "out_dir": "out/release",
    "trust_policy": "trust-policy.json"
  }
}"#,
        ),
    )
    .expect("write project manifest");
    fs::write(
        root.join("clg.package-metadata.json"),
        r#"{"schema_version":1,"packages":[]}"#,
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
    assert_eq!(e0.get("code").and_then(|s| s.as_str()), Some("C027"));
    assert!(
        e0.get("message")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .contains("project.clg_version"),
        "expected clg_version compatibility diagnostics"
    );
}

