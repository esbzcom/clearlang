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

fn parse_json_lines(lines: &str) -> Vec<Value> {
    lines
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str::<Value>(line).expect("json line"))
        .collect()
}

#[cfg(unix)]
fn create_dir_link(link: &Path, target: &Path) {
    std::os::unix::fs::symlink(target, link).expect("create symlink dir");
}

#[cfg(windows)]
fn create_dir_link(link: &Path, target: &Path) {
    let link_str = link.to_string_lossy().into_owned();
    let target_str = target.to_string_lossy().into_owned();
    let output = Command::new("cmd")
        .args(["/C", "mklink", "/J", link_str.as_str(), target_str.as_str()])
        .output()
        .expect("create junction");
    assert!(
        output.status.success(),
        "mklink /J failed: stdout=`{}` stderr=`{}`",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_command_layout_error_reports_c134() {
    let tmp = tempdir().expect("tempdir");
    let root = tmp.path().join("project");
    fs::create_dir_all(&root).expect("create project root");

    let output = Command::cargo_bin("clg")
        .expect("bin")
        .args(["--json-errors", "test"])
        .arg(&root)
        .output()
        .expect("run clg test");
    assert_eq!(output.status.code(), Some(1));

    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    let payload: Value = serde_json::from_str(stdout.trim()).expect("json errors");
    let errors = payload
        .get("errors")
        .and_then(|v| v.as_array())
        .expect("errors array");
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].get("code").and_then(|v| v.as_str()), Some("C134"));
    assert!(
        errors[0]
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .contains("tests/unit"),
        "expected deterministic layout error for missing tests/unit"
    );
}

#[test]
fn test_command_invalid_signature_reports_c134() {
    let tmp = tempdir().expect("tempdir");
    let root = tmp.path().join("project");
    let unit = root.join("tests").join("unit");
    fs::create_dir_all(&unit).expect("create tests/unit");
    write_test_file(
        &unit.join("bad_signature.clear"),
        "function test_bad(arg: Int) -> Bool { true }\n",
    );

    let output = Command::cargo_bin("clg")
        .expect("bin")
        .args(["--json-errors", "test"])
        .arg(&root)
        .output()
        .expect("run clg test");
    assert_eq!(output.status.code(), Some(1));

    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    let payload: Value = serde_json::from_str(stdout.trim()).expect("json errors");
    let errors = payload
        .get("errors")
        .and_then(|v| v.as_array())
        .expect("errors array");
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].get("code").and_then(|v| v.as_str()), Some("C134"));
    assert!(
        errors[0]
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .contains("must have signature `() -> Bool`"),
        "expected deterministic signature error for test_* contract"
    );
}

#[test]
fn test_command_duplicate_discovered_test_id_reports_c134() {
    let tmp = tempdir().expect("tempdir");
    let root = tmp.path().join("project");
    let unit = root.join("tests").join("unit");
    fs::create_dir_all(&unit).expect("create tests/unit");
    write_test_file(
        &unit.join("dup.clear"),
        "function test_dup() -> Bool { true }\nfunction test_dup() -> Bool { true }\n",
    );

    let output = Command::cargo_bin("clg")
        .expect("bin")
        .args(["--json-errors", "test"])
        .arg(&root)
        .output()
        .expect("run clg test");
    assert_eq!(output.status.code(), Some(1));

    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    let payload: Value = serde_json::from_str(stdout.trim()).expect("json errors");
    let errors = payload
        .get("errors")
        .and_then(|v| v.as_array())
        .expect("errors array");
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].get("code").and_then(|v| v.as_str()), Some("C134"));
    assert!(
        errors[0]
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .contains("duplicate discovered test id"),
        "expected deterministic duplicate-id discovery error"
    );
}

#[test]
fn test_command_plan_schema_error_reports_c135() {
    let tmp = tempdir().expect("tempdir");
    let root = tmp.path().join("project");
    let tests_root = root.join("tests");
    let unit = tests_root.join("unit");
    fs::create_dir_all(&unit).expect("create tests/unit");
    write_test_file(
        &unit.join("schema.clear"),
        "function test_schema() -> Bool { true }\n",
    );
    fs::write(
        tests_root.join("test-plan.json"),
        r#"{
  "schema_version": 2,
  "default_mock_sets": [],
  "cases": []
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
    let errors = payload
        .get("errors")
        .and_then(|v| v.as_array())
        .expect("errors array");
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].get("code").and_then(|v| v.as_str()), Some("C135"));
    assert!(
        errors[0]
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .contains("unsupported schema_version"),
        "expected deterministic plan-governance schema error"
    );
}

#[test]
fn test_command_plan_unsorted_case_order_reports_c135() {
    let tmp = tempdir().expect("tempdir");
    let root = tmp.path().join("project");
    let tests_root = root.join("tests");
    let unit = tests_root.join("unit");
    fs::create_dir_all(&unit).expect("create tests/unit");
    write_test_file(
        &unit.join("a.clear"),
        "function test_a() -> Bool { true }\n",
    );
    write_test_file(
        &unit.join("b.clear"),
        "function test_b() -> Bool { true }\n",
    );
    fs::write(
        tests_root.join("test-plan.json"),
        r#"{
  "schema_version": 1,
  "default_mock_sets": [],
  "cases": [
    {
      "test_id": "tests/unit/b.clear::test_b",
      "mock_sets": []
    },
    {
      "test_id": "tests/unit/a.clear::test_a",
      "mock_sets": []
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
    let errors = payload
        .get("errors")
        .and_then(|v| v.as_array())
        .expect("errors array");
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].get("code").and_then(|v| v.as_str()), Some("C135"));
    assert!(
        errors[0]
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .contains("cases must be sorted by test_id"),
        "expected deterministic sorted-order governance error"
    );
}

#[test]
fn test_command_plan_duplicate_test_id_reports_c135() {
    let tmp = tempdir().expect("tempdir");
    let root = tmp.path().join("project");
    let tests_root = root.join("tests");
    let unit = tests_root.join("unit");
    fs::create_dir_all(&unit).expect("create tests/unit");
    write_test_file(
        &unit.join("dup_plan.clear"),
        "function test_dup_plan() -> Bool { true }\n",
    );
    fs::write(
        tests_root.join("test-plan.json"),
        r#"{
  "schema_version": 1,
  "default_mock_sets": [],
  "cases": [
    {
      "test_id": "tests/unit/dup_plan.clear::test_dup_plan",
      "mock_sets": []
    },
    {
      "test_id": "tests/unit/dup_plan.clear::test_dup_plan",
      "mock_sets": []
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
    let errors = payload
        .get("errors")
        .and_then(|v| v.as_array())
        .expect("errors array");
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].get("code").and_then(|v| v.as_str()), Some("C135"));
    assert!(
        errors[0]
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .contains("duplicate test_id"),
        "expected deterministic duplicate-id plan governance error"
    );
}

#[test]
fn test_command_plan_unknown_test_id_reports_c135() {
    let tmp = tempdir().expect("tempdir");
    let root = tmp.path().join("project");
    let tests_root = root.join("tests");
    let unit = tests_root.join("unit");
    fs::create_dir_all(&unit).expect("create tests/unit");
    write_test_file(
        &unit.join("known.clear"),
        "function test_known() -> Bool { true }\n",
    );
    fs::write(
        tests_root.join("test-plan.json"),
        r#"{
  "schema_version": 1,
  "default_mock_sets": [],
  "cases": [
    {
      "test_id": "tests/unit/unknown.clear::test_unknown",
      "mock_sets": []
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
    let errors = payload
        .get("errors")
        .and_then(|v| v.as_array())
        .expect("errors array");
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].get("code").and_then(|v| v.as_str()), Some("C135"));
    assert!(
        errors[0]
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .contains("references unknown test_id"),
        "expected deterministic unknown-id plan governance error"
    );
}

#[test]
fn test_command_plan_timeout_zero_reports_c135() {
    let tmp = tempdir().expect("tempdir");
    let root = tmp.path().join("project");
    let tests_root = root.join("tests");
    let unit = tests_root.join("unit");
    fs::create_dir_all(&unit).expect("create tests/unit");
    write_test_file(
        &unit.join("timeout_zero.clear"),
        "function test_timeout_zero() -> Bool { true }\n",
    );
    fs::write(
        tests_root.join("test-plan.json"),
        r#"{
  "schema_version": 1,
  "default_mock_sets": [],
  "cases": [
    {
      "test_id": "tests/unit/timeout_zero.clear::test_timeout_zero",
      "mock_sets": [],
      "timeout_ms": 0
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
    let errors = payload
        .get("errors")
        .and_then(|v| v.as_array())
        .expect("errors array");
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].get("code").and_then(|v| v.as_str()), Some("C135"));
    assert!(
        errors[0]
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .contains("invalid timeout_ms=0"),
        "expected deterministic timeout floor governance error"
    );
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
    assert_eq!(payload.get("executed").and_then(|v| v.as_u64()), Some(0));
    assert_eq!(payload["tests"].as_array().map(|v| v.len()), Some(0));
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
    assert_eq!(
        all_payload.get("status").and_then(|v| v.as_str()),
        Some("ok")
    );
    assert_eq!(
        all_payload.get("executed").and_then(|v| v.as_u64()),
        Some(2)
    );
    assert_eq!(all_payload.get("passed").and_then(|v| v.as_u64()), Some(2));
    assert_eq!(all_payload.get("failed").and_then(|v| v.as_u64()), Some(0));
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
    let first = all_payload["tests"]
        .as_array()
        .expect("tests array")
        .first()
        .expect("first test");
    assert_eq!(first.get("status").and_then(|v| v.as_str()), Some("passed"));
    assert_eq!(
        first.get("failure_code").and_then(|v| v.as_str()),
        None,
        "passing cases must not include failure_code"
    );
    assert_eq!(
        first.get("captured_stdout").and_then(|v| v.as_str()),
        Some("")
    );
    assert_eq!(
        first.get("captured_stderr").and_then(|v| v.as_str()),
        Some("")
    );
    let replay_argv = first
        .get("replay")
        .and_then(|v| v.get("argv"))
        .and_then(|v| v.as_array())
        .expect("replay argv array");
    let replay_filter = replay_argv
        .iter()
        .position(|v| v.as_str() == Some("--filter"))
        .expect("--filter in replay argv");
    assert_eq!(
        replay_argv.get(replay_filter + 1).and_then(|v| v.as_str()),
        Some("tests/unit/a.clear::test_a")
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
    assert_eq!(
        filtered_payload
            .get("executed")
            .and_then(|value| value.as_u64()),
        Some(1)
    );
    assert_eq!(
        filtered_payload
            .get("passed")
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
fn test_command_fails_with_nonzero_exit_when_any_test_returns_false() {
    let tmp = tempdir().expect("tempdir");
    let root = tmp.path().join("project");
    let unit = root.join("tests").join("unit");
    fs::create_dir_all(&unit).expect("create tests/unit");
    write_test_file(
        &unit.join("logic.clear"),
        "function test_pass() -> Bool { true }\nfunction test_fail() -> Bool { false }\n",
    );

    let output = Command::cargo_bin("clg")
        .expect("bin")
        .args(["test"])
        .arg(&root)
        .args(["--report", "json"])
        .output()
        .expect("run clg test");
    assert_eq!(output.status.code(), Some(1));

    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    let payload: Value = serde_json::from_str(stdout.trim()).expect("json summary");
    assert_eq!(
        payload.get("status").and_then(|v| v.as_str()),
        Some("failed")
    );
    assert_eq!(payload.get("executed").and_then(|v| v.as_u64()), Some(2));
    assert_eq!(payload.get("passed").and_then(|v| v.as_u64()), Some(1));
    assert_eq!(payload.get("failed").and_then(|v| v.as_u64()), Some(1));
    let failed_case = payload["tests"]
        .as_array()
        .expect("tests array")
        .iter()
        .find(|case| {
            case.get("id").and_then(|v| v.as_str()) == Some("tests/unit/logic.clear::test_fail")
        })
        .expect("failed test case");
    assert_eq!(
        failed_case.get("status").and_then(|v| v.as_str()),
        Some("failed")
    );
    assert_eq!(
        failed_case.get("failure_kind").and_then(|v| v.as_str()),
        Some("assertion_false")
    );
    assert_eq!(
        failed_case.get("failure_code").and_then(|v| v.as_str()),
        Some("C139")
    );
    assert!(
        failed_case
            .get("reason")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .contains("assertion returned false"),
        "expected deterministic assertion failure reason"
    );
    assert_eq!(
        failed_case
            .get("captured_stdout")
            .and_then(|v| v.as_str())
            .unwrap_or_default(),
        ""
    );
    assert_eq!(
        failed_case
            .get("captured_stderr")
            .and_then(|v| v.as_str())
            .unwrap_or_default(),
        ""
    );
    let replay_argv = failed_case
        .get("replay")
        .and_then(|v| v.get("argv"))
        .and_then(|v| v.as_array())
        .expect("replay argv array");
    assert_eq!(
        replay_argv.first().and_then(|v| v.as_str()),
        Some("clg"),
        "replay must start with clg command token"
    );
}

#[test]
fn test_command_std_unit_assertions_map_to_bool_contract_deterministically() {
    let tmp = tempdir().expect("tempdir");
    let root = tmp.path().join("project");
    let unit = root.join("tests").join("unit");
    fs::create_dir_all(&unit).expect("create tests/unit");
    write_test_file(
        &unit.join("unit_assertions.clear"),
        "import std::unit as unit\n\nfunction test_pass() -> Bool {\n    unit::assert_true(true, \"assert_true pass\")\n        && unit::assert_eq_int(2 + 3, 5, \"assert_eq_int pass\")\n        && unit::assert_eq_bool(true, unit::assert_true(true, \"nested\"), \"assert_eq_bool pass\")\n}\n\nfunction test_fail() -> Bool {\n    unit::fail(\"forced failure\")\n}\n",
    );

    let output = Command::cargo_bin("clg")
        .expect("bin")
        .args(["test"])
        .arg(&root)
        .args(["--report", "json"])
        .output()
        .expect("run clg test");
    assert_eq!(output.status.code(), Some(1));

    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    let payload: Value = serde_json::from_str(stdout.trim()).expect("json summary");
    assert_eq!(
        payload.get("status").and_then(|v| v.as_str()),
        Some("failed")
    );
    assert_eq!(payload.get("executed").and_then(|v| v.as_u64()), Some(2));
    assert_eq!(payload.get("passed").and_then(|v| v.as_u64()), Some(1));
    assert_eq!(payload.get("failed").and_then(|v| v.as_u64()), Some(1));

    let cases = payload
        .get("tests")
        .and_then(|v| v.as_array())
        .expect("tests array");
    let pass_case = cases
        .iter()
        .find(|case| {
            case.get("id").and_then(|v| v.as_str())
                == Some("tests/unit/unit_assertions.clear::test_pass")
        })
        .expect("pass case");
    assert_eq!(
        pass_case.get("status").and_then(|v| v.as_str()),
        Some("passed")
    );

    let fail_case = cases
        .iter()
        .find(|case| {
            case.get("id").and_then(|v| v.as_str())
                == Some("tests/unit/unit_assertions.clear::test_fail")
        })
        .expect("fail case");
    assert_eq!(
        fail_case.get("status").and_then(|v| v.as_str()),
        Some("failed")
    );
    assert_eq!(
        fail_case.get("failure_kind").and_then(|v| v.as_str()),
        Some("assertion_false")
    );
    assert_eq!(
        fail_case.get("failure_code").and_then(|v| v.as_str()),
        Some("C139")
    );
}

#[test]
fn test_command_junit_report_contains_failure_type_and_capture_fields() {
    let tmp = tempdir().expect("tempdir");
    let root = tmp.path().join("project");
    let unit = root.join("tests").join("unit");
    fs::create_dir_all(&unit).expect("create tests/unit");
    write_test_file(
        &unit.join("logic.clear"),
        "function test_fail() -> Bool { false }\n",
    );

    let output = Command::cargo_bin("clg")
        .expect("bin")
        .args(["test"])
        .arg(&root)
        .args(["--report", "junit"])
        .output()
        .expect("run clg test");
    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert!(
        stdout.contains("<failure type=\"C139\""),
        "junit failure should carry deterministic failure code"
    );
    assert!(
        stdout.contains("<system-out></system-out>"),
        "junit should expose captured stdout contract"
    );
    assert!(
        stdout.contains("<system-err></system-err>"),
        "junit should expose captured stderr contract"
    );
}

#[test]
fn test_command_json_events_stream_includes_test_case_stage() {
    let tmp = tempdir().expect("tempdir");
    let root = tmp.path().join("project");
    let unit = root.join("tests").join("unit");
    fs::create_dir_all(&unit).expect("create tests/unit");
    write_test_file(
        &unit.join("logic.clear"),
        "function test_ok() -> Bool { true }\n",
    );

    let output = Command::cargo_bin("clg")
        .expect("bin")
        .args(["--json-events", "test"])
        .arg(&root)
        .args(["--report", "json"])
        .output()
        .expect("run clg test");
    assert_eq!(output.status.code(), Some(0));

    let stderr = String::from_utf8(output.stderr).expect("stderr utf8");
    let events = parse_json_lines(&stderr);
    assert!(!events.is_empty(), "expected json event stream");
    assert!(events.iter().all(|event| {
        event.get("schema_version").and_then(|v| v.as_u64()) == Some(1)
            && event.get("command").and_then(|v| v.as_str()) == Some("test")
    }));
    assert!(events.iter().any(|event| {
        event.get("stage").and_then(|v| v.as_str()) == Some("test_case")
            && event.get("event").and_then(|v| v.as_str()) == Some("finish")
    }));
}

#[test]
fn test_command_timeout_override_is_reflected_in_report() {
    let tmp = tempdir().expect("tempdir");
    let root = tmp.path().join("project");
    let tests_root = root.join("tests");
    let unit = tests_root.join("unit");
    fs::create_dir_all(&unit).expect("create tests/unit");
    write_test_file(
        &unit.join("timeout_case.clear"),
        "function test_timeout_override() -> Bool { true }\n",
    );
    fs::write(
        tests_root.join("test-plan.json"),
        r#"{
  "schema_version": 1,
  "default_mock_sets": [],
  "cases": [
    {
      "test_id": "tests/unit/timeout_case.clear::test_timeout_override",
      "mock_sets": [],
      "timeout_ms": 7
    }
  ]
}"#,
    )
    .expect("write test-plan");

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
    let timeout_ms = payload["tests"]
        .as_array()
        .expect("tests array")
        .first()
        .and_then(|case| case.get("timeout_ms"))
        .and_then(|value| value.as_u64());
    assert_eq!(timeout_ms, Some(7));
}

#[test]
fn test_command_timeout_failure_maps_to_c137() {
    let tmp = tempdir().expect("tempdir");
    let root = tmp.path().join("project");
    let tests_root = root.join("tests");
    let unit = tests_root.join("unit");
    fs::create_dir_all(&unit).expect("create tests/unit");
    write_test_file(
        &unit.join("timeout_failure.clear"),
        "function test_timeout() -> Bool { true }\n",
    );
    fs::write(
        tests_root.join("test-plan.json"),
        r#"{
  "schema_version": 1,
  "default_mock_sets": [],
  "cases": [
    {
      "test_id": "tests/unit/timeout_failure.clear::test_timeout",
      "mock_sets": [],
      "timeout_ms": 1
    }
  ]
}"#,
    )
    .expect("write test-plan");

    let output = Command::cargo_bin("clg")
        .expect("bin")
        .args(["test"])
        .arg(&root)
        .args(["--report", "json"])
        .output()
        .expect("run clg test");
    assert_eq!(output.status.code(), Some(1));

    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    let payload: Value = serde_json::from_str(stdout.trim()).expect("json summary");
    let failed_case = payload["tests"]
        .as_array()
        .expect("tests array")
        .iter()
        .find(|case| {
            case.get("id").and_then(|v| v.as_str())
                == Some("tests/unit/timeout_failure.clear::test_timeout")
        })
        .expect("timeout case");
    assert_eq!(
        failed_case.get("failure_kind").and_then(|v| v.as_str()),
        Some("timeout")
    );
    assert_eq!(
        failed_case.get("failure_code").and_then(|v| v.as_str()),
        Some("C137")
    );
    assert!(
        failed_case
            .get("reason")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .contains("timeout after 1ms"),
        "expected deterministic timeout reason"
    );
}

#[test]
fn test_command_runtime_failure_maps_to_c138() {
    let tmp = tempdir().expect("tempdir");
    let root = tmp.path().join("project");
    let unit = root.join("tests").join("unit");
    fs::create_dir_all(&unit).expect("create tests/unit");
    write_test_file(
        &unit.join("runtime_trap.clear"),
        "function test_runtime_trap() -> Bool {\n    (1 / 0) == 1\n}\n",
    );

    let output = Command::cargo_bin("clg")
        .expect("bin")
        .args(["test"])
        .arg(&root)
        .args(["--report", "json"])
        .output()
        .expect("run clg test");
    assert_eq!(output.status.code(), Some(1));

    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    let payload: Value = serde_json::from_str(stdout.trim()).expect("json summary");
    let failed_case = payload["tests"]
        .as_array()
        .expect("tests array")
        .iter()
        .find(|case| {
            case.get("id").and_then(|v| v.as_str())
                == Some("tests/unit/runtime_trap.clear::test_runtime_trap")
        })
        .expect("runtime trap case");
    assert_eq!(
        failed_case.get("failure_kind").and_then(|v| v.as_str()),
        Some("runtime")
    );
    assert_eq!(
        failed_case.get("failure_code").and_then(|v| v.as_str()),
        Some("C138")
    );
    assert!(
        failed_case
            .get("reason")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .contains("runtime_trap:"),
        "expected deterministic runtime-trap reason prefix"
    );
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

#[test]
fn test_command_mock_path_traversal_like_binding_fails_closed_with_c136() {
    let tmp = tempdir().expect("tempdir");
    let root = tmp.path().join("project");
    let tests_root = root.join("tests");
    let unit = tests_root.join("unit");
    fs::create_dir_all(&unit).expect("create tests/unit");
    write_test_file(
        &unit.join("traversal.clear"),
        "function test_traversal() -> Bool { true }\n",
    );
    fs::write(
        tests_root.join("test-plan.json"),
        r#"{
  "schema_version": 1,
  "default_mock_sets": ["../outside"],
  "cases": [
    {
      "test_id": "tests/unit/traversal.clear::test_traversal",
      "mock_sets": []
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
    let errors = payload
        .get("errors")
        .and_then(|v| v.as_array())
        .expect("errors array");
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].get("code").and_then(|v| v.as_str()), Some("C136"));
    assert!(
        errors[0]
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .contains("unknown mock set `../outside`"),
        "expected traversal-like mock binding rejection"
    );
}

#[test]
fn test_command_mock_symlink_or_out_of_root_set_fails_closed_with_c136() {
    let tmp = tempdir().expect("tempdir");
    let root = tmp.path().join("project");
    let tests_root = root.join("tests");
    let unit = tests_root.join("unit");
    let mocks_root = tests_root.join("mocks");
    fs::create_dir_all(&unit).expect("create tests/unit");
    fs::create_dir_all(&mocks_root).expect("create tests/mocks");
    write_test_file(
        &unit.join("symlink.clear"),
        "function test_symlink_guard() -> Bool { true }\n",
    );

    let outside = root.join("outside-mocks");
    fs::create_dir_all(&outside).expect("create outside target");
    create_dir_link(&mocks_root.join("escape"), &outside);

    fs::write(
        tests_root.join("test-plan.json"),
        r#"{
  "schema_version": 1,
  "default_mock_sets": ["escape"],
  "cases": [
    {
      "test_id": "tests/unit/symlink.clear::test_symlink_guard",
      "mock_sets": []
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
    let errors = payload
        .get("errors")
        .and_then(|v| v.as_array())
        .expect("errors array");
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].get("code").and_then(|v| v.as_str()), Some("C136"));
    let message = errors[0]
        .get("message")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    assert!(
        message.contains("path safety violation")
            && (message.contains("symlink mock-set entries are not allowed")
                || message.contains("resolves outside")),
        "expected deterministic unsafe mock-path rejection, got: {message}"
    );
}

#[test]
fn test_command_known_mock_set_executes_with_deterministic_override() {
    let tmp = tempdir().expect("tempdir");
    let root = tmp.path().join("project");
    let tests_root = root.join("tests");
    let unit = tests_root.join("unit");
    fs::create_dir_all(root.join("services")).expect("create services");
    fs::create_dir_all(root.join("domain")).expect("create domain");
    fs::create_dir_all(tests_root.join("mocks").join("common").join("domain"))
        .expect("create mock set");
    fs::create_dir_all(&unit).expect("create tests/unit");
    write_test_file(
        &unit.join("discount_tests.clear"),
        "import services::discount\n\nfunction test_discount() -> Bool {\n    discount::compute_discount(200) == 50\n}\n",
    );
    write_test_file(
        &root.join("services").join("discount.clear"),
        "import domain::pricing\n\nexport pure function compute_discount(subtotal: Int) -> Int {\n    subtotal * pricing::base_discount_rate() / 100\n}\n",
    );
    write_test_file(
        &root.join("domain").join("pricing.clear"),
        "export pure function base_discount_rate() -> Int {\n    10\n}\n",
    );
    write_test_file(
        &tests_root
            .join("mocks")
            .join("common")
            .join("domain")
            .join("pricing.clear"),
        "export pure function base_discount_rate() -> Int {\n    25\n}\n",
    );
    fs::write(
        tests_root.join("test-plan.json"),
        r#"{
  "schema_version": 1,
  "default_mock_sets": ["common"],
  "cases": [
    {
      "test_id": "tests/unit/discount_tests.clear::test_discount",
      "mock_sets": []
    }
  ]
}"#,
    )
    .expect("write test-plan");

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
    assert_eq!(payload.get("status").and_then(|v| v.as_str()), Some("ok"));
    let tests = payload
        .get("tests")
        .and_then(|v| v.as_array())
        .expect("tests array");
    assert_eq!(tests.len(), 1);
    assert!(
        tests[0]
            .get("mock_sets")
            .and_then(|v| v.as_array())
            .map(|sets| sets.iter().any(|set| set.as_str() == Some("common")))
            .unwrap_or(false),
        "expected executed test to report selected mock set"
    );
}

#[test]
fn test_command_missing_explicit_case_binding_fails_closed_with_c136() {
    let tmp = tempdir().expect("tempdir");
    let root = tmp.path().join("project");
    let tests_root = root.join("tests");
    let unit = tests_root.join("unit");
    fs::create_dir_all(tests_root.join("mocks").join("common").join("domain"))
        .expect("create mock set");
    fs::create_dir_all(&unit).expect("create tests/unit");
    write_test_file(
        &unit.join("discount_tests.clear"),
        "function test_discount() -> Bool { true }\n",
    );
    write_test_file(
        &tests_root
            .join("mocks")
            .join("common")
            .join("domain")
            .join("pricing.clear"),
        "export pure function base_discount_rate() -> Int {\n    25\n}\n",
    );
    fs::write(
        tests_root.join("test-plan.json"),
        r#"{
  "schema_version": 1,
  "default_mock_sets": ["common"],
  "cases": []
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
    assert!(
        errors[0]
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .contains("missing explicit mock binding"),
        "expected explicit-binding fail-closed message"
    );
}
