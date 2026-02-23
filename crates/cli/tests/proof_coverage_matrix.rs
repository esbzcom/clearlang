use serde::Deserialize;
use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

#[derive(Deserialize)]
struct CoverageMatrix {
    version: u32,
    entries: Vec<CoverageEntry>,
}

#[derive(Deserialize)]
struct CoverageEntry {
    id: String,
    kind: String,
    surface: String,
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

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

fn allowed_tier_value(value: &str) -> bool {
    matches!(value, "proved" | "assumed" | "blocked")
}

#[test]
fn proof_coverage_matrix_is_complete_and_stable() {
    let matrix_path = repo_root()
        .join("docs")
        .join("proofs")
        .join("proof-coverage-matrix.json");
    let raw = fs::read_to_string(&matrix_path).expect("read proof coverage matrix");
    let matrix: CoverageMatrix = serde_json::from_str(&raw).expect("parse proof coverage matrix");

    assert_eq!(matrix.version, 1, "matrix version should be 1");
    assert!(!matrix.entries.is_empty(), "matrix must include entries");

    let mut sorted_ids = matrix
        .entries
        .iter()
        .map(|entry| entry.id.as_str())
        .collect::<Vec<_>>();
    let original_ids = sorted_ids.clone();
    sorted_ids.sort_unstable();
    assert_eq!(
        original_ids, sorted_ids,
        "matrix entries must be sorted by id for deterministic diffs"
    );
    let unique_count = original_ids.iter().collect::<BTreeSet<_>>().len();
    assert_eq!(
        unique_count,
        original_ids.len(),
        "matrix ids must be unique"
    );

    let allowed_assumptions = BTreeSet::from([
        "unsigned.int_model",
        "bitwise.uninterpreted",
        "crypto.uninterpreted",
    ]);
    let mut required_surfaces = BTreeSet::from([
        "std::bytes::eq_ct",
        "std::crypto::hash",
        "std::crypto::hmac",
        "std::crypto::verify",
        "std::u64::add_wrap",
        "std::u64::sub_wrap",
        "std::u64::mul_wrap",
        "std::u64::add_sat",
        "std::u64::sub_sat",
        "std::u64::mul_sat",
        "std::u64::rotl",
        "std::u64::rotr",
        "std::u64::to_bytes_le",
        "std::u64::to_bytes_be",
        "std::u64::from_bytes_le",
        "std::u64::from_bytes_be",
        "std::u128::from_limbs",
        "std::u128::lo",
        "std::u128::hi",
        "std::u256::from_limbs",
        "std::u256::limb0",
        "std::u256::limb1",
        "std::u256::limb2",
        "std::u256::limb3",
    ]);

    let mut saw_feature = false;
    let mut saw_intrinsic = false;

    for entry in &matrix.entries {
        assert!(
            !entry.surface.trim().is_empty(),
            "surface must be non-empty: {}",
            entry.id
        );
        assert!(
            matches!(entry.kind.as_str(), "feature" | "intrinsic"),
            "unknown kind for {}: {}",
            entry.id,
            entry.kind
        );
        if entry.kind == "feature" {
            saw_feature = true;
        }
        if entry.kind == "intrinsic" {
            saw_intrinsic = true;
            required_surfaces.remove(entry.surface.as_str());
        }

        assert!(
            matches!(entry.status.as_str(), "proved" | "assumed"),
            "unknown status for {}: {}",
            entry.id,
            entry.status
        );

        for tier in [&entry.tiers.l0, &entry.tiers.l1, &entry.tiers.l2, &entry.tiers.l3] {
            assert!(
                allowed_tier_value(tier),
                "invalid tier value `{}` for {}",
                tier,
                entry.id
            );
        }

        match entry.status.as_str() {
            "proved" => {
                assert!(
                    entry.assumption_boundary.is_none(),
                    "proved entry must not carry assumption boundary: {}",
                    entry.id
                );
                assert_eq!(entry.tiers.l0, "proved");
                assert_eq!(entry.tiers.l1, "proved");
                assert_eq!(entry.tiers.l2, "proved");
                assert_eq!(entry.tiers.l3, "proved");
            }
            "assumed" => {
                let boundary = entry
                    .assumption_boundary
                    .as_deref()
                    .unwrap_or("<missing-assumption-boundary>");
                assert!(
                    allowed_assumptions.contains(boundary),
                    "assumed entry must use a known assumption boundary: {} -> {}",
                    entry.id,
                    boundary
                );
                assert_eq!(entry.tiers.l0, "assumed");
                assert_eq!(entry.tiers.l1, "assumed");
                assert_eq!(entry.tiers.l2, "assumed");
                assert_eq!(entry.tiers.l3, "blocked");
            }
            _ => unreachable!(),
        }
    }

    assert!(saw_feature, "matrix must include feature rows");
    assert!(saw_intrinsic, "matrix must include intrinsic rows");
    assert!(
        required_surfaces.is_empty(),
        "matrix missing required intrinsic surfaces: {:?}",
        required_surfaces
    );
}
