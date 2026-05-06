use serde_json::json;

fn current_clg_version_requirement() -> String {
    let current_full = env!("CARGO_PKG_VERSION");
    let current_core = current_full
        .split(['-', '+'])
        .next()
        .unwrap_or(current_full);
    let mut parts = current_core.split('.');
    let major = parts
        .next()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(0);
    let minor = parts
        .next()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(1);
    format!("^{}.{}.0", major, minor)
}

fn write_verify_trust_policy_v1(path: &Path) {
    let value = json!({
        "schema_version": 1,
        "trust_anchors": {
            "lean_checker": "4.14.0",
            "coq_checker": "8.19.2",
        }
    });
    fs::write(
        path,
        serde_json::to_vec_pretty(&value).expect("serialize trust policy"),
    )
    .expect("write trust policy");
}

fn write_release_project_defaults(
    path: &Path,
    advisory_as_of: &str,
    key_id: &str,
    entry: &str,
    out_dir: &str,
    trust_policy: &str,
) {
    let value = json!({
        "schema_version": 1,
        "project": {
            "name": "example-app",
            "description": "Example project",
            "version": "0.1.0",
            "clg_version": current_clg_version_requirement(),
            "entry": entry,
            "website": "https://example.com",
            "contact": {
                "name": "Example Maintainer",
                "email": "maintainer@example.com"
            }
        },
        "dependencies": [],
        "release_defaults": {
            "advisory_as_of": advisory_as_of,
            "key_id": key_id,
            "out_dir": out_dir,
            "trust_policy": trust_policy,
        }
    });
    fs::write(
        path,
        serde_json::to_vec_pretty(&value).expect("serialize project defaults"),
    )
    .expect("write project defaults");
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

fn write_release_verify_keyring(path: &Path, entries: &[(&str, &Path)], revoked: &[&str]) {
    let keys = entries
        .iter()
        .map(|(key_id, pubkey)| {
            json!({
                "key_id": key_id,
                "pubkey": pubkey.to_string_lossy().to_string(),
            })
        })
        .collect::<Vec<_>>();
    let revoked_ids = revoked.iter().map(|id| json!(id)).collect::<Vec<_>>();
    let value = json!({
        "schema_version": 1,
        "keys": keys,
        "revoked_key_ids": revoked_ids,
    });
    fs::write(
        path,
        serde_json::to_vec_pretty(&value).expect("serialize release verify keyring"),
    )
    .expect("write release verify keyring");
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


include!("release_command_pipeline.rs");
include!("release_command_manifest_and_discovery.rs");
