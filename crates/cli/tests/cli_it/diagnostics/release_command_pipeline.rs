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

