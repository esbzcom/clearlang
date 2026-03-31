use serde_json::json;

fn write_verify_trust_policy_v1(path: &Path) {
    let value = json!({
        "schema_version": 1,
        "trust_anchors": {
            "lean_checker": "4.14.0",
            "coq_checker": "8.19.2",
        }
    });
    fs::write(path, serde_json::to_vec_pretty(&value).expect("serialize trust policy"))
        .expect("write trust policy");
}

fn write_signing_keys(root: &Path) -> (PathBuf, PathBuf) {
    let signing = SigningKey::from_bytes(&[7u8; 32]);
    let public = signing.verifying_key();
    let key_path = root.join("keys").join("signing.json");
    let pubkey_path = root.join("keys").join("public.json");
    fs::create_dir_all(key_path.parent().expect("key dir")).expect("create key dir");
    fs::write(
        &key_path,
        serde_json::to_vec_pretty(&json!({
            "scheme": "ed25519",
            "private_key": hex::encode(signing.to_bytes()),
            "public_key": hex::encode(public.to_bytes()),
        }))
        .expect("serialize signing key"),
    )
    .expect("write signing key");
    fs::write(
        &pubkey_path,
        serde_json::to_vec_pretty(&json!({
            "scheme": "ed25519",
            "public_key": hex::encode(public.to_bytes()),
        }))
        .expect("serialize public key"),
    )
    .expect("write public key");
    (key_path, pubkey_path)
}

fn write_release_success_source(path: &Path) {
    fs::write(
        path,
        r#"
pure function inc(x: Int) -> Int
    require { 0 <= x }
    ensure { result > x }
{ x + 1 }
function main() -> Int { inc(1) }
"#,
    )
    .expect("write source");
}

fn write_release_not_proved_source(path: &Path) {
    fs::write(path, "function main() -> Int { 0 }").expect("write source");
}

fn write_fake_unsat_solver(dir: &Path) -> PathBuf {
    let solver = if cfg!(windows) {
        dir.join("fake-release-z3.exe")
    } else {
        dir.join("fake-release-z3")
    };
    let source = r#"
fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|arg| arg == "--version" || arg == "-version") {
        println!("Z3 version 4.16.0 - fake-release");
        return;
    }
    println!("unsat");
}
"#;
    write_fake_solver_to(&solver, source)
}

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
    let (key_path, pubkey_path) = write_signing_keys(&root);

    let solver = write_fake_unsat_solver(&root.join("solver"));
    write_solver_integrity_sidecars(&solver);

    let out_dir = root.join("out").join("release");
    let output = Command::cargo_bin("clg")
        .unwrap()
        .env("CLG_SOLVER_BIN", &solver)
        .args(["release"])
        .arg(&source)
        .args(["--advisory-as-of", "2026-03-31T00:00:00Z"])
        .args(["--key"])
        .arg(&key_path)
        .args(["--key-id", "release-2026q2"])
        .args(["--pubkey"])
        .arg(&pubkey_path)
        .args(["--root"])
        .arg(&root)
        .args(["--out-dir"])
        .arg(&out_dir)
        .args(["--trust-policy"])
        .arg(&verify_trust_policy)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8(output).expect("utf8 stdout");
    assert!(stdout.contains("\"policy_version\": \"25.2.3\""));
    assert!(stdout.contains("\"verify(require-assurance=proved_all)\""));

    let bundle_path = out_dir.join("main.release-bundle.json");
    assert!(bundle_path.exists(), "bundle manifest should exist");
    let bundle: Value =
        serde_json::from_slice(&fs::read(&bundle_path).expect("read bundle manifest")).expect("json");
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
    for field in ["module", "vcs", "proof", "signature", "assurance_manifest"] {
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
        assert_eq!(hash.len(), 64, "artifact hash should be lowercase sha256 hex");
    }
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
    let (key_path, pubkey_path) = write_signing_keys(&root);
    let solver = write_fake_unsat_solver(&root.join("solver"));
    write_solver_integrity_sidecars(&solver);

    let out_dir = root.join("out").join("release");
    let output = Command::cargo_bin("clg")
        .unwrap()
        .env("CLG_SOLVER_BIN", &solver)
        .args(["--json-errors", "release"])
        .arg(&source)
        .args(["--advisory-as-of", "2026-03-31T00:00:00Z"])
        .args(["--key"])
        .arg(&key_path)
        .args(["--key-id", "release-2026q2"])
        .args(["--pubkey"])
        .arg(&pubkey_path)
        .args(["--root"])
        .arg(&root)
        .args(["--out-dir"])
        .arg(&out_dir)
        .args(["--trust-policy"])
        .arg(&verify_trust_policy)
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(output).expect("utf8");
    assert!(text.contains("\"code\": \"C121\""), "expected C121, got: {text}");
    assert!(
        text.contains("\"stage\": \"build\""),
        "expected build stage, got: {text}"
    );
}
