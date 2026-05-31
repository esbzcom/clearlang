use std::fs;
use std::path::PathBuf;

use serde_json::Value;
use sha2::{Digest, Sha256};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

#[test]
fn phase26_map_performance_machine_contract_is_valid() {
    let root = repo_root();
    let json_path = root
        .join("docs")
        .join("evidence")
        .join("phase-26.4-map-performance.json");
    let raw = fs::read_to_string(json_path).expect("read map performance json");
    let parsed: Value = serde_json::from_str(&raw).expect("parse map performance json");

    assert_eq!(
        parsed.get("schema_version").and_then(Value::as_u64),
        Some(2)
    );
    assert_eq!(parsed.get("gate").and_then(Value::as_str), Some("26.4.6"));
    assert_eq!(
        parsed.get("aggregation_mode").and_then(Value::as_str),
        Some("pinned-baseline-regression")
    );
    let measurement_artifact = parsed
        .get("measurement_artifact")
        .and_then(Value::as_object)
        .expect("measurement_artifact object");
    assert_eq!(
        measurement_artifact
            .get("schema_version")
            .and_then(Value::as_u64),
        Some(1),
        "raw measurement artifact schema version must be pinned"
    );
    let raw_path_rel = measurement_artifact
        .get("path")
        .and_then(Value::as_str)
        .expect("measurement_artifact.path");
    let expected_raw_sha = measurement_artifact
        .get("sha256")
        .and_then(Value::as_str)
        .expect("measurement_artifact.sha256");
    let raw_path = root.join(raw_path_rel);
    let raw_bytes = fs::read(&raw_path).expect("read raw measurement artifact");
    let actual_raw_sha = hex::encode(Sha256::digest(&raw_bytes));
    assert_eq!(
        actual_raw_sha, expected_raw_sha,
        "raw measurement artifact hash mismatch"
    );
    let raw: Value = serde_json::from_slice(&raw_bytes).expect("parse raw measurement artifact");
    assert_eq!(raw.get("schema_version").and_then(Value::as_u64), Some(1));
    assert_eq!(raw.get("gate").and_then(Value::as_str), Some("26.4.6"));

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
    let names: Vec<&str> = fixtures.iter().filter_map(Value::as_str).collect();
    assert!(names.contains(&"map_readonly"));
    assert!(names.contains(&"map_membership_overwrite"));
    assert!(names.contains(&"map_mutation_take"));

    let measurements = parsed
        .get("measurements")
        .and_then(Value::as_object)
        .expect("measurements object");
    assert!(
        measurements
            .get("generated_on")
            .and_then(Value::as_str)
            .is_some_and(|s| !s.is_empty()),
        "measurements.generated_on must be present"
    );

    let per_fixture = measurements
        .get("per_fixture")
        .and_then(Value::as_array)
        .expect("measurements.per_fixture array");
    let raw_fixtures = raw
        .get("fixtures")
        .and_then(Value::as_array)
        .expect("raw fixtures array");
    assert_eq!(
        per_fixture.len(),
        names.len(),
        "each fixture must have a measured record"
    );
    assert_eq!(
        raw_fixtures.len(),
        names.len(),
        "raw fixture count must match fixture list"
    );

    let median_threshold = thresholds
        .get("median_regression_pct_max")
        .and_then(Value::as_u64)
        .expect("median threshold");
    let p95_threshold = thresholds
        .get("p95_regression_pct_max")
        .and_then(Value::as_u64)
        .expect("p95 threshold");
    let timeout_threshold = thresholds
        .get("timeout_count_max")
        .and_then(Value::as_u64)
        .expect("timeout threshold");

    let mut max_median = 0_u64;
    let mut max_p95 = 0_u64;
    let mut total_timeouts = 0_u64;
    for item in per_fixture {
        let obj = item.as_object().expect("fixture measurement object");
        let name = obj
            .get("name")
            .and_then(Value::as_str)
            .expect("fixture measurement name");
        assert!(
            names.contains(&name),
            "fixture measurement name must match fixture list"
        );

        let baseline = obj
            .get("baseline_ms")
            .and_then(Value::as_object)
            .expect("baseline_ms object");
        let current = obj
            .get("current_ms")
            .and_then(Value::as_object)
            .expect("current_ms object");
        let regression = obj
            .get("regression_pct")
            .and_then(Value::as_object)
            .expect("regression_pct object");
        let timeout_count = obj
            .get("timeout_count")
            .and_then(Value::as_u64)
            .expect("timeout_count");
        let raw_fixture = raw_fixtures
            .iter()
            .find(|entry| entry.get("name").and_then(Value::as_str) == Some(name))
            .and_then(Value::as_object)
            .expect("matching raw fixture entry");
        let raw_baseline = raw_fixture
            .get("baseline_ms")
            .and_then(Value::as_object)
            .expect("raw baseline_ms object");
        let raw_current = raw_fixture
            .get("current_ms")
            .and_then(Value::as_object)
            .expect("raw current_ms object");
        let raw_timeout = raw_fixture
            .get("timeout_count")
            .and_then(Value::as_u64)
            .expect("raw timeout_count");

        assert!(
            baseline.get("median").and_then(Value::as_u64).unwrap_or(0) > 0,
            "baseline median must be > 0"
        );
        assert!(
            baseline.get("p95").and_then(Value::as_u64).unwrap_or(0) > 0,
            "baseline p95 must be > 0"
        );
        assert!(
            current.get("median").and_then(Value::as_u64).unwrap_or(0) > 0,
            "current median must be > 0"
        );
        assert!(
            current.get("p95").and_then(Value::as_u64).unwrap_or(0) > 0,
            "current p95 must be > 0"
        );
        assert_eq!(
            raw_baseline.get("median").and_then(Value::as_u64),
            baseline.get("median").and_then(Value::as_u64),
            "summary baseline median must match raw capture"
        );
        assert_eq!(
            raw_baseline.get("p95").and_then(Value::as_u64),
            baseline.get("p95").and_then(Value::as_u64),
            "summary baseline p95 must match raw capture"
        );
        assert_eq!(
            raw_current.get("median").and_then(Value::as_u64),
            current.get("median").and_then(Value::as_u64),
            "summary current median must match raw capture"
        );
        assert_eq!(
            raw_current.get("p95").and_then(Value::as_u64),
            current.get("p95").and_then(Value::as_u64),
            "summary current p95 must match raw capture"
        );
        assert_eq!(
            raw_timeout, timeout_count,
            "summary timeout count must match raw capture"
        );

        let median_reg = regression
            .get("median")
            .and_then(Value::as_u64)
            .expect("median regression");
        let p95_reg = regression
            .get("p95")
            .and_then(Value::as_u64)
            .expect("p95 regression");
        assert!(
            median_reg <= median_threshold,
            "fixture median regression exceeds threshold"
        );
        assert!(
            p95_reg <= p95_threshold,
            "fixture p95 regression exceeds threshold"
        );
        assert!(
            timeout_count <= timeout_threshold,
            "fixture timeout count exceeds threshold"
        );

        max_median = max_median.max(median_reg);
        max_p95 = max_p95.max(p95_reg);
        total_timeouts += timeout_count;
    }

    let aggregate = measurements
        .get("aggregate")
        .and_then(Value::as_object)
        .expect("measurements.aggregate object");
    assert_eq!(
        aggregate
            .get("max_median_regression_pct")
            .and_then(Value::as_u64),
        Some(max_median)
    );
    assert_eq!(
        aggregate
            .get("max_p95_regression_pct")
            .and_then(Value::as_u64),
        Some(max_p95)
    );
    assert_eq!(
        aggregate.get("total_timeouts").and_then(Value::as_u64),
        Some(total_timeouts)
    );
}
