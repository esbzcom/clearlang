use assert_cmd::prelude::*;
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::tempdir;

#[derive(Deserialize)]
struct FixtureManifest {
    version: u32,
    profile: String,
    fixtures: Vec<ProfileFixture>,
}

#[derive(Deserialize)]
struct ProfileFixture {
    id: String,
    path: String,
    min_tier: String,
}

struct ExpectedFixture {
    id: &'static str,
    path: &'static str,
    min_tier: &'static str,
}

const VERIFIED_STD_CORE_V1_BASELINE: [ExpectedFixture; 3] = [
    ExpectedFixture {
        id: "fixture.contract_core",
        path: "clearlang-tests/profile/01_contract_core.clear",
        min_tier: "L1",
    },
    ExpectedFixture {
        id: "fixture.linear_collection_flow",
        path: "clearlang-tests/profile/03_linear_collection_flow.clear",
        min_tier: "L1",
    },
    ExpectedFixture {
        id: "fixture.refinement_loops",
        path: "clearlang-tests/profile/02_refinement_loops.clear",
        min_tier: "L1",
    },
];

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

fn tier_rank(tier: &str) -> Option<u8> {
    match tier {
        "L0" => Some(0),
        "L1" => Some(1),
        "L2" => Some(2),
        "L3" => Some(3),
        _ => None,
    }
}

fn write_fake_solver_to(path: &Path) -> PathBuf {
    let source = path.with_extension("rs");
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create fake solver dir");
    }
    let solver = path.to_path_buf();
    let fake_solver_src = r#"
use std::io::{self, Read};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|arg| arg == "--version" || arg == "-version") {
        println!("Z3 version 4.16.0 - fake");
        return;
    }
    let mut stdin = Vec::new();
    let _ = io::stdin().read_to_end(&mut stdin);
    println!("unsat");
}
"#;
    fs::write(&source, fake_solver_src).expect("write fake solver source");
    let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_string());
    let output = Command::new(rustc)
        .arg(&source)
        .arg("-O")
        .arg("-o")
        .arg(&solver)
        .output()
        .expect("run rustc for fake solver");
    assert!(
        output.status.success(),
        "fake solver compile failed: {}",
        String::from_utf8_lossy(output.stderr.as_slice())
    );
    solver
}

fn write_solver_integrity_sidecars(solver: &Path) {
    let solver_bytes = fs::read(solver).expect("read solver bytes");
    let checksum = format!("sha256:{}", hex::encode(Sha256::digest(&solver_bytes)));
    let checksum_path = solver.with_file_name(format!(
        "{}.sha256",
        solver
            .file_name()
            .expect("solver filename")
            .to_string_lossy()
    ));
    fs::write(&checksum_path, format!("{checksum}\n")).expect("write checksum sidecar");

    let signature = format!(
        "sha256:{}",
        hex::encode(Sha256::digest(checksum.as_bytes()))
    );
    let signature_path = solver.with_file_name(format!(
        "{}.sig",
        solver
            .file_name()
            .expect("solver filename")
            .to_string_lossy()
    ));
    fs::write(&signature_path, format!("{signature}\n")).expect("write signature sidecar");
}

#[test]
fn verified_profile_fixtures_do_not_regress_assurance_tier() {
    let root = repo_root();
    let manifest_path = root
        .join("docs")
        .join("proofs")
        .join("verified-profile-fixtures.json");
    let raw = fs::read_to_string(&manifest_path).expect("read fixture manifest");
    let manifest: FixtureManifest = serde_json::from_str(&raw).expect("parse fixture manifest");

    assert_eq!(manifest.version, 1, "fixture manifest version should be 1");
    assert_eq!(
        manifest.profile, "verified.std_core.v1",
        "fixture manifest profile should match verified subset"
    );
    assert!(
        manifest.fixtures.len() == VERIFIED_STD_CORE_V1_BASELINE.len(),
        "fixture manifest must match the verified.std_core.v1 baseline fixture count"
    );

    let mut fixture_ids: Vec<&str> = manifest.fixtures.iter().map(|f| f.id.as_str()).collect();
    let original_ids = fixture_ids.clone();
    fixture_ids.sort_unstable();
    assert_eq!(
        original_ids, fixture_ids,
        "fixtures must be sorted by id for deterministic diffs"
    );

    for (fixture, expected) in manifest
        .fixtures
        .iter()
        .zip(VERIFIED_STD_CORE_V1_BASELINE.iter())
    {
        assert_eq!(
            fixture.id, expected.id,
            "fixture id baseline drift for profile verified.std_core.v1"
        );
        assert_eq!(
            fixture.path, expected.path,
            "fixture path baseline drift for fixture `{}`",
            fixture.id
        );
        assert_eq!(
            fixture.min_tier, expected.min_tier,
            "fixture min_tier baseline drift for fixture `{}`",
            fixture.id
        );

        let src_path = root.join(&fixture.path);
        assert!(
            src_path.exists(),
            "fixture source does not exist: {}",
            fixture.path
        );
        let min_rank = tier_rank(expected.min_tier).unwrap_or_else(|| {
            panic!(
                "fixture `{}` has unknown min_tier `{}`",
                fixture.id, expected.min_tier
            )
        });

        let tmp = tempdir().unwrap();
        let wasm_path = tmp.path().join("fixture.wasm");
        let vcs_standard = tmp.path().join("fixture.standard.vc.json");
        let vcs_strict = tmp.path().join("fixture.strict.vc.json");
        let strict_src_path = tmp.path().join("fixture.clear");
        let solver_path = if cfg!(windows) {
            tmp.path().join("fake-z3.exe")
        } else {
            tmp.path().join("fake-z3")
        };
        fs::copy(&src_path, &strict_src_path).expect("copy strict fixture source");
        let solver = write_fake_solver_to(&solver_path);
        write_solver_integrity_sidecars(&solver);
        fs::write(
            tmp.path().join("clg.lock.json"),
            r#"{"schema_version":0,"dependencies":[]}"#,
        )
        .expect("write strict lockfile");
        fs::write(
            tmp.path().join("clg.trust-policy.json"),
            r#"{
  "schema_version": 0,
  "trusted_signers": [
    {
      "key_id": "std-core-release-ed25519-2026q1",
      "scheme": "ed25519",
      "public_key": "hex:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "not_before": "2026-01-01T00:00:00Z",
      "not_after": "2027-01-01T00:00:00Z"
    }
  ],
  "revoked_key_ids": []
}"#,
        )
        .expect("write strict trust policy");
        fs::write(
            tmp.path().join("clg.host-profile.json"),
            r#"{
  "schema_version": 0,
  "profile": "contract_static",
  "capabilities": []
}"#,
        )
        .expect("write strict host profile");
        fs::write(
            tmp.path().join("clg.package-metadata.json"),
            r#"{
  "schema_version": 0,
  "packages": []
}"#,
        )
        .expect("write strict package metadata");
        fs::write(
            tmp.path().join("clg.package-abi.json"),
            r#"{
  "schema_version": 0,
  "contracts": []
}"#,
        )
        .expect("write strict package ABI");

        Command::cargo_bin("clg")
            .unwrap()
            .args(["build"])
            .arg(&src_path)
            .args(["-o"])
            .arg(&wasm_path)
            .args(["--emit-vcs"])
            .arg(&vcs_standard)
            .args(["--compiler-mode", "standard"])
            .assert()
            .success();

        let vc_raw = fs::read_to_string(&vcs_standard).expect("read vcs");
        let vcs: Value = serde_json::from_str(&vc_raw).expect("parse vcs json");
        let items = vcs.as_array().expect("vcs array");
        assert!(
            !items.is_empty(),
            "fixture `{}` produced zero vcs",
            fixture.id
        );
        for vc in items {
            let tier = vc
                .get("assurance")
                .and_then(|a| a.get("tier"))
                .and_then(|t| t.as_str())
                .unwrap_or_else(|| {
                    panic!("fixture `{}` has VC missing assurance.tier", fixture.id)
                });
            let rank = tier_rank(tier).unwrap_or_else(|| {
                panic!("fixture `{}` has unknown tier `{}`", fixture.id, tier);
            });
            assert!(
                rank >= min_rank,
                "fixture `{}` regressed tier: expected >= {}, found {}",
                fixture.id,
                expected.min_tier,
                tier
            );
            if rank >= 1 {
                assert!(
                    vc.get("assumptions").is_none(),
                    "fixture `{}` has unexpected assumptions at tier {}",
                    fixture.id,
                    tier
                );
            }
        }

        Command::cargo_bin("clg")
            .unwrap()
            .env("CLG_SOLVER_BIN", &solver)
            .args(["build"])
            .arg(&strict_src_path)
            .args(["-o"])
            .arg(&wasm_path)
            .args(["--emit-vcs"])
            .arg(&vcs_strict)
            .args(["--compiler-mode", "strict"])
            .assert()
            .success();
    }
}
