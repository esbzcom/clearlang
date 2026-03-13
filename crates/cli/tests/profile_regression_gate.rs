use assert_cmd::prelude::*;
use serde::Deserialize;
use serde_json::Value;
use std::fs;
use std::path::PathBuf;
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
        fs::copy(&src_path, &strict_src_path).expect("copy strict fixture source");
        fs::write(
            tmp.path().join("clg.lock.json"),
            r#"{"schema_version":0,"dependencies":[]}"#,
        )
        .expect("write strict lockfile");

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
