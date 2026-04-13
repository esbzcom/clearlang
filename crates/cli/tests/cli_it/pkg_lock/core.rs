use super::*;
use ed25519_dalek::{Signer, SigningKey};
use serde_json::json;
use sha2::{Digest, Sha256};

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn extract_first_sha256_from_stdout(stdout: &[u8]) -> String {
    let text = String::from_utf8(stdout.to_vec()).expect("stdout utf8");
    let marker = "[sha256:";
    let start = text.find(marker).expect("stdout contains sha256 marker") + marker.len();
    let rest = &text[start..];
    let end = rest.find(']').expect("sha256 marker closed");
    rest[..end].to_string()
}

fn canonicalize_json_value(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut keys: Vec<_> = map.keys().cloned().collect();
            keys.sort();
            let mut out = serde_json::Map::new();
            for key in keys {
                out.insert(
                    key.clone(),
                    canonicalize_json_value(map.get(key.as_str()).expect("key exists")),
                );
            }
            Value::Object(out)
        }
        Value::Array(items) => Value::Array(items.iter().map(canonicalize_json_value).collect()),
        _ => value.clone(),
    }
}

fn canonical_json_bytes(value: &Value) -> Vec<u8> {
    serde_json::to_vec(&canonicalize_json_value(value)).expect("serialize canonical json")
}

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

#[test]
fn pkg_lock_generate_reports_c109_when_legacy_metadata_model_is_present() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    fs::write(
        root.join("clg-packages.json"),
        r#"{"schema_version":1,"packages":[]}"#,
    )
    .expect("write legacy metadata");
    fs::write(
        root.join("clg.package-metadata.json"),
        r#"{"schema_version":1,"packages":[]}"#,
    )
    .expect("write canonical metadata");

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
    assert_eq!(e0.get("code").and_then(|s| s.as_str()), Some("C109"));
    assert_eq!(e0.get("stage").and_then(|s| s.as_str()), Some("build"));
    assert!(
        e0.get("message")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .contains("coexistence conflict"),
        "expected coexistence conflict diagnostics"
    );
}

#[test]
fn pkg_lock_update_reports_c109_when_manifest_roots_conflict_with_existing_lock() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    fs::write(
        root.join("clg.project.json"),
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
    { "name": "std::core", "requirement": "^1.0.0" }
  ],
  "release_defaults": {
    "advisory_as_of": "2026-03-31T00:00:00Z",
    "key_id": "release-2026q2",
    "out_dir": "out/release",
    "trust_policy": "trust-policy.json"
  }
}"#,
    )
    .expect("write project manifest");
    fs::write(
        root.join("clg.lock.json"),
        r#"{
  "schema_version": 1,
  "resolver_version": 1,
  "roots": [
    {
      "name": "example-app",
      "dependencies": [
        { "name": "std::host", "requirement": "^1.0.0" }
      ]
    }
  ],
  "packages": []
}"#,
    )
    .expect("write existing lockfile");
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
    assert_eq!(e0.get("code").and_then(|s| s.as_str()), Some("C109"));
    assert_eq!(e0.get("stage").and_then(|s| s.as_str()), Some("build"));
    assert!(
        e0.get("message")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .contains("manifest roots do not match existing lockfile roots"),
        "expected manifest-lock conflict diagnostics"
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
fn pkg_lock_generate_rejects_artifact_parent_traversal_with_c027() {
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
      "artifact": { "format": "wasm", "path": "../store/std-core.wasm" },
      "abi_id": "abi:std::core:1.0.0"
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
    let errs = v
        .get("errors")
        .and_then(|e| e.as_array())
        .expect("errors array");
    assert_eq!(errs.len(), 1);
    let e0 = &errs[0];
    assert_eq!(e0.get("code").and_then(|s| s.as_str()), Some("C027"));
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
      "artifact": { "format": "wasm", "path": "store/std-core.wasm" },
      "abi_id": "abi:std::core:1.0.0"
    },
    {
      "name": "std::host",
      "version": "1.0.0",
      "digest": "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
      "artifact": { "format": "wasm", "path": "store/std-host.wasm" },
      "abi_id": "abi:std::host:1.0.0"
    }
  ]
}"#,
    )
    .expect("write metadata");

    let first_output = Command::cargo_bin("clg")
        .unwrap()
        .args(["pkg", "lock", "--generate", "--root"])
        .arg(root)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let first_hash = extract_first_sha256_from_stdout(&first_output);
    let first = fs::read(root.join("clg.lock.json")).expect("read first lockfile bytes");
    let first_canonical = &first[..first.len() - 1];
    assert_eq!(sha256_hex(first_canonical), first_hash);
    let first_graph =
        fs::read(root.join("clg.resolved-graph.json")).expect("read first resolved graph bytes");
    let first_graph_hash =
        fs::read_to_string(root.join("clg.resolved-graph.sha256")).expect("read first graph hash");
    let first_graph_canonical = &first_graph[..first_graph.len() - 1];
    assert_eq!(sha256_hex(first_graph_canonical), first_graph_hash.trim());

    let second_output = Command::cargo_bin("clg")
        .unwrap()
        .args(["pkg", "lock", "--update", "--root"])
        .arg(root)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let second_hash = extract_first_sha256_from_stdout(&second_output);
    let second = fs::read(root.join("clg.lock.json")).expect("read second lockfile bytes");
    let second_canonical = &second[..second.len() - 1];
    assert_eq!(sha256_hex(second_canonical), second_hash);
    let second_graph =
        fs::read(root.join("clg.resolved-graph.json")).expect("read second resolved graph bytes");
    let second_graph_hash =
        fs::read_to_string(root.join("clg.resolved-graph.sha256")).expect("read second graph hash");
    let second_graph_canonical = &second_graph[..second_graph.len() - 1];
    assert_eq!(sha256_hex(second_graph_canonical), second_graph_hash.trim());

    assert_eq!(first, second, "lockfile replay bytes must be identical");
    assert_eq!(
        first_hash, second_hash,
        "lockfile hash must be deterministic"
    );
    assert_eq!(
        first_graph, second_graph,
        "resolved graph artifact bytes must be deterministic"
    );
    assert_eq!(
        first_graph_hash, second_graph_hash,
        "resolved graph artifact hash must be deterministic"
    );
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
      "artifact": { "format": "wasm", "path": "store/app-entry.wasm" },
      "abi_id": "abi:app::entry:1.0.0",
      "dependencies": [
        { "name": "lib::core", "requirement": "^1.0.0" }
      ]
    },
    {
      "name": "lib::core",
      "version": "1.0.0",
      "digest": "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
      "artifact": { "format": "wasm", "path": "store/lib-core.wasm" },
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
      "artifact": { "format": "wasm", "path": "store/z.wasm" },
      "abi_id": "abi:z::pkg:1.0.0",
      "dependencies": [{ "name": "y::pkg", "requirement": "^1.0.0" }]
    },
    {
      "name": "y::pkg",
      "version": "1.0.0",
      "digest": "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
      "artifact": { "format": "wasm", "path": "store/y.wasm" },
      "abi_id": "abi:y::pkg:1.0.0",
      "dependencies": [{ "name": "x::pkg", "requirement": "^1.0.0" }]
    },
    {
      "name": "x::pkg",
      "version": "1.0.0",
      "digest": "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
      "artifact": { "format": "wasm", "path": "store/x.wasm" },
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
      "artifact": { "format": "wasm", "path": "store/app-entry.wasm" },
      "abi_id": "abi:app::entry:1.0.0",
      "dependencies": [{ "name": "lib::core", "requirement": "^2.0.0" }]
    },
    {
      "name": "lib::core",
      "version": "1.0.0",
      "digest": "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
      "artifact": { "format": "wasm", "path": "store/lib-core.wasm" },
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
