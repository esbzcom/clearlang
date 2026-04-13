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

