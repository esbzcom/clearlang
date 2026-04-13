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

