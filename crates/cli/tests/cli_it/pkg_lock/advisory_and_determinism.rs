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
      "artifact": { "format": "wasm", "path": "store/app-entry.wasm" },
      "abi_id": "abi:app::entry:1.0.0",
      "dependencies": [{ "name": "lib::core", "requirement": "^1.0.0" }]
    },
    {
      "name": "lib::core",
      "version": "1.1.0",
      "digest": "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
      "artifact": { "format": "wasm", "path": "store/lib-core.wasm" },
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
      "artifact": { "format": "wasm", "path": "store/app-entry.wasm" },
      "abi_id": "abi:app::entry:1.0.0",
      "dependencies": [{ "name": "lib::core", "requirement": "^1.0.0" }]
    },
    {
      "name": "lib::core",
      "version": "1.1.0",
      "digest": "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
      "artifact": { "format": "wasm", "path": "store/lib-core.wasm" },
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
      "artifact": { "format": "wasm", "path": "store/app-entry.wasm" },
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

#[test]
fn pkg_lock_strict_mode_requires_advisory_as_of() {
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
      "abi_id": "abi:app::entry:1.0.0"
    }
  ]
}"#,
    )
    .expect("write metadata");

    let output = Command::cargo_bin("clg")
        .unwrap()
        .args([
            "--json-errors",
            "pkg",
            "lock",
            "--generate",
            "--root",
            root.to_str().expect("root utf8"),
            "--compiler-mode",
            "strict",
        ])
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

#[test]
fn pkg_lock_strict_mode_rejects_unsigned_advisory_envelope() {
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
      "version": "1.0.0",
      "digest": "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
      "artifact": { "format": "wasm", "path": "store/lib-core.wasm" },
      "abi_id": "abi:lib::core:1.0.0"
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
      "id": "ADV-STRICT-001",
      "package": "lib::core",
      "affected": "^1.0.0",
      "severity": "critical",
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
        .args([
            "--json-errors",
            "pkg",
            "lock",
            "--generate",
            "--root",
            root.to_str().expect("root utf8"),
            "--compiler-mode",
            "strict",
            "--advisory-as-of",
            "2026-06-01T00:00:00Z",
        ])
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

#[test]
fn pkg_lock_strict_mode_accepts_valid_signed_advisory_envelope() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    let signing = SigningKey::from_bytes(&[9u8; 32]);
    let key_hex = hex::encode(signing.verifying_key().to_bytes());
    let signed_at = "2026-01-10T00:00:00Z";

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
      "abi_id": "abi:app::entry:1.0.0"
    }
  ]
}"#,
    )
    .expect("write metadata");
    fs::write(
        root.join("clg.trust-policy.json"),
        format!(
            r#"{{
  "schema_version": 0,
  "trusted_signers": [
    {{
      "key_id": "k1",
      "scheme": "ed25519",
      "public_key": "hex:{key_hex}",
      "not_before": "2026-01-01T00:00:00Z",
      "not_after": "2027-01-01T00:00:00Z"
    }}
  ],
  "revoked_key_ids": []
}}"#
        ),
    )
    .expect("write trust policy");

    let advisories = json!([
        {
            "id": "ADV-STRICT-OK-001",
            "package": "app::entry",
            "affected": "^1.0.0",
            "severity": "high",
            "action": "deny",
            "minimum_safe_version": "1.0.0",
            "issued_at": "2028-01-01T00:00:00Z",
            "expires_at": "2029-01-01T00:00:00Z"
        }
    ]);
    let payload = json!({
        "schema_version": 1,
        "advisories": advisories,
        "signed_at": signed_at,
    });
    let sig_hex = hex::encode(
        signing
            .sign(canonical_json_bytes(&payload).as_slice())
            .to_bytes(),
    );

    fs::write(
        root.join("clg.advisories.json"),
        serde_json::to_vec_pretty(&json!({
            "schema_version": 1,
            "advisories": payload.get("advisories").expect("advisories").clone(),
            "signature": {
                "key_id": "k1",
                "signed_at": signed_at,
                "signature_format": "ed25519",
                "signature": sig_hex
            }
        }))
        .expect("serialize advisories"),
    )
    .expect("write advisories");

    Command::cargo_bin("clg")
        .unwrap()
        .args([
            "pkg",
            "lock",
            "--generate",
            "--root",
            root.to_str().expect("root utf8"),
            "--compiler-mode",
            "strict",
            "--advisory-as-of",
            "2026-06-01T00:00:00Z",
        ])
        .assert()
        .success();
}

#[test]
fn pkg_lock_advisory_as_of_filters_active_window_deterministically() {
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
      "version": "1.0.0",
      "digest": "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
      "artifact": { "format": "wasm", "path": "store/lib-core.wasm" },
      "abi_id": "abi:lib::core:1.0.0"
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
      "id": "ADV-WINDOW-001",
      "package": "lib::core",
      "affected": "^1.0.0",
      "severity": "high",
      "action": "deny",
      "issued_at": "2026-01-01T00:00:00Z",
      "expires_at": "2026-02-01T00:00:00Z"
    }
  ]
}"#,
    )
    .expect("write advisories");

    Command::cargo_bin("clg")
        .unwrap()
        .args([
            "pkg",
            "lock",
            "--generate",
            "--root",
            root.to_str().expect("root utf8"),
            "--compiler-mode",
            "standard",
            "--advisory-as-of",
            "2026-03-01T00:00:00Z",
        ])
        .assert()
        .success();

    fs::remove_file(root.join("clg.lock.json")).expect("remove generated lockfile");
    fs::remove_file(root.join("clg.resolved-graph.json")).expect("remove resolved graph");
    fs::remove_file(root.join("clg.resolved-graph.sha256")).expect("remove graph hash");

    let output = Command::cargo_bin("clg")
        .unwrap()
        .args([
            "--json-errors",
            "pkg",
            "lock",
            "--generate",
            "--root",
            root.to_str().expect("root utf8"),
            "--compiler-mode",
            "standard",
            "--advisory-as-of",
            "2026-01-15T00:00:00Z",
        ])
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
fn pkg_lock_diagnostics_output_is_deterministic_across_identical_runs() {
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

    let first = Command::cargo_bin("clg")
        .unwrap()
        .args(["--json-errors", "pkg", "lock", "--generate", "--root"])
        .arg(root)
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let second = Command::cargo_bin("clg")
        .unwrap()
        .args(["--json-errors", "pkg", "lock", "--generate", "--root"])
        .arg(root)
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    assert_eq!(
        first, second,
        "resolver/solver diagnostics output must be deterministic across identical runs"
    );
}
