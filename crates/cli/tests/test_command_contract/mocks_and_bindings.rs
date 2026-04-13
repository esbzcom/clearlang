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
