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

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
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

fn write_release_project_defaults_shared_std(
    path: &Path,
    advisory_as_of: &str,
    key_id: &str,
    entry: &str,
    out_dir: &str,
    trust_policy: &str,
) {
    let value = json!({
        "schema_version": 2,
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
        "std": {
            "delivery": "shared",
            "packages": [
                {
                    "package_id": "std::text",
                    "version_requirement": "^1.2.0",
                    "verified_std_abi": {
                        "major": 1,
                        "minor_min": 0,
                        "minor_max": 0
                    },
                    "registry": "default",
                    "signer_policy": "std-publisher-prod",
                    "allow_compat_shims": false
                }
            ]
        },
        "release_defaults": {
            "advisory_as_of": advisory_as_of,
            "key_id": key_id,
            "out_dir": out_dir,
            "trust_policy": trust_policy,
        }
    });
    fs::write(
        path,
        serde_json::to_vec_pretty(&value).expect("serialize shared std project defaults"),
    )
    .expect("write shared std project defaults");
}

fn write_release_project_defaults_shared_std_with_package(
    path: &Path,
    advisory_as_of: &str,
    key_id: &str,
    entry: &str,
    out_dir: &str,
    trust_policy: &str,
    version_requirement: &str,
    abi_major: u32,
    abi_minor_min: u32,
    abi_minor_max: u32,
) {
    let value = json!({
        "schema_version": 2,
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
        "std": {
            "delivery": "shared",
            "packages": [
                {
                    "package_id": "std::text",
                    "version_requirement": version_requirement,
                    "verified_std_abi": {
                        "major": abi_major,
                        "minor_min": abi_minor_min,
                        "minor_max": abi_minor_max
                    },
                    "registry": "default",
                    "signer_policy": "std-publisher-prod",
                    "allow_compat_shims": false
                }
            ]
        },
        "release_defaults": {
            "advisory_as_of": advisory_as_of,
            "key_id": key_id,
            "out_dir": out_dir,
            "trust_policy": trust_policy,
        }
    });
    fs::write(
        path,
        serde_json::to_vec_pretty(&value).expect("serialize shared std project defaults"),
    )
    .expect("write shared std project defaults");
}

#[derive(Clone, Copy)]
struct SharedStdPackageFixture<'a> {
    version: &'a str,
    artifact_text: &'a str,
    key_seed: u8,
    key_id: &'a str,
    signed_at: &'a str,
    statement_digest: &'a str,
    statement_format: &'a str,
    abi_major: u32,
    abi_minor_min: u32,
    abi_minor_max: u32,
}

fn write_shared_std_package_metadata_fixture(root: &Path) {
    let signing = SigningKey::from_bytes(&[7u8; 32]);
    let signed_at = "2026-06-01T00:00:00Z";
    let digest = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    let key_id = "std-publisher-ed25519-2026q2";
    let signature = hex::encode(
        signing
            .sign(
                format!(
                    "clg-package-signature-v0\n{}\n{}\n{}\n{}\n",
                    "std::text", "1.2.0", digest, signed_at
                )
                .as_bytes(),
            )
            .to_bytes(),
    );
    let store_dir = root.join("std-packages");
    fs::create_dir_all(&store_dir).expect("create std-packages");
    fs::write(store_dir.join("std-text-1.2.0.wasm"), b"wasm").expect("write shared std artifact");
    fs::write(
        root.join("clg.package-metadata.json"),
        serde_json::to_vec_pretty(&json!({
            "schema_version": 1,
            "packages": [
                {
                    "name": "std::text",
                    "version": "1.2.0",
                    "digest": digest,
                    "artifact": { "format": "wasm", "path": "std-packages/std-text-1.2.0.wasm" },
                    "abi_id": "abi:std::text:1.2.0",
                    "dependencies": [],
                    "signature": {
                        "format": "ed25519",
                        "key_id": key_id,
                        "signed_at": signed_at,
                        "signature": signature
                    },
                    "trust": {
                        "trusted_anchor_ids": [key_id]
                    },
                    "verified_std_abi": {
                        "major": 1,
                        "minor_min": 0,
                        "minor_max": 0
                    },
                    "provenance": {
                        "statement_digest": "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
                        "statement_format": "in-toto-v1"
                    }
                }
            ]
        }))
        .expect("serialize shared std package metadata"),
    )
    .expect("write shared std package metadata");
    fs::write(
        root.join("clg.package-abi.json"),
        serde_json::to_vec_pretty(&json!({
            "schema_version": 0,
            "contracts": [
                {
                    "abi_id": "abi:std::text:1.2.0",
                    "package": "std::text",
                    "version": "1.2.0",
                    "imports": []
                }
            ]
        }))
        .expect("serialize shared std package abi"),
    )
    .expect("write shared std package abi");
    fs::write(
        root.join("clg.package-signatures.json"),
        serde_json::to_vec_pretty(&json!({
            "schema_version": 0,
            "signatures": [
                {
                    "name": "std::text",
                    "version": "1.2.0",
                    "digest": digest,
                    "key_id": key_id,
                    "signed_at": signed_at,
                    "signature_format": "ed25519",
                    "signature": signature
                }
            ]
        }))
        .expect("serialize shared std package signatures"),
    )
    .expect("write shared std package signatures");
    fs::write(
        root.join("clg.trust-policy.json"),
        serde_json::to_vec_pretty(&json!({
            "schema_version": 0,
            "trusted_signers": [
                {
                    "key_id": key_id,
                    "scheme": "ed25519",
                    "public_key": format!("hex:{}", hex::encode(signing.verifying_key().to_bytes())),
                    "not_before": "2026-01-01T00:00:00Z",
                    "not_after": "2027-01-01T00:00:00Z"
                }
            ],
            "revoked_key_ids": []
        }))
        .expect("serialize shared std trust policy"),
    )
    .expect("write shared std trust policy");
}

fn write_shared_std_package_metadata_fixtures(root: &Path, fixtures: &[SharedStdPackageFixture<'_>]) {
    let store_dir = root.join("std-packages");
    fs::create_dir_all(&store_dir).expect("create std-packages");

    let mut metadata_packages = Vec::with_capacity(fixtures.len());
    let mut abi_contracts = Vec::with_capacity(fixtures.len());
    let mut signatures = Vec::with_capacity(fixtures.len());
    let mut trusted_signers = Vec::with_capacity(fixtures.len());

    for fixture in fixtures {
        let signing = SigningKey::from_bytes(&[fixture.key_seed; 32]);
        let digest = format!("sha256:{}", sha256_hex(fixture.artifact_text.as_bytes()));
        let artifact_name = format!("std-text-{}.wasm", fixture.version);
        fs::write(store_dir.join(&artifact_name), fixture.artifact_text.as_bytes())
            .expect("write shared std artifact");
        let signature = hex::encode(
            signing
                .sign(
                    format!(
                        "clg-package-signature-v0\n{}\n{}\n{}\n{}\n",
                        "std::text", fixture.version, digest, fixture.signed_at
                    )
                    .as_bytes(),
                )
                .to_bytes(),
        );

        metadata_packages.push(json!({
            "name": "std::text",
            "version": fixture.version,
            "digest": digest,
            "artifact": { "format": "wasm", "path": format!("std-packages/{artifact_name}") },
            "abi_id": format!("abi:std::text:{}", fixture.version),
            "dependencies": [],
            "signature": {
                "format": "ed25519",
                "key_id": fixture.key_id,
                "signed_at": fixture.signed_at,
                "signature": signature
            },
            "trust": {
                "trusted_anchor_ids": [fixture.key_id]
            },
            "verified_std_abi": {
                "major": fixture.abi_major,
                "minor_min": fixture.abi_minor_min,
                "minor_max": fixture.abi_minor_max
            },
            "provenance": {
                "statement_digest": fixture.statement_digest,
                "statement_format": fixture.statement_format
            }
        }));
        abi_contracts.push(json!({
            "abi_id": format!("abi:std::text:{}", fixture.version),
            "package": "std::text",
            "version": fixture.version,
            "imports": []
        }));
        signatures.push(json!({
            "name": "std::text",
            "version": fixture.version,
            "digest": digest,
            "key_id": fixture.key_id,
            "signed_at": fixture.signed_at,
            "signature_format": "ed25519",
            "signature": signature
        }));
        trusted_signers.push(json!({
            "key_id": fixture.key_id,
            "scheme": "ed25519",
            "public_key": format!("hex:{}", hex::encode(signing.verifying_key().to_bytes())),
            "not_before": "2026-01-01T00:00:00Z",
            "not_after": "2027-01-01T00:00:00Z"
        }));
    }

    fs::write(
        root.join("clg.package-metadata.json"),
        serde_json::to_vec_pretty(&json!({
            "schema_version": 1,
            "packages": metadata_packages
        }))
        .expect("serialize shared std package metadata"),
    )
    .expect("write shared std package metadata");
    fs::write(
        root.join("clg.package-abi.json"),
        serde_json::to_vec_pretty(&json!({
            "schema_version": 0,
            "contracts": abi_contracts
        }))
        .expect("serialize shared std package abi"),
    )
    .expect("write shared std package abi");
    fs::write(
        root.join("clg.package-signatures.json"),
        serde_json::to_vec_pretty(&json!({
            "schema_version": 0,
            "signatures": signatures
        }))
        .expect("serialize shared std package signatures"),
    )
    .expect("write shared std package signatures");
    fs::write(
        root.join("clg.trust-policy.json"),
        serde_json::to_vec_pretty(&json!({
            "schema_version": 0,
            "trusted_signers": trusted_signers,
            "revoked_key_ids": []
        }))
        .expect("serialize shared std trust policy"),
    )
    .expect("write shared std trust policy");
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
