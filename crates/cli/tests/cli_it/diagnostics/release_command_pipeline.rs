#[test]
fn release_command_orchestrates_lock_build_sign_verify_and_bundle() {
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
    let out_dir = root.join("out").join("release");

    let output = Command::cargo_bin("clg")
        .unwrap()
        .env("CLG_SOLVER_BIN", &solver)
        .args(["--non-interactive", "release"])
        .args(["--key"])
        .arg(&key_path)
        .args(["--pubkey"])
        .arg(&pubkey_path)
        .args(["--root"])
        .arg(&root)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8(output).expect("utf8 stdout");
    let stdout_json: Value = serde_json::from_str(stdout.trim()).expect("stdout json");
    assert_eq!(
        stdout_json.get("policy_version").and_then(|v| v.as_str()),
        Some("25.2.3")
    );
    assert!(
        stdout.contains("\"verify(require-assurance=proved_all)\""),
        "stdout should include verify stage contract"
    );

    let bundle_path = out_dir.join("main.release-bundle.json");
    assert!(bundle_path.exists(), "bundle manifest should exist");
    let bundle: Value =
        serde_json::from_slice(&fs::read(&bundle_path).expect("read bundle manifest"))
            .expect("json");
    assert_eq!(
        bundle.get("schema_version").and_then(|v| v.as_u64()),
        Some(1)
    );
    assert_eq!(
        bundle.get("policy_version").and_then(|v| v.as_str()),
        Some("25.2.3")
    );
    let orchestration = bundle
        .get("orchestration")
        .and_then(|v| v.as_array())
        .expect("orchestration");
    assert_eq!(orchestration.len(), 5, "all release stages should execute");
    for stage in orchestration {
        assert_eq!(stage.get("status").and_then(|v| v.as_str()), Some("ok"));
    }
    for field in [
        "module",
        "strict_import_map",
        "vcs",
        "proof",
        "signature",
        "assurance_manifest",
        "provenance",
    ] {
        let artifact = bundle
            .get("artifacts")
            .and_then(|v| v.get(field))
            .expect("artifact");
        let artifact_path = artifact
            .get("path")
            .and_then(|v| v.as_str())
            .expect("artifact path");
        assert!(
            Path::new(artifact_path).exists(),
            "artifact `{}` should exist at {}",
            field,
            artifact_path
        );
        let hash = artifact
            .get("sha256")
            .and_then(|v| v.as_str())
            .expect("artifact hash");
        assert_eq!(
            hash.len(),
            64,
            "artifact hash should be lowercase sha256 hex"
        );
    }
    assert!(
        bundle.get("shared_std").and_then(|v| v.as_array()).is_some(),
        "bundle manifest should include shared_std[] evidence"
    );
    let strict_import_map_path = bundle
        .get("artifacts")
        .and_then(|v| v.get("strict_import_map"))
        .and_then(|v| v.get("path"))
        .and_then(|v| v.as_str())
        .expect("strict import-map artifact path");
    let strict_import_map: Value = serde_json::from_slice(
        fs::read(strict_import_map_path)
            .expect("read strict import-map artifact")
            .as_slice(),
    )
    .expect("parse strict import-map artifact");
    assert!(
        strict_import_map
            .get("source_files")
            .and_then(|v| v.as_array())
            .map(|files| !files.is_empty())
            .unwrap_or(false),
        "strict import-map artifact should include source_files[] evidence"
    );
    assert!(
        strict_import_map
            .get("shared_std")
            .and_then(|v| v.as_array())
            .is_some(),
        "strict import-map artifact should include shared_std[] evidence"
    );
    let provenance_path = bundle
        .get("artifacts")
        .and_then(|v| v.get("provenance"))
        .and_then(|v| v.get("path"))
        .and_then(|v| v.as_str())
        .expect("provenance artifact path");
    let provenance: Value = serde_json::from_slice(
        fs::read(provenance_path)
            .expect("read provenance artifact")
            .as_slice(),
    )
    .expect("parse provenance artifact");
    assert!(
        provenance
            .get("payload")
            .and_then(|v| v.get("shared_std"))
            .and_then(|v| v.as_array())
            .is_some(),
        "provenance payload should include shared_std[] evidence"
    );
}

#[test]
fn release_command_preserves_non_empty_shared_std_evidence_for_shared_manifest() {
    let tmp = tempdir().unwrap();
    let root = tmp.path().join("project");
    fs::create_dir_all(&root).expect("create root");

    let source = root.join("main.clear");
    write_release_success_source(&source);
    write_minimal_strict_preflight_files(&root);
    write_shared_std_package_metadata_fixture(&root);
    fs::remove_file(root.join("clg.lock.json")).expect("remove legacy lockfile fixture");

    let verify_trust_policy = root.join("trust-policy.json");
    write_verify_trust_policy_v1(&verify_trust_policy);
    write_release_project_defaults_shared_std(
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
    let out_dir = root.join("out").join("release");

    Command::cargo_bin("clg")
        .unwrap()
        .env("CLG_SOLVER_BIN", &solver)
        .args(["--non-interactive", "release"])
        .args(["--key"])
        .arg(&key_path)
        .args(["--pubkey"])
        .arg(&pubkey_path)
        .args(["--root"])
        .arg(&root)
        .assert()
        .success();

    let bundle_path = out_dir.join("main.release-bundle.json");
    let bundle: Value =
        serde_json::from_slice(&fs::read(&bundle_path).expect("read bundle manifest"))
            .expect("parse bundle");
    let shared_std = bundle
        .get("shared_std")
        .and_then(|v| v.as_array())
        .expect("bundle shared_std array");
    assert_eq!(shared_std.len(), 1, "shared release should carry non-empty evidence");
    assert_eq!(
        shared_std[0]
            .get("package_id")
            .and_then(|v| v.as_str()),
        Some("std::text")
    );

    let strict_import_map_path = bundle
        .get("artifacts")
        .and_then(|v| v.get("strict_import_map"))
        .and_then(|v| v.get("path"))
        .and_then(|v| v.as_str())
        .expect("strict import-map artifact path");
    let strict_import_map: Value = serde_json::from_slice(
        fs::read(strict_import_map_path)
            .expect("read strict import-map artifact")
            .as_slice(),
    )
    .expect("parse strict import-map artifact");
    let import_map_shared_std = strict_import_map
        .get("shared_std")
        .and_then(|v| v.as_array())
        .expect("strict import-map shared_std");
    assert_eq!(import_map_shared_std.len(), 1);

    let provenance_path = bundle
        .get("artifacts")
        .and_then(|v| v.get("provenance"))
        .and_then(|v| v.get("path"))
        .and_then(|v| v.as_str())
        .expect("provenance artifact path");
    let provenance: Value = serde_json::from_slice(
        fs::read(provenance_path)
            .expect("read provenance artifact")
            .as_slice(),
    )
    .expect("parse provenance artifact");
    let provenance_shared_std = provenance
        .get("payload")
        .and_then(|v| v.get("shared_std"))
        .and_then(|v| v.as_array())
        .expect("provenance shared_std");
    assert_eq!(provenance_shared_std.len(), 1);
}

#[test]
fn release_command_accepts_std_contract_shared_std_evidence_for_shared_manifest() {
    let tmp = tempdir().unwrap();
    let root = tmp.path().join("project");
    fs::create_dir_all(&root).expect("create root");

    let source = root.join("main.clear");
    write_release_success_source(&source);
    write_minimal_strict_preflight_files(&root);
    write_shared_std_package_metadata_fixtures(
        &root,
        &[SharedStdPackageFixture {
            package_id: "std::contract",
            version: "1.2.0",
            artifact_text: "wasm-contract",
            key_seed: 7,
            key_id: "std-publisher-ed25519-2026q2",
            signed_at: "2026-06-01T00:00:00Z",
            statement_digest: "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            statement_format: "in-toto-v1",
            abi_major: 1,
            abi_minor_min: 0,
            abi_minor_max: 0,
        }],
    );
    fs::remove_file(root.join("clg.lock.json")).expect("remove legacy lockfile fixture");

    let verify_trust_policy = root.join("trust-policy.json");
    write_verify_trust_policy_v1(&verify_trust_policy);
    write_release_project_defaults_shared_std_with_named_package(
        root.join("clg.project.json").as_path(),
        "2026-03-31T00:00:00Z",
        "release-2026q2",
        "main.clear",
        "out/release",
        "trust-policy.json",
        "std::contract",
        "^1.2.0",
        1,
        0,
        0,
    );
    let (key_path, pubkey_path) = write_signing_keys(&root);

    let solver = write_fake_unsat_solver(&root.join("solver"));
    write_solver_integrity_sidecars(&solver);
    let out_dir = root.join("out").join("release");

    Command::cargo_bin("clg")
        .unwrap()
        .env("CLG_SOLVER_BIN", &solver)
        .args(["--non-interactive", "release"])
        .args(["--key"])
        .arg(&key_path)
        .args(["--pubkey"])
        .arg(&pubkey_path)
        .args(["--root"])
        .arg(&root)
        .assert()
        .success();

    let bundle_path = out_dir.join("main.release-bundle.json");
    let bundle: Value =
        serde_json::from_slice(&fs::read(&bundle_path).expect("read bundle manifest"))
            .expect("parse bundle");
    let shared_std = bundle
        .get("shared_std")
        .and_then(|v| v.as_array())
        .expect("bundle shared_std array");
    assert_eq!(shared_std.len(), 1, "shared release should carry non-empty evidence");
    assert_eq!(
        shared_std[0]
            .get("package_id")
            .and_then(|v| v.as_str()),
        Some("std::contract")
    );

    let strict_import_map_path = bundle
        .get("artifacts")
        .and_then(|v| v.get("strict_import_map"))
        .and_then(|v| v.get("path"))
        .and_then(|v| v.as_str())
        .expect("strict import-map artifact path");
    let strict_import_map: Value = serde_json::from_slice(
        fs::read(strict_import_map_path)
            .expect("read strict import-map artifact")
            .as_slice(),
    )
    .expect("parse strict import-map artifact");
    let import_map_shared_std = strict_import_map
        .get("shared_std")
        .and_then(|v| v.as_array())
        .expect("strict import-map shared_std");
    assert_eq!(import_map_shared_std.len(), 1);
    assert_eq!(
        import_map_shared_std[0]
            .get("package_id")
            .and_then(|v| v.as_str()),
        Some("std::contract")
    );
}

#[test]
fn release_command_accepts_std_eth_shared_std_evidence_for_shared_manifest() {
    let tmp = tempdir().unwrap();
    let root = tmp.path().join("project");
    fs::create_dir_all(&root).expect("create root");

    let source = root.join("main.clear");
    write_release_success_source(&source);
    write_minimal_strict_preflight_files(&root);
    write_shared_std_package_metadata_fixtures(
        &root,
        &[SharedStdPackageFixture {
            package_id: "std::eth",
            version: "1.2.0",
            artifact_text: "wasm-eth",
            key_seed: 9,
            key_id: "std-publisher-ed25519-2026q3",
            signed_at: "2026-06-15T00:00:00Z",
            statement_digest: "sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd",
            statement_format: "in-toto-v1",
            abi_major: 1,
            abi_minor_min: 0,
            abi_minor_max: 0,
        }],
    );
    fs::remove_file(root.join("clg.lock.json")).expect("remove legacy lockfile fixture");

    let verify_trust_policy = root.join("trust-policy.json");
    write_verify_trust_policy_v1(&verify_trust_policy);
    write_release_project_defaults_shared_std_with_named_package(
        root.join("clg.project.json").as_path(),
        "2026-03-31T00:00:00Z",
        "release-2026q2",
        "main.clear",
        "out/release",
        "trust-policy.json",
        "std::eth",
        "^1.2.0",
        1,
        0,
        0,
    );
    let (key_path, pubkey_path) = write_signing_keys(&root);

    let solver = write_fake_unsat_solver(&root.join("solver"));
    write_solver_integrity_sidecars(&solver);
    let out_dir = root.join("out").join("release");

    Command::cargo_bin("clg")
        .unwrap()
        .env("CLG_SOLVER_BIN", &solver)
        .args(["--non-interactive", "release"])
        .args(["--key"])
        .arg(&key_path)
        .args(["--pubkey"])
        .arg(&pubkey_path)
        .args(["--root"])
        .arg(&root)
        .assert()
        .success();

    let bundle_path = out_dir.join("main.release-bundle.json");
    let bundle: Value =
        serde_json::from_slice(&fs::read(&bundle_path).expect("read bundle manifest"))
            .expect("parse bundle");
    let shared_std = bundle
        .get("shared_std")
        .and_then(|v| v.as_array())
        .expect("bundle shared_std array");
    assert_eq!(shared_std.len(), 1, "shared release should carry non-empty evidence");
    assert_eq!(
        shared_std[0]
            .get("package_id")
            .and_then(|v| v.as_str()),
        Some("std::eth")
    );

    let strict_import_map_path = bundle
        .get("artifacts")
        .and_then(|v| v.get("strict_import_map"))
        .and_then(|v| v.get("path"))
        .and_then(|v| v.as_str())
        .expect("strict import-map artifact path");
    let strict_import_map: Value = serde_json::from_slice(
        fs::read(strict_import_map_path)
            .expect("read strict import-map artifact")
            .as_slice(),
    )
    .expect("parse strict import-map artifact");
    let import_map_shared_std = strict_import_map
        .get("shared_std")
        .and_then(|v| v.as_array())
        .expect("strict import-map shared_std");
    assert_eq!(import_map_shared_std.len(), 1);
    assert_eq!(
        import_map_shared_std[0]
            .get("package_id")
            .and_then(|v| v.as_str()),
        Some("std::eth")
    );

    let verify_out = Command::cargo_bin("clg")
        .unwrap()
        .args(["verify-bundle", "--bundle"])
        .arg(&bundle_path)
        .args(["--pubkey"])
        .arg(&pubkey_path)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let verify_text = String::from_utf8(verify_out).expect("utf8");
    let verify_summary: Value =
        serde_json::from_str(verify_text.trim()).expect("verify-bundle summary json");
    assert_eq!(
        verify_summary.get("status").and_then(|v| v.as_str()),
        Some("verified")
    );
}

#[test]
fn release_command_accepts_std_solana_shared_std_evidence_for_shared_manifest() {
    let tmp = tempdir().unwrap();
    let root = tmp.path().join("project");
    fs::create_dir_all(&root).expect("create root");

    let source = root.join("main.clear");
    write_release_success_source(&source);
    write_minimal_strict_preflight_files(&root);
    write_shared_std_package_metadata_fixtures(
        &root,
        &[SharedStdPackageFixture {
            package_id: "std::solana",
            version: "1.2.0",
            artifact_text: "wasm-solana",
            key_seed: 10,
            key_id: "std-publisher-ed25519-2026q4",
            signed_at: "2026-06-20T00:00:00Z",
            statement_digest: "sha256:eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
            statement_format: "in-toto-v1",
            abi_major: 1,
            abi_minor_min: 0,
            abi_minor_max: 0,
        }],
    );
    fs::remove_file(root.join("clg.lock.json")).expect("remove legacy lockfile fixture");

    let verify_trust_policy = root.join("trust-policy.json");
    write_verify_trust_policy_v1(&verify_trust_policy);
    write_release_project_defaults_shared_std_with_named_package(
        root.join("clg.project.json").as_path(),
        "2026-03-31T00:00:00Z",
        "release-2026q2",
        "main.clear",
        "out/release",
        "trust-policy.json",
        "std::solana",
        "^1.2.0",
        1,
        0,
        0,
    );
    let (key_path, pubkey_path) = write_signing_keys(&root);

    let solver = write_fake_unsat_solver(&root.join("solver"));
    write_solver_integrity_sidecars(&solver);
    let out_dir = root.join("out").join("release");

    Command::cargo_bin("clg")
        .unwrap()
        .env("CLG_SOLVER_BIN", &solver)
        .args(["--non-interactive", "release"])
        .args(["--key"])
        .arg(&key_path)
        .args(["--pubkey"])
        .arg(&pubkey_path)
        .args(["--root"])
        .arg(&root)
        .assert()
        .success();

    let bundle_path = out_dir.join("main.release-bundle.json");
    let bundle: Value =
        serde_json::from_slice(&fs::read(&bundle_path).expect("read bundle manifest"))
            .expect("parse bundle");
    let shared_std = bundle
        .get("shared_std")
        .and_then(|v| v.as_array())
        .expect("bundle shared_std array");
    assert_eq!(shared_std.len(), 1, "shared release should carry non-empty evidence");
    assert_eq!(
        shared_std[0]
            .get("package_id")
            .and_then(|v| v.as_str()),
        Some("std::solana")
    );

    let strict_import_map_path = bundle
        .get("artifacts")
        .and_then(|v| v.get("strict_import_map"))
        .and_then(|v| v.get("path"))
        .and_then(|v| v.as_str())
        .expect("strict import-map artifact path");
    let strict_import_map: Value = serde_json::from_slice(
        fs::read(strict_import_map_path)
            .expect("read strict import-map artifact")
            .as_slice(),
    )
    .expect("parse strict import-map artifact");
    let import_map_shared_std = strict_import_map
        .get("shared_std")
        .and_then(|v| v.as_array())
        .expect("strict import-map shared_std");
    assert_eq!(import_map_shared_std.len(), 1);
    assert_eq!(
        import_map_shared_std[0]
            .get("package_id")
            .and_then(|v| v.as_str()),
        Some("std::solana")
    );

    let verify_out = Command::cargo_bin("clg")
        .unwrap()
        .args(["verify-bundle", "--bundle"])
        .arg(&bundle_path)
        .args(["--pubkey"])
        .arg(&pubkey_path)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let verify_text = String::from_utf8(verify_out).expect("utf8");
    let verify_summary: Value =
        serde_json::from_str(verify_text.trim()).expect("verify-bundle summary json");
    assert_eq!(
        verify_summary.get("status").and_then(|v| v.as_str()),
        Some("verified")
    );
}

#[test]
fn shared_std_pkg_lock_update_and_release_support_upgrade_rotation_and_rollback() {
    let tmp = tempdir().unwrap();
    let root = tmp.path().join("project");
    fs::create_dir_all(&root).expect("create root");

    let source = root.join("main.clear");
    write_release_success_source(&source);
    write_minimal_strict_preflight_files(&root);
    fs::remove_file(root.join("clg.lock.json")).expect("remove legacy lockfile fixture");

    let verify_trust_policy = root.join("trust-policy.json");
    write_verify_trust_policy_v1(&verify_trust_policy);
    let (key_path, pubkey_path) = write_signing_keys(&root);
    let solver = write_fake_unsat_solver(&root.join("solver"));
    write_solver_integrity_sidecars(&solver);
    let out_dir = root.join("out").join("release");
    let lock_path = root.join("clg.lock.json");
    let bundle_path = out_dir.join("main.release-bundle.json");

    let v120 = SharedStdPackageFixture {
        package_id: "std::text",
        version: "1.2.0",
        artifact_text: "wasm-v120",
        key_seed: 7,
        key_id: "std-publisher-ed25519-2026q2",
        signed_at: "2026-06-01T00:00:00Z",
        statement_digest: "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        statement_format: "in-toto-v1",
        abi_major: 1,
        abi_minor_min: 0,
        abi_minor_max: 0,
    };
    let v130 = SharedStdPackageFixture {
        package_id: "std::text",
        version: "1.3.0",
        artifact_text: "wasm-v130",
        key_seed: 8,
        key_id: "std-publisher-ed25519-2026q3",
        signed_at: "2026-09-01T00:00:00Z",
        statement_digest: "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
        statement_format: "in-toto-v1",
        abi_major: 1,
        abi_minor_min: 0,
        abi_minor_max: 1,
    };

    let lock_and_release = |fixture: SharedStdPackageFixture<'_>,
                            version_requirement: &str,
                            abi_minor_max: u32|
     -> (Value, Value) {
        write_shared_std_package_metadata_fixtures(&root, &[fixture]);

        write_release_project_defaults_shared_std_with_package(
            root.join("clg.project.json").as_path(),
            "2026-03-31T00:00:00Z",
            "release-2026q2",
            "main.clear",
            "out/release",
            "trust-policy.json",
            version_requirement,
            1,
            0,
            abi_minor_max,
        );

        let lock_flag = if lock_path.exists() { "--update" } else { "--generate" };
        Command::cargo_bin("clg")
            .unwrap()
            .args(["pkg", "lock", lock_flag, "--root"])
            .arg(&root)
            .assert()
            .success();

        Command::cargo_bin("clg")
            .unwrap()
            .env("CLG_SOLVER_BIN", &solver)
            .args(["--non-interactive", "release"])
            .args(["--key"])
            .arg(&key_path)
            .args(["--pubkey"])
            .arg(&pubkey_path)
            .args(["--root"])
            .arg(&root)
            .assert()
            .success();

        Command::cargo_bin("clg")
            .unwrap()
            .args(["verify-bundle", "--bundle"])
            .arg(&bundle_path)
            .args(["--pubkey"])
            .arg(&pubkey_path)
            .assert()
            .success();

        let lock_json: Value =
            serde_json::from_slice(&fs::read(&lock_path).expect("read lockfile")).expect("json");
        let bundle_json: Value =
            serde_json::from_slice(&fs::read(&bundle_path).expect("read bundle")).expect("json");
        (lock_json, bundle_json)
    };

    let (lock_v120, bundle_v120) = lock_and_release(v120, "=1.2.0", 0);
    assert_eq!(
        lock_v120["std"]["packages"][0]["version"].as_str(),
        Some("1.2.0")
    );
    assert_eq!(
        lock_v120["std"]["packages"][0]["signature"]["key_id"].as_str(),
        Some("std-publisher-ed25519-2026q2")
    );
    assert_eq!(
        lock_v120["std"]["packages"][0]["provenance"]["statement_digest"].as_str(),
        Some("sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb")
    );
    assert_eq!(
        lock_v120["std"]["packages"][0]["verified_std_abi"]["minor_max"].as_u64(),
        Some(0)
    );
    assert_eq!(
        bundle_v120["shared_std"][0]["version"].as_str(),
        Some("1.2.0")
    );
    assert_eq!(
        bundle_v120["shared_std"][0]["signature_key_id"].as_str(),
        Some("std-publisher-ed25519-2026q2")
    );

    let (lock_v130, bundle_v130) = lock_and_release(v130, "=1.3.0", 1);
    assert_eq!(
        lock_v130["std"]["packages"][0]["version"].as_str(),
        Some("1.3.0")
    );
    assert_eq!(
        lock_v130["std"]["packages"][0]["signature"]["key_id"].as_str(),
        Some("std-publisher-ed25519-2026q3")
    );
    assert_eq!(
        lock_v130["std"]["packages"][0]["provenance"]["statement_digest"].as_str(),
        Some("sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc")
    );
    assert_eq!(
        lock_v130["std"]["packages"][0]["verified_std_abi"]["minor_max"].as_u64(),
        Some(1)
    );
    assert_eq!(
        bundle_v130["shared_std"][0]["version"].as_str(),
        Some("1.3.0")
    );
    assert_eq!(
        bundle_v130["shared_std"][0]["signature_key_id"].as_str(),
        Some("std-publisher-ed25519-2026q3")
    );
    assert_eq!(
        bundle_v130["shared_std"][0]["provenance_digest"].as_str(),
        Some("sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc")
    );
    assert_eq!(
        bundle_v130["shared_std"][0]["verified_std_abi"]["minor_max"].as_u64(),
        Some(1)
    );

    let (lock_rollback, bundle_rollback) = lock_and_release(v120, "=1.2.0", 0);
    assert_eq!(
        lock_rollback["std"]["packages"][0]["version"].as_str(),
        Some("1.2.0")
    );
    assert_eq!(
        lock_rollback["std"]["packages"][0]["signature"]["key_id"].as_str(),
        Some("std-publisher-ed25519-2026q2")
    );
    assert_eq!(
        bundle_rollback["shared_std"][0]["version"].as_str(),
        Some("1.2.0")
    );
    assert_eq!(
        bundle_rollback["shared_std"][0]["signature_key_id"].as_str(),
        Some("std-publisher-ed25519-2026q2")
    );
}

#[test]
fn release_command_fails_closed_when_proved_all_is_not_met() {
    let tmp = tempdir().unwrap();
    let root = tmp.path().join("project");
    fs::create_dir_all(&root).expect("create root");

    let source = root.join("main.clear");
    write_release_not_proved_source(&source);
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

    let output = Command::cargo_bin("clg")
        .unwrap()
        .env("CLG_SOLVER_BIN", &solver)
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
        text.contains("\"code\": \"C121\""),
        "expected C121, got: {text}"
    );
    assert!(
        text.contains("\"stage\": \"build\""),
        "expected build stage, got: {text}"
    );
}

#[test]
fn release_command_fails_closed_when_module_graph_references_tests_with_c128() {
    let tmp = tempdir().unwrap();
    let root = tmp.path().join("project");
    fs::create_dir_all(root.join("tests").join("unit")).expect("create tests/unit");

    let source = root.join("main.clear");
    fs::write(
        &source,
        r#"
import tests::unit::helper
function main() -> Int { helper::value() }
"#,
    )
    .expect("write source");
    fs::write(
        root.join("tests").join("unit").join("helper.clear"),
        "export function value() -> Int { 1 }\n",
    )
    .expect("write test helper module");
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

    let out = Command::cargo_bin("clg")
        .unwrap()
        .env("CLG_SOLVER_BIN", &solver)
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
    let text = String::from_utf8(out).expect("utf8");
    assert!(
        text.contains("\"code\": \"C128\""),
        "expected C128, got: {text}"
    );
    assert!(
        text.contains("tests/unit/helper.clear"),
        "expected offending tests path evidence, got: {text}"
    );
}

#[test]
fn verify_rejects_mock_substitution_against_release_signature_with_v003() {
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

    let out_dir = root.join("out").join("release");
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

    let module = out_dir.join("main.wasm");
    let sig = out_dir.join("main.sig.json");
    let manifest = out_dir.join("main.assurance.json");
    let proof = out_dir.join("main.proof.json");
    fs::create_dir_all(root.join("tests").join("mocks").join("common")).expect("create mocks dir");
    let mock_entry = root
        .join("tests")
        .join("mocks")
        .join("common")
        .join("main.clear");
    fs::write(&mock_entry, "function main() -> Int { 999 }\n").expect("write mock entry");
    Command::cargo_bin("clg")
        .unwrap()
        .args(["build"])
        .arg(&mock_entry)
        .args(["-o"])
        .arg(&module)
        .assert()
        .success();

    let out = Command::cargo_bin("clg")
        .unwrap()
        .args(["--json-errors", "verify"])
        .args(["--module"])
        .arg(&module)
        .args(["--sig"])
        .arg(&sig)
        .args(["--pubkey"])
        .arg(&pubkey_path)
        .args(["--verify-mode", "compile-time"])
        .args(["--trust-policy"])
        .arg(&verify_trust_policy)
        .args(["--assurance-manifest"])
        .arg(&manifest)
        .args(["--proof-artifact"])
        .arg(&proof)
        .args(["--require-assurance", "proved_all"])
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(out).expect("utf8");
    assert!(
        text.contains("\"code\": \"V002\"") || text.contains("\"code\": \"V003\""),
        "expected tamper-evidence verify failure (V002/V003), got: {text}"
    );
}

#[test]
fn release_check_only_runs_readiness_gate_without_signing_flags() {
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
    let solver = write_fake_unsat_solver(&root.join("solver"));
    write_solver_integrity_sidecars(&solver);

    let output = Command::cargo_bin("clg")
        .unwrap()
        .env("CLG_SOLVER_BIN", &solver)
        .args(["release", "--check-only", "--root"])
        .arg(&root)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let stdout = String::from_utf8(output).expect("utf8 stdout");
    let summary: Value = serde_json::from_str(stdout.trim()).expect("check-only summary json");
    assert_eq!(
        summary.get("mode").and_then(|v| v.as_str()),
        Some("check_only")
    );
    assert_eq!(summary.get("status").and_then(|v| v.as_str()), Some("ready"));
    assert!(
        !root.join("out").join("release").join("main.sig.json").exists(),
        "check-only should not emit signed artifact"
    );
}

#[test]
fn release_check_only_rejects_signing_flags() {
    let tmp = tempdir().unwrap();
    let root = tmp.path().join("project");
    fs::create_dir_all(&root).expect("create root");

    let source = root.join("main.clear");
    write_release_success_source(&source);
    write_minimal_strict_preflight_files(&root);
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

    let output = Command::cargo_bin("clg")
        .unwrap()
        .args(["release", "--check-only", "--key"])
        .arg(&key_path)
        .args(["--pubkey"])
        .arg(&pubkey_path)
        .args(["--root"])
        .arg(&root)
        .output()
        .expect("run release --check-only conflict");
    assert_eq!(
        output.status.code(),
        Some(2),
        "conflicting flags should fail as usage error"
    );
    let text = String::from_utf8(output.stderr).expect("utf8 stderr");
    assert!(
        text.contains("--check-only"),
        "expected check-only conflict message, got: {text}"
    );
}

#[test]
fn verify_bundle_verifies_release_manifest_without_manual_artifact_flags() {
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

    let bundle = root
        .join("out")
        .join("release")
        .join("main.release-bundle.json");
    let out = Command::cargo_bin("clg")
        .unwrap()
        .args(["verify-bundle", "--bundle"])
        .arg(&bundle)
        .args(["--pubkey"])
        .arg(&pubkey_path)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(out).expect("utf8");
    let summary: Value = serde_json::from_str(text.trim()).expect("verify-bundle summary json");
    assert_eq!(
        summary.get("status").and_then(|v| v.as_str()),
        Some("verified")
    );
}

#[test]
fn verify_bundle_verifies_with_keyring_key_id_resolution() {
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

    let keyring = root.join("keys").join("release-keyring.json");
    write_release_verify_keyring(&keyring, &[("release-2026q2", &pubkey_path)], &[]);
    let bundle = root
        .join("out")
        .join("release")
        .join("main.release-bundle.json");
    let out = Command::cargo_bin("clg")
        .unwrap()
        .args(["verify-bundle", "--bundle"])
        .arg(&bundle)
        .args(["--keyring"])
        .arg(&keyring)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(out).expect("utf8");
    let summary: Value = serde_json::from_str(text.trim()).expect("verify-bundle summary json");
    assert_eq!(
        summary.get("status").and_then(|v| v.as_str()),
        Some("verified")
    );
}

#[test]
fn verify_bundle_fails_closed_when_signature_key_id_is_unknown_in_keyring() {
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

    let keyring = root.join("keys").join("release-keyring.json");
    write_release_verify_keyring(&keyring, &[("release-2026q1", &pubkey_path)], &[]);
    let bundle = root
        .join("out")
        .join("release")
        .join("main.release-bundle.json");

    let out = Command::cargo_bin("clg")
        .unwrap()
        .args(["--json-errors", "verify-bundle", "--bundle"])
        .arg(&bundle)
        .args(["--keyring"])
        .arg(&keyring)
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(out).expect("utf8");
    assert!(
        text.contains("\"code\": \"C140\""),
        "expected C140 for unknown key_id, got: {text}"
    );
    assert!(
        text.contains("not found in keyring"),
        "expected unknown key_id message, got: {text}"
    );
}

#[test]
fn verify_bundle_fails_closed_when_signature_key_id_is_revoked_in_keyring() {
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

    let keyring = root.join("keys").join("release-keyring.json");
    write_release_verify_keyring(
        &keyring,
        &[("release-2026q2", &pubkey_path)],
        &["release-2026q2"],
    );
    let bundle = root
        .join("out")
        .join("release")
        .join("main.release-bundle.json");

    let out = Command::cargo_bin("clg")
        .unwrap()
        .args(["--json-errors", "verify-bundle", "--bundle"])
        .arg(&bundle)
        .args(["--keyring"])
        .arg(&keyring)
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(out).expect("utf8");
    assert!(
        text.contains("\"code\": \"C140\""),
        "expected C140 for revoked key_id, got: {text}"
    );
    assert!(
        text.contains("is revoked"),
        "expected revoked key_id message, got: {text}"
    );
}

#[test]
fn verify_bundle_fails_closed_on_artifact_hash_mismatch_with_c140() {
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

    let module = root.join("out").join("release").join("main.wasm");
    fs::write(&module, b"tampered wasm bytes").expect("tamper module");
    let bundle = root
        .join("out")
        .join("release")
        .join("main.release-bundle.json");

    let out = Command::cargo_bin("clg")
        .unwrap()
        .args(["--json-errors", "verify-bundle", "--bundle"])
        .arg(&bundle)
        .args(["--pubkey"])
        .arg(&pubkey_path)
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(out).expect("utf8");
    assert!(
        text.contains("\"code\": \"C140\""),
        "expected C140, got: {text}"
    );
    assert!(
        text.contains("hash mismatch"),
        "expected hash mismatch message, got: {text}"
    );
}

#[test]
fn verify_bundle_fails_closed_when_manifest_is_tampered_or_missing_members() {
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

    let bundle = root
        .join("out")
        .join("release")
        .join("main.release-bundle.json");
    let manifest_bytes = fs::read(&bundle).expect("read bundle");
    let manifest: Value = serde_json::from_slice(&manifest_bytes).expect("parse bundle");

    let mut tampered_manifest = manifest.clone();
    tampered_manifest
        .as_object_mut()
        .expect("bundle object")
        .insert("schema_version".to_string(), serde_json::json!(999));
    fs::write(
        &bundle,
        serde_json::to_vec_pretty(&tampered_manifest).expect("serialize tampered bundle"),
    )
    .expect("write tampered bundle");

    let tampered = Command::cargo_bin("clg")
        .unwrap()
        .args(["--json-errors", "verify-bundle", "--bundle"])
        .arg(&bundle)
        .args(["--pubkey"])
        .arg(&pubkey_path)
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let tampered_text = String::from_utf8(tampered).expect("utf8");
    assert!(
        tampered_text.contains("\"code\": \"C140\""),
        "expected C140 for tampered schema, got: {tampered_text}"
    );

    let mut missing = manifest;
    if let Some(artifacts) = missing.get_mut("artifacts").and_then(Value::as_object_mut) {
        artifacts.remove("proof");
    }
    fs::write(
        &bundle,
        serde_json::to_vec_pretty(&missing).expect("serialize missing member bundle"),
    )
    .expect("write missing member bundle");

    let missing_out = Command::cargo_bin("clg")
        .unwrap()
        .args(["--json-errors", "verify-bundle", "--bundle"])
        .arg(&bundle)
        .args(["--pubkey"])
        .arg(&pubkey_path)
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let missing_text = String::from_utf8(missing_out).expect("utf8");
    assert!(
        missing_text.contains("\"code\": \"C140\""),
        "expected C140 for missing bundle member, got: {missing_text}"
    );
}

#[test]
fn verify_bundle_fails_closed_on_detached_signature_mismatch_with_v001() {
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

    let out_dir = root.join("out").join("release");
    let signature = out_dir.join("main.sig.json");
    fs::write(
        &signature,
        serde_json::to_vec_pretty(&serde_json::json!({
            "key_id": "release-2026q2",
            "scope": "both",
            "signature_format": "ed25519",
            "signature": "00",
            "payload": {}
        }))
        .expect("serialize tampered signature"),
    )
    .expect("write tampered signature");

    let bundle = out_dir.join("main.release-bundle.json");
    let mut manifest: Value =
        serde_json::from_slice(&fs::read(&bundle).expect("read bundle")).expect("parse bundle");
    let sig_hash = format!("sha256:{}", hex::encode(Sha256::digest(fs::read(&signature).expect("read signature"))));
    if let Some(artifacts) = manifest.get_mut("artifacts").and_then(Value::as_object_mut) {
        if let Some(sig_entry) = artifacts.get_mut("signature").and_then(Value::as_object_mut) {
            sig_entry.insert("sha256".to_string(), Value::String(sig_hash.trim_start_matches("sha256:").to_string()));
        }
    }
    fs::write(
        &bundle,
        serde_json::to_vec_pretty(&manifest).expect("serialize bundle"),
    )
    .expect("write bundle");

    let out = Command::cargo_bin("clg")
        .unwrap()
        .args(["--json-errors", "verify-bundle", "--bundle"])
        .arg(&bundle)
        .args(["--pubkey"])
        .arg(&pubkey_path)
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(out).expect("utf8");
    assert!(
        text.contains("\"code\": \"V001\""),
        "expected detached signature verification failure V001, got: {text}"
    );
}

#[test]
fn verify_bundle_fails_closed_when_required_provenance_is_missing() {
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

    let bundle = root
        .join("out")
        .join("release")
        .join("main.release-bundle.json");
    let mut manifest: Value =
        serde_json::from_slice(&fs::read(&bundle).expect("read bundle")).expect("parse bundle");
    if let Some(artifacts) = manifest.get_mut("artifacts").and_then(Value::as_object_mut) {
        artifacts.remove("provenance");
    }
    fs::write(
        &bundle,
        serde_json::to_vec_pretty(&manifest).expect("serialize bundle"),
    )
    .expect("write bundle");

    let out = Command::cargo_bin("clg")
        .unwrap()
        .args(["--json-errors", "verify-bundle", "--bundle"])
        .arg(&bundle)
        .args(["--pubkey"])
        .arg(&pubkey_path)
        .args(["--require-provenance"])
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(out).expect("utf8");
    assert!(
        text.contains("\"code\": \"C140\""),
        "expected C140 for missing required provenance, got: {text}"
    );
    assert!(
        text.contains("required provenance"),
        "expected required provenance diagnostic, got: {text}"
    );
}

#[test]
fn verify_bundle_fails_closed_when_provenance_signature_payload_is_tampered() {
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

    let out_dir = root.join("out").join("release");
    let provenance = out_dir.join("main.provenance.json");
    let mut provenance_json: Value =
        serde_json::from_slice(&fs::read(&provenance).expect("read provenance"))
            .expect("parse provenance");
    if let Some(payload) = provenance_json.get_mut("payload").and_then(Value::as_object_mut) {
        payload.insert(
            "release_signature_key_id".to_string(),
            Value::String("different-key".to_string()),
        );
    }
    fs::write(
        &provenance,
        serde_json::to_vec_pretty(&provenance_json).expect("serialize provenance"),
    )
    .expect("write provenance");

    let bundle = out_dir.join("main.release-bundle.json");
    let mut bundle_json: Value =
        serde_json::from_slice(&fs::read(&bundle).expect("read bundle")).expect("parse bundle");
    let prov_hash = hex::encode(Sha256::digest(fs::read(&provenance).expect("read provenance")));
    if let Some(artifacts) = bundle_json.get_mut("artifacts").and_then(Value::as_object_mut) {
        if let Some(entry) = artifacts.get_mut("provenance").and_then(Value::as_object_mut) {
            entry.insert("sha256".to_string(), Value::String(prov_hash));
        }
    }
    fs::write(
        &bundle,
        serde_json::to_vec_pretty(&bundle_json).expect("serialize bundle"),
    )
    .expect("write bundle");

    let out = Command::cargo_bin("clg")
        .unwrap()
        .args(["--json-errors", "verify-bundle", "--bundle"])
        .arg(&bundle)
        .args(["--pubkey"])
        .arg(&pubkey_path)
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(out).expect("utf8");
    assert!(
        text.contains("\"code\": \"C140\""),
        "expected C140 for tampered provenance signature, got: {text}"
    );
    assert!(
        text.contains("provenance signature verification failed"),
        "expected provenance signature failure detail, got: {text}"
    );
}

#[test]
fn verify_bundle_fails_closed_when_strict_import_map_shared_std_evidence_is_tampered() {
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

    let out_dir = root.join("out").join("release");
    let strict_import_map = out_dir.join("main.strict-import-map.json");
    let mut strict_import_map_json: Value =
        serde_json::from_slice(&fs::read(&strict_import_map).expect("read strict import-map"))
            .expect("parse strict import-map");
    let shared_std = strict_import_map_json
        .get_mut("shared_std")
        .and_then(Value::as_array_mut)
        .expect("shared_std array");
    shared_std.push(serde_json::json!({
        "package_id": "std::text",
        "version": "1.2.0",
        "verified_std_abi": {
            "major": 1,
            "minor_min": 0,
            "minor_max": 0
        },
        "artifact_digest": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "signature_key_id": "std-publisher-ed25519-2026q2",
        "provenance_digest": "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
    }));
    fs::write(
        &strict_import_map,
        serde_json::to_vec_pretty(&strict_import_map_json).expect("serialize strict import-map"),
    )
    .expect("write strict import-map");

    let bundle = out_dir.join("main.release-bundle.json");
    let mut bundle_json: Value =
        serde_json::from_slice(&fs::read(&bundle).expect("read bundle")).expect("parse bundle");
    let strict_import_map_hash = hex::encode(Sha256::digest(
        fs::read(&strict_import_map).expect("read strict import-map"),
    ));
    if let Some(artifacts) = bundle_json.get_mut("artifacts").and_then(Value::as_object_mut) {
        if let Some(entry) = artifacts
            .get_mut("strict_import_map")
            .and_then(Value::as_object_mut)
        {
            entry.insert("sha256".to_string(), Value::String(strict_import_map_hash));
        }
    }
    fs::write(
        &bundle,
        serde_json::to_vec_pretty(&bundle_json).expect("serialize bundle"),
    )
    .expect("write bundle");

    let out = Command::cargo_bin("clg")
        .unwrap()
        .args(["--json-errors", "verify-bundle", "--bundle"])
        .arg(&bundle)
        .args(["--pubkey"])
        .arg(&pubkey_path)
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(out).expect("utf8");
    assert!(
        text.contains("\"code\": \"C140\""),
        "expected C140 for tampered strict import-map shared_std evidence, got: {text}"
    );
    assert!(
        text.contains("strict import-map shared std evidence mismatch"),
        "expected shared std parity detail, got: {text}"
    );
}

#[test]
fn verify_bundle_fails_closed_when_bundle_manifest_shared_std_evidence_is_tampered() {
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

    let out_dir = root.join("out").join("release");
    let bundle = out_dir.join("main.release-bundle.json");
    let mut bundle_json: Value =
        serde_json::from_slice(&fs::read(&bundle).expect("read bundle")).expect("parse bundle");
    bundle_json
        .get_mut("shared_std")
        .and_then(Value::as_array_mut)
        .expect("shared_std array")
        .push(serde_json::json!({
            "package_id": "std::text",
            "version": "1.2.0",
            "verified_std_abi": {
                "major": 1,
                "minor_min": 0,
                "minor_max": 0
            },
            "artifact_digest": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "signature_key_id": "std-publisher-ed25519-2026q2",
            "provenance_digest": "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
        }));
    fs::write(
        &bundle,
        serde_json::to_vec_pretty(&bundle_json).expect("serialize bundle"),
    )
    .expect("write bundle");

    let out = Command::cargo_bin("clg")
        .unwrap()
        .args(["--json-errors", "verify-bundle", "--bundle"])
        .arg(&bundle)
        .args(["--pubkey"])
        .arg(&pubkey_path)
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(out).expect("utf8");
    assert!(
        text.contains("\"code\": \"C140\""),
        "expected C140 for tampered bundle shared_std evidence, got: {text}"
    );
    assert!(
        text.contains("shared std provenance mismatch"),
        "expected shared std parity detail, got: {text}"
    );
}

#[test]
fn verify_bundle_fails_closed_when_provenance_shared_std_evidence_is_tampered() {
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

    let out_dir = root.join("out").join("release");
    let provenance = out_dir.join("main.provenance.json");
    let mut provenance_json: Value =
        serde_json::from_slice(&fs::read(&provenance).expect("read provenance"))
            .expect("parse provenance");
    let payload = provenance_json
        .get_mut("payload")
        .and_then(Value::as_object_mut)
        .expect("payload object");
    payload.insert(
        "shared_std".to_string(),
        serde_json::json!([{
            "package_id": "std::text",
            "version": "1.2.0",
            "verified_std_abi": {
                "major": 1,
                "minor_min": 0,
                "minor_max": 0
            },
            "artifact_digest": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "signature_key_id": "std-publisher-ed25519-2026q2",
            "provenance_digest": "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
        }]),
    );
    let payload_value = provenance_json
        .get("payload")
        .cloned()
        .expect("payload clone for resign");
    clg_cli::signing::sign_assurance_manifest(
        payload_value,
        &key_path,
        "release-2026q2",
        &provenance,
    )
    .expect("re-sign provenance");

    let bundle = out_dir.join("main.release-bundle.json");
    let mut bundle_json: Value =
        serde_json::from_slice(&fs::read(&bundle).expect("read bundle")).expect("parse bundle");
    let prov_hash = hex::encode(Sha256::digest(fs::read(&provenance).expect("read provenance")));
    if let Some(artifacts) = bundle_json.get_mut("artifacts").and_then(Value::as_object_mut) {
        if let Some(entry) = artifacts.get_mut("provenance").and_then(Value::as_object_mut) {
            entry.insert("sha256".to_string(), Value::String(prov_hash));
        }
    }
    fs::write(
        &bundle,
        serde_json::to_vec_pretty(&bundle_json).expect("serialize bundle"),
    )
    .expect("write bundle");

    let out = Command::cargo_bin("clg")
        .unwrap()
        .args(["--json-errors", "verify-bundle", "--bundle"])
        .arg(&bundle)
        .args(["--pubkey"])
        .arg(&pubkey_path)
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(out).expect("utf8");
    assert!(
        text.contains("\"code\": \"C140\""),
        "expected C140 for tampered provenance shared_std evidence, got: {text}"
    );
    assert!(
        text.contains("shared std provenance mismatch"),
        "expected shared std parity detail, got: {text}"
    );
}
