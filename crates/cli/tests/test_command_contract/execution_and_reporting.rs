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
    assert_eq!(
        failed_case.get("failure_id").and_then(|v| v.as_str()),
        Some("assertion.bool_false")
    );
    let diff = failed_case
        .get("assertion_diff")
        .and_then(|v| v.as_object())
        .expect("assertion_diff object");
    assert_eq!(diff.get("schema_version").and_then(|v| v.as_u64()), Some(1));
    assert_eq!(diff.get("kind").and_then(|v| v.as_str()), Some("bool_return"));
    assert_eq!(diff.get("expected").and_then(|v| v.as_str()), Some("1"));
    assert_eq!(diff.get("actual").and_then(|v| v.as_str()), Some("0"));
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
        "import std::unit as unit\n\nfunction test_pass() -> Bool {\n    unit::assert_true(true, \"assert_true pass\")\n        && unit::assert_false(false, \"assert_false pass\")\n        && unit::assert_eq_int(2 + 3, 5, \"assert_eq_int pass\")\n        && unit::assert_eq_u64(U64(7), U64(7), \"assert_eq_u64 pass\")\n        && unit::assert_eq_bool(true, unit::assert_true(true, \"nested\"), \"assert_eq_bool pass\")\n}\n\nfunction test_fail() -> Bool {\n    unit::fail(\"forced failure\")\n}\n",
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
    assert_eq!(
        fail_case.get("failure_id").and_then(|v| v.as_str()),
        Some("assertion.bool_false")
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
