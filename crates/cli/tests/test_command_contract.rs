use assert_cmd::prelude::*;
use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::tempdir;

fn write_test_file(path: &Path, body: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent");
    }
    fs::write(path, body).expect("write test source");
}

#[test]
fn test_command_json_report_returns_no_tests_status_when_none_discovered() {
    let tmp = tempdir().expect("tempdir");
    let root = tmp.path().join("project");
    fs::create_dir_all(root.join("tests").join("unit")).expect("create tests/unit");

    let output = Command::cargo_bin("clg")
        .expect("bin")
        .args(["test"])
        .arg(&root)
        .args(["--report", "json"])
        .output()
        .expect("run clg test");
    assert_eq!(output.status.code(), Some(0));

    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    let payload: Value = serde_json::from_str(stdout.trim()).expect("json summary");
    assert_eq!(
        payload.get("schema_version").and_then(|v| v.as_u64()),
        Some(1)
    );
    assert_eq!(
        payload.get("status").and_then(|v| v.as_str()),
        Some("no_tests")
    );
    assert_eq!(payload.get("discovered").and_then(|v| v.as_u64()), Some(0));
    assert_eq!(payload.get("selected").and_then(|v| v.as_u64()), Some(0));
}

#[test]
fn test_command_discovery_is_deterministic_and_filter_is_applied_to_test_id() {
    let tmp = tempdir().expect("tempdir");
    let root = tmp.path().join("project");
    let unit = root.join("tests").join("unit");
    fs::create_dir_all(&unit).expect("create tests/unit");
    write_test_file(
        &unit.join("b.clear"),
        "function test_b() -> Bool { true }\n",
    );
    write_test_file(
        &unit.join("a.clear"),
        "function test_a() -> Bool { true }\n",
    );

    let output_all = Command::cargo_bin("clg")
        .expect("bin")
        .args(["test"])
        .arg(&root)
        .args(["--report", "json"])
        .output()
        .expect("run clg test all");
    assert_eq!(output_all.status.code(), Some(0));
    let all_stdout = String::from_utf8(output_all.stdout).expect("stdout utf8");
    let all_payload: Value = serde_json::from_str(all_stdout.trim()).expect("json summary");
    let ids: Vec<&str> = all_payload["tests"]
        .as_array()
        .expect("tests array")
        .iter()
        .map(|case| case["id"].as_str().expect("id str"))
        .collect();
    assert_eq!(
        ids,
        vec!["tests/unit/a.clear::test_a", "tests/unit/b.clear::test_b",]
    );

    let output_filtered = Command::cargo_bin("clg")
        .expect("bin")
        .args(["test"])
        .arg(&root)
        .args(["--filter", "test_b", "--report", "json"])
        .output()
        .expect("run clg test filter");
    assert_eq!(output_filtered.status.code(), Some(0));
    let filtered_stdout = String::from_utf8(output_filtered.stdout).expect("stdout utf8");
    let filtered_payload: Value =
        serde_json::from_str(filtered_stdout.trim()).expect("json summary");
    assert_eq!(
        filtered_payload
            .get("selected")
            .and_then(|value| value.as_u64()),
        Some(1)
    );
    let filtered_ids: Vec<&str> = filtered_payload["tests"]
        .as_array()
        .expect("tests array")
        .iter()
        .map(|case| case["id"].as_str().expect("id str"))
        .collect();
    assert_eq!(filtered_ids, vec!["tests/unit/b.clear::test_b"]);
}

#[test]
fn test_command_unknown_mock_set_in_plan_fails_with_c136() {
    let tmp = tempdir().expect("tempdir");
    let root = tmp.path().join("project");
    let tests_root = root.join("tests");
    let unit = tests_root.join("unit");
    fs::create_dir_all(&unit).expect("create tests/unit");
    write_test_file(
        &unit.join("discount_tests.clear"),
        "function test_discount() -> Bool { true }\n",
    );
    fs::write(
        tests_root.join("test-plan.json"),
        r#"{
  "schema_version": 1,
  "default_mock_sets": ["missing"],
  "cases": [
    {
      "test_id": "tests/unit/discount_tests.clear::test_discount",
      "mock_sets": ["missing"]
    }
  ]
}"#,
    )
    .expect("write test-plan");

    let output = Command::cargo_bin("clg")
        .expect("bin")
        .args(["--json-errors", "test"])
        .arg(&root)
        .output()
        .expect("run clg test");
    assert_eq!(output.status.code(), Some(1));

    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    let payload: Value = serde_json::from_str(stdout.trim()).expect("json errors");
    assert_eq!(payload.get("ok").and_then(|v| v.as_bool()), Some(false));
    let errors = payload
        .get("errors")
        .and_then(|v| v.as_array())
        .expect("errors array");
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].get("code").and_then(|v| v.as_str()), Some("C136"));
    assert_eq!(
        errors[0].get("stage").and_then(|v| v.as_str()),
        Some("test")
    );
}
