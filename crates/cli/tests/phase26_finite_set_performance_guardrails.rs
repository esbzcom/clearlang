use std::fs;
use std::path::PathBuf;

use serde_json::Value;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

#[test]
fn phase26_finite_set_performance_evidence_is_published() {
    let root = repo_root();
    let doc_path = root
        .join("docs")
        .join("evidence")
        .join("phase-26.2-finite-set-performance.md");
    let doc = fs::read_to_string(doc_path).expect("read finite-set performance evidence doc");
    assert!(doc.contains("<= +10%"));
    assert!(doc.contains("<= +20%"));
    assert!(doc.contains("0"));
    assert!(doc.contains("set_subset_membership"));
    assert!(doc.contains("set_algebra"));
    assert!(doc.contains("set_cardinality"));
}

#[test]
fn phase26_finite_set_performance_machine_contract_is_valid() {
    let root = repo_root();
    let json_path = root
        .join("docs")
        .join("evidence")
        .join("phase-26.2-finite-set-performance.json");
    let raw = fs::read_to_string(json_path).expect("read finite-set performance json");
    let parsed: Value = serde_json::from_str(&raw).expect("parse finite-set performance json");

    assert_eq!(
        parsed.get("schema_version").and_then(Value::as_u64),
        Some(1)
    );
    assert_eq!(parsed.get("gate").and_then(Value::as_str), Some("26.2.5"));
    assert_eq!(
        parsed.get("aggregation_mode").and_then(Value::as_str),
        Some("pinned-baseline-regression")
    );

    let thresholds = parsed
        .get("thresholds")
        .and_then(Value::as_object)
        .expect("thresholds object");
    assert_eq!(
        thresholds
            .get("median_regression_pct_max")
            .and_then(Value::as_u64),
        Some(10)
    );
    assert_eq!(
        thresholds
            .get("p95_regression_pct_max")
            .and_then(Value::as_u64),
        Some(20)
    );
    assert_eq!(
        thresholds.get("timeout_count_max").and_then(Value::as_u64),
        Some(0)
    );

    let fixtures = parsed
        .get("fixtures")
        .and_then(Value::as_array)
        .expect("fixtures array");
    let names: Vec<&str> = fixtures
        .iter()
        .filter_map(Value::as_str)
        .collect();
    assert!(names.contains(&"set_subset_membership"));
    assert!(names.contains(&"set_algebra"));
    assert!(names.contains(&"set_cardinality"));
}
