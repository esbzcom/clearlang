use std::fs;
use std::path::PathBuf;

use serde_json::Value;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

#[test]
fn phase26_list_performance_machine_contract_is_valid() {
    let root = repo_root();
    let json_path = root
        .join("docs")
        .join("evidence")
        .join("phase-26.3-list-performance.json");
    let raw = fs::read_to_string(json_path).expect("read list performance json");
    let parsed: Value = serde_json::from_str(&raw).expect("parse list performance json");

    assert_eq!(parsed.get("schema_version").and_then(Value::as_u64), Some(2));
    assert_eq!(parsed.get("gate").and_then(Value::as_str), Some("26.3.7"));
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
    let names: Vec<&str> = fixtures.iter().filter_map(Value::as_str).collect();
    assert!(names.contains(&"list_readonly"));
    assert!(names.contains(&"list_append_pop"));
    assert!(names.contains(&"list_indexed_mutation"));
    assert!(names.contains(&"list_checked_mutation"));

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
    assert_eq!(
        per_fixture.len(),
        names.len(),
        "each fixture must have a measured record"
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
