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

