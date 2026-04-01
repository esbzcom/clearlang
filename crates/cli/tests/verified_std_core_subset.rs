use assert_cmd::prelude::*;
use ed25519_dalek::{Signer, SigningKey};
use serde::Deserialize;
use serde_json::json;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::tempdir;

#[derive(Deserialize)]
struct CoverageMatrix {
    entries: Vec<CoverageEntry>,
}

#[derive(Deserialize)]
struct CoverageEntry {
    id: String,
    kind: String,
    status: String,
    assumption_boundary: Option<String>,
    tiers: CoverageTiers,
}

#[derive(Deserialize)]
struct CoverageTiers {
    #[serde(rename = "L0")]
    l0: String,
    #[serde(rename = "L1")]
    l1: String,
    #[serde(rename = "L2")]
    l2: String,
    #[serde(rename = "L3")]
    l3: String,
}

#[derive(Deserialize)]
struct VerifiedSubset {
    version: u32,
    profile: String,
    entries: Vec<SubsetEntry>,
    regression_obligations: Vec<RegressionObligation>,
}

#[derive(Deserialize)]
struct SubsetEntry {
    coverage_id: String,
    summary: String,
}

#[derive(Deserialize)]
struct RegressionObligation {
    coverage_id: String,
    test_file: String,
    test_name: String,
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

fn file_or_includes_contains_signature(
    path: &Path,
    signature: &str,
    visited: &mut BTreeSet<PathBuf>,
) -> bool {
    let normalized = path.to_path_buf();
    if !visited.insert(normalized.clone()) {
        return false;
    }
    let Ok(content) = fs::read_to_string(&normalized) else {
        return false;
    };
    if content.contains(signature) {
        return true;
    }

    for line in content.lines() {
        let trimmed = line.trim();
        let Some(rest) = trimmed.strip_prefix("include!(\"") else {
            continue;
        };
        let Some(rel_path) = rest.strip_suffix("\");") else {
            continue;
        };
        let include_path = normalized
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(rel_path);
        if file_or_includes_contains_signature(include_path.as_path(), signature, visited) {
            return true;
        }
    }
    false
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

    let signing = SigningKey::from_bytes(&[7u8; 32]);
    let signature = hex::encode(signing.sign(checksum.as_bytes()).to_bytes());
    let signature_path = solver.with_file_name(format!(
        "{}.sig",
        solver
            .file_name()
            .expect("solver filename")
            .to_string_lossy()
    ));
    fs::write(
        &signature_path,
        serde_json::to_vec_pretty(&json!({
            "schema_version": 1,
            "key_id": "z3-vendor-k7-2026q2",
            "scheme": "ed25519",
            "signed_payload": checksum,
            "signature": signature
        }))
        .expect("serialize signature sidecar"),
    )
    .expect("write signature sidecar");
}

#[test]
fn verified_std_core_subset_maps_to_proved_coverage_and_real_tests() {
    let root = repo_root();
    let matrix_path = root
        .join("docs")
        .join("proofs")
        .join("proof-coverage-matrix.json");
    let subset_path = root
        .join("docs")
        .join("proofs")
        .join("verified-std-core-subset.json");

    let matrix_raw = fs::read_to_string(&matrix_path).expect("read matrix");
    let subset_raw = fs::read_to_string(&subset_path).expect("read subset profile");
    let matrix: CoverageMatrix = serde_json::from_str(&matrix_raw).expect("parse matrix");
    let subset: VerifiedSubset = serde_json::from_str(&subset_raw).expect("parse subset profile");

    assert_eq!(subset.version, 1, "subset profile version should be 1");
    assert_eq!(subset.profile, "verified.std_core.v1");
    assert!(
        !subset.entries.is_empty(),
        "subset entries should not be empty"
    );

    let mut subset_ids: Vec<&str> = subset
        .entries
        .iter()
        .map(|entry| entry.coverage_id.as_str())
        .collect();
    let original_ids = subset_ids.clone();
    subset_ids.sort_unstable();
    assert_eq!(
        original_ids, subset_ids,
        "subset entries must stay sorted by coverage_id"
    );
    let unique_subset_ids = original_ids.iter().collect::<BTreeSet<_>>();
    assert_eq!(
        unique_subset_ids.len(),
        original_ids.len(),
        "subset coverage IDs must be unique"
    );

    let mut proved_feature_ids: Vec<&str> = matrix
        .entries
        .iter()
        .filter(|entry| entry.kind == "feature" && entry.status == "proved")
        .map(|entry| entry.id.as_str())
        .collect();
    proved_feature_ids.sort_unstable();

    assert_eq!(
        subset_ids, proved_feature_ids,
        "subset must track all current proved feature rows from coverage matrix"
    );

    for entry in &subset.entries {
        assert!(
            !entry.summary.trim().is_empty(),
            "subset summary must be non-empty for {}",
            entry.coverage_id
        );
        let matrix_entry = matrix
            .entries
            .iter()
            .find(|item| item.id == entry.coverage_id)
            .unwrap_or_else(|| panic!("missing matrix entry for {}", entry.coverage_id));
        assert_eq!(matrix_entry.status, "proved");
        assert!(
            matrix_entry.assumption_boundary.is_none(),
            "subset entry must not have assumption boundary: {}",
            entry.coverage_id
        );
        assert_eq!(matrix_entry.tiers.l0, "proved");
        assert_eq!(matrix_entry.tiers.l1, "proved");
        assert_eq!(matrix_entry.tiers.l2, "proved");
        assert_eq!(matrix_entry.tiers.l3, "proved");
    }

    assert!(
        !subset.regression_obligations.is_empty(),
        "subset must declare regression obligations"
    );
    let subset_id_set = subset_ids.into_iter().collect::<BTreeSet<_>>();
    let mut obligations_per_id = std::collections::BTreeMap::<&str, usize>::new();
    for obligation in &subset.regression_obligations {
        assert!(
            subset_id_set.contains(obligation.coverage_id.as_str()),
            "obligation coverage_id not present in subset: {}",
            obligation.coverage_id
        );
        *obligations_per_id
            .entry(obligation.coverage_id.as_str())
            .or_insert(0) += 1;

        let test_path = root.join(&obligation.test_file);
        assert!(
            test_path.exists(),
            "obligation test file does not exist: {}",
            obligation.test_file
        );
        let signature = format!("fn {}(", obligation.test_name);
        let mut visited = BTreeSet::new();
        assert!(
            file_or_includes_contains_signature(test_path.as_path(), &signature, &mut visited),
            "obligation test function not found: {} in {}",
            obligation.test_name,
            obligation.test_file
        );
    }

    for coverage_id in &subset_id_set {
        assert!(
            obligations_per_id.contains_key(coverage_id),
            "missing regression obligation for subset entry: {}",
            coverage_id
        );
    }
}

#[test]
fn strict_profile_core_fixture_emits_vcs_without_assumptions() {
    let tmp = tempdir().unwrap();
    let src_path = tmp.path().join("verified_std_core.clear");
    let wasm_path = tmp.path().join("verified_std_core.wasm");
    let vcs_path = tmp.path().join("verified_std_core.vc.json");
    let solver_path = if cfg!(windows) {
        tmp.path().join("fake-z3.exe")
    } else {
        tmp.path().join("fake-z3")
    };
    let src = r#"
        type Nat = Int where v >= 0;

        pure function id_nat(x: Nat) -> Nat { x }

        pure function step(n: Int) -> Int
            require { n >= 0 }
            ensure { result >= 0 }
        {
            if n > 0 { n - 1 } else { n }
        }

        pure function countdown(n: Int) -> Int
            require { n >= 0 }
            ensure { result >= 0 }
        {
            while n > 0 invariant { n >= 0 } variant { n } {
                n;
            }
            n
        }

        function main() -> Int { countdown(step(3)) }
    "#;
    fs::write(&src_path, src).expect("write source");
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
    let solver = write_fake_solver_to(&solver_path);
    write_solver_integrity_sidecars(&solver);

    Command::cargo_bin("clg")
        .unwrap()
        .env("CLG_SOLVER_BIN", &solver)
        .args(["build"])
        .arg(&src_path)
        .args(["-o"])
        .arg(&wasm_path)
        .args(["--emit-vcs"])
        .arg(&vcs_path)
        .args(["--compiler-mode", "strict"])
        .assert()
        .success();

    let vc_raw = fs::read_to_string(&vcs_path).expect("read vcs");
    let vcs: Value = serde_json::from_str(&vc_raw).expect("parse vcs json");
    let items = vcs.as_array().expect("vcs array");
    assert!(!items.is_empty(), "expected emitted vcs");
    for vc in items {
        assert!(
            vc.get("assumptions").is_none(),
            "strict core fixture should not emit assumptions: {:?}",
            vc.get("assumptions")
        );
    }
}
