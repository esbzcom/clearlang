#[test]
fn release_command_uses_project_defaults_for_advisory_and_key_id() {
    let tmp = tempdir().unwrap();
    let root = tmp.path().join("project");
    fs::create_dir_all(&root).expect("create root");

    let source = root.join("main.clear");
    write_release_success_source(&source);
    write_minimal_strict_preflight_files(&root);
    fs::remove_file(root.join("clg.lock.json")).expect("remove legacy lockfile fixture");

    let verify_trust_policy = root.join("trust-policy.json");
    write_verify_trust_policy_v1(&verify_trust_policy);
    write_release_project_defaults(
        root.join("clg.project.json").as_path(),
        "2026-03-31T00:00:00Z",
        "release-from-project",
        "main.clear",
        "release-output",
        "trust-policy.json",
    );
    let (key_path, pubkey_path) = write_signing_keys(&root);

    let solver = write_fake_unsat_solver(&root.join("solver"));
    write_solver_integrity_sidecars(&solver);

    Command::cargo_bin("clg")
        .unwrap()
        .env("CLG_SOLVER_BIN", &solver)
        .args(["release"])
        .args(["--key"])
        .arg(&key_path)
        .args(["--pubkey"])
        .arg(&pubkey_path)
        .args(["--root"])
        .arg(&root)
        .assert()
        .success();

    let bundle_path = root.join("release-output").join("main.release-bundle.json");
    let bundle: Value =
        serde_json::from_slice(&fs::read(&bundle_path).expect("read bundle manifest"))
            .expect("json");
    assert_eq!(
        bundle.get("advisory_as_of").and_then(|v| v.as_str()),
        Some("2026-03-31T00:00:00Z")
    );
    assert_eq!(
        bundle.get("key_id").and_then(|v| v.as_str()),
        Some("release-from-project")
    );
    assert_eq!(
        bundle.get("trust_policy").and_then(|v| v.as_str()),
        Some(verify_trust_policy.to_string_lossy().as_ref())
    );
}

#[test]
fn release_command_requires_manifest_defaults_when_placeholders_are_unresolved() {
    let tmp = tempdir().unwrap();
    let root = tmp.path().join("project");
    fs::create_dir_all(&root).expect("create root");

    let source = root.join("main.clear");
    write_release_success_source(&source);
    write_minimal_strict_preflight_files(&root);
    write_release_project_defaults(
        root.join("clg.project.json").as_path(),
        "REQUIRED_RFC3339_UTC",
        "REQUIRED_KEY_ID",
        "main.clear",
        "out/release",
        "trust-policy.json",
    );
    let verify_trust_policy = root.join("trust-policy.json");
    write_verify_trust_policy_v1(&verify_trust_policy);
    let (key_path, pubkey_path) = write_signing_keys(&root);

    let output = Command::cargo_bin("clg")
        .unwrap()
        .args(["--json-errors", "release"])
        .args(["--key"])
        .arg(&key_path)
        .args(["--pubkey"])
        .arg(&pubkey_path)
        .args(["--root"])
        .arg(&root)
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(output).expect("utf8");
    assert!(
        text.contains("\"code\": \"C130\""),
        "expected C130, got: {text}"
    );
    assert!(
        text.contains("release_defaults.advisory_as_of"),
        "expected unresolved advisory placeholder message, got: {text}"
    );
}

#[test]
fn release_command_fails_closed_when_project_entry_is_missing() {
    let tmp = tempdir().unwrap();
    let root = tmp.path().join("project");
    fs::create_dir_all(&root).expect("create root");

    write_minimal_strict_preflight_files(&root);
    write_release_project_defaults(
        root.join("clg.project.json").as_path(),
        "2026-03-31T00:00:00Z",
        "release-2026q2",
        "missing.clear",
        "out/release",
        "trust-policy.json",
    );
    let verify_trust_policy = root.join("trust-policy.json");
    write_verify_trust_policy_v1(&verify_trust_policy);
    let (key_path, pubkey_path) = write_signing_keys(&root);

    let output = Command::cargo_bin("clg")
        .unwrap()
        .args(["--json-errors", "release"])
        .args(["--key"])
        .arg(&key_path)
        .args(["--pubkey"])
        .arg(&pubkey_path)
        .args(["--root"])
        .arg(&root)
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(output).expect("utf8");
    assert!(
        text.contains("\"code\": \"C130\""),
        "expected C130, got: {text}"
    );
    assert!(
        text.contains("project.entry"),
        "expected missing project.entry diagnostic context, got: {text}"
    );
}

#[test]
fn release_command_fails_closed_when_project_clg_version_is_incompatible() {
    let tmp = tempdir().unwrap();
    let root = tmp.path().join("project");
    fs::create_dir_all(&root).expect("create root");

    write_release_success_source(root.join("main.clear").as_path());
    write_minimal_strict_preflight_files(&root);
    fs::write(
        root.join("clg.project.json"),
        r#"{
  "schema_version": 1,
  "project": {
    "name": "example-app",
    "description": "Example project",
    "version": "0.1.0",
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
    let verify_trust_policy = root.join("trust-policy.json");
    write_verify_trust_policy_v1(&verify_trust_policy);
    let (key_path, pubkey_path) = write_signing_keys(&root);

    let output = Command::cargo_bin("clg")
        .unwrap()
        .args(["--json-errors", "release"])
        .args(["--key"])
        .arg(&key_path)
        .args(["--pubkey"])
        .arg(&pubkey_path)
        .args(["--root"])
        .arg(&root)
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(output).expect("utf8");
    assert!(
        text.contains("\"code\": \"C130\""),
        "expected C130, got: {text}"
    );
    assert!(
        text.contains("project.clg_version"),
        "expected clg_version compatibility diagnostics, got: {text}"
    );
}

#[test]
fn release_command_json_events_emit_structured_progress() {
    let tmp = tempdir().unwrap();
    let root = tmp.path().join("project");
    fs::create_dir_all(&root).expect("create root");

    let source = root.join("main.clear");
    write_release_success_source(&source);
    write_minimal_strict_preflight_files(&root);
    fs::remove_file(root.join("clg.lock.json")).expect("remove legacy lockfile fixture");

    let verify_trust_policy = root.join("trust-policy.json");
    write_verify_trust_policy_v1(&verify_trust_policy);
    write_release_project_defaults(
        root.join("clg.project.json").as_path(),
        "2026-03-31T00:00:00Z",
        "release-2026q2",
        "main.clear",
        "out/release",
        "trust-policy.json",
    );
    let (key_path, pubkey_path) = write_signing_keys(&root);
    let solver = write_fake_unsat_solver(&root.join("solver"));
    write_solver_integrity_sidecars(&solver);

    let stderr = Command::cargo_bin("clg")
        .unwrap()
        .env("CLG_SOLVER_BIN", &solver)
        .args(["--json-events", "release"])
        .args(["--key"])
        .arg(&key_path)
        .args(["--pubkey"])
        .arg(&pubkey_path)
        .args(["--root"])
        .arg(&root)
        .assert()
        .success()
        .get_output()
        .stderr
        .clone();
    let text = String::from_utf8(stderr).expect("utf8 stderr");
    let events: Vec<Value> = text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str::<Value>(line).expect("json event line"))
        .collect();
    assert!(
        events.iter().any(|event| {
            event.get("schema_version").and_then(|v| v.as_u64()) == Some(1)
                && event.get("command").and_then(|v| v.as_str()) == Some("release")
                && event.get("event").and_then(|v| v.as_str()) == Some("start")
                && event.get("stage").and_then(|v| v.as_str()) == Some("release_lock")
        }),
        "expected release_lock start json event, got: {text}"
    );
}

#[test]
fn release_command_rejects_retired_non_secret_flags_with_usage_error() {
    for (flag, value) in [
        ("--advisory-as-of", "2026-03-31T00:00:00Z"),
        ("--key-id", "release-2026q2"),
        ("--out-dir", "out/release"),
        ("--trust-policy", "trust-policy.json"),
    ] {
        let output = Command::cargo_bin("clg")
            .unwrap()
            .args(["release"])
            .args(["--key", "keys/signing.json"])
            .args(["--pubkey", "keys/public.json"])
            .arg(flag)
            .arg(value)
            .output()
            .expect("run release usage rejection");
        assert_eq!(
            output.status.code(),
            Some(2),
            "usage error should exit with 2 for {flag}"
        );
        let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
        assert!(
            stderr.contains(format!("unexpected argument '{flag}'").as_str()),
            "expected retired flag rejection for {flag}, got: {stderr}"
        );
    }
}

#[test]
fn release_command_discovers_unique_manifest_under_current_directory() {
    let tmp = tempdir().unwrap();
    let workspace = tmp.path().join("workspace");
    let root = workspace.join("generic");
    fs::create_dir_all(&root).expect("create root");

    let source = root.join("main.clear");
    write_release_success_source(&source);
    write_minimal_strict_preflight_files(&root);
    fs::remove_file(root.join("clg.lock.json")).expect("remove legacy lockfile fixture");

    let verify_trust_policy = root.join("trust-policy.json");
    write_verify_trust_policy_v1(&verify_trust_policy);
    write_release_project_defaults(
        root.join("clg.project.json").as_path(),
        "2026-03-31T00:00:00Z",
        "release-2026q2",
        "main.clear",
        "out/release",
        "trust-policy.json",
    );
    let (key_path, pubkey_path) = write_signing_keys(&root);
    let solver = write_fake_unsat_solver(&root.join("solver"));
    write_solver_integrity_sidecars(&solver);

    Command::cargo_bin("clg")
        .unwrap()
        .current_dir(&workspace)
        .env("CLG_SOLVER_BIN", &solver)
        .args(["release"])
        .args(["--key"])
        .arg(&key_path)
        .args(["--pubkey"])
        .arg(&pubkey_path)
        .assert()
        .success();

    assert!(
        root.join("out")
            .join("release")
            .join("main.release-bundle.json")
            .exists(),
        "release should discover project root from current directory subtree"
    );
}

#[test]
fn release_command_rejects_ambiguous_manifest_discovery_without_root() {
    let tmp = tempdir().unwrap();
    let workspace = tmp.path().join("workspace");
    let generic = workspace.join("generic");
    let crypto = workspace.join("crypto");
    fs::create_dir_all(&generic).expect("create generic");
    fs::create_dir_all(&crypto).expect("create crypto");
    write_release_success_source(&generic.join("main.clear"));
    write_release_success_source(&crypto.join("main.clear"));
    write_minimal_strict_preflight_files(&generic);
    write_minimal_strict_preflight_files(&crypto);
    write_release_project_defaults(
        generic.join("clg.project.json").as_path(),
        "2026-03-31T00:00:00Z",
        "release-generic",
        "main.clear",
        "out/release",
        "trust-policy.json",
    );
    write_release_project_defaults(
        crypto.join("clg.project.json").as_path(),
        "2026-03-31T00:00:00Z",
        "release-crypto",
        "main.clear",
        "out/release",
        "trust-policy.json",
    );

    let out = Command::cargo_bin("clg")
        .unwrap()
        .current_dir(&workspace)
        .args(["--json-errors", "release"])
        .args(["--key"])
        .arg(workspace.join("keys").join("signing.json"))
        .args(["--pubkey"])
        .arg(workspace.join("keys").join("public.json"))
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(out).expect("utf8");
    assert!(
        text.contains("\"code\": \"C130\""),
        "expected C130, got: {text}"
    );
    assert!(
        text.contains("multiple `clg.project.json` files found"),
        "expected ambiguity diagnostics, got: {text}"
    );
}

#[test]
fn release_command_rejects_missing_manifest_discovery_without_root() {
    let tmp = tempdir().unwrap();
    let workspace = tmp.path().join("workspace");
    fs::create_dir_all(&workspace).expect("create workspace");

    let out = Command::cargo_bin("clg")
        .unwrap()
        .current_dir(&workspace)
        .args(["--json-errors", "release"])
        .args(["--key"])
        .arg(workspace.join("keys").join("signing.json"))
        .args(["--pubkey"])
        .arg(workspace.join("keys").join("public.json"))
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(out).expect("utf8");
    assert!(
        text.contains("\"code\": \"C130\""),
        "expected C130, got: {text}"
    );
    assert!(
        text.contains("could not find `clg.project.json`"),
        "expected missing-manifest discovery diagnostics, got: {text}"
    );
}
