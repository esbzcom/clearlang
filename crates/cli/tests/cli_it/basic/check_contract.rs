#[test]
fn check_succeeds_with_strict_inputs() {
    let tmp = tempdir().expect("tempdir");
    let root = tmp.path().join("project");
    fs::create_dir_all(&root).expect("create root");
    let file = root.join("main.clear");
    fs::write(&file, "function main() -> Int { 0 }").expect("write source");

    Command::cargo_bin("clg")
        .unwrap()
        .args(["strict", "init"])
        .arg(&root)
        .assert()
        .success();

    let output = Command::cargo_bin("clg")
        .unwrap()
        .args(["--non-interactive", "check"])
        .arg(&file)
        .args(["--root"])
        .arg(&root)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(output).expect("utf8 stdout");
    assert!(text.contains("check ok:"));
}

#[test]
fn check_reports_missing_lockfile_with_check_stage() {
    let tmp = tempdir().expect("tempdir");
    let root = tmp.path().join("project");
    fs::create_dir_all(&root).expect("create root");
    let file = root.join("main.clear");
    fs::write(&file, "function main() -> Int { 0 }").expect("write source");

    Command::cargo_bin("clg")
        .unwrap()
        .args(["strict", "init"])
        .arg(&root)
        .assert()
        .success();
    fs::remove_file(root.join("clg.lock.json")).expect("remove lockfile");

    let output = Command::cargo_bin("clg")
        .unwrap()
        .args(["--json-errors", "check"])
        .arg(&file)
        .args(["--root"])
        .arg(&root)
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let v: Value = serde_json::from_slice(&output).expect("json");
    let errs = v
        .get("errors")
        .and_then(|e| e.as_array())
        .expect("errors array");
    assert_eq!(errs.len(), 1);
    let e0 = &errs[0];
    assert_eq!(e0.get("code").and_then(|s| s.as_str()), Some("C101"));
    assert_eq!(e0.get("stage").and_then(|s| s.as_str()), Some("check"));
}

#[test]
fn check_json_events_emit_structured_progress_for_ide_contract() {
    let tmp = tempdir().expect("tempdir");
    let root = tmp.path().join("project");
    fs::create_dir_all(&root).expect("create root");
    let file = root.join("main.clear");
    fs::write(&file, "function main() -> Int { 0 }").expect("write source");

    Command::cargo_bin("clg")
        .unwrap()
        .args(["strict", "init"])
        .arg(&root)
        .assert()
        .success();

    let stderr = Command::cargo_bin("clg")
        .unwrap()
        .args(["--json-events", "check"])
        .arg(&file)
        .args(["--root"])
        .arg(&root)
        .assert()
        .success()
        .get_output()
        .stderr
        .clone();
    let text = String::from_utf8(stderr).expect("utf8 stderr");
    let events: Vec<Value> = text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str::<Value>(line).expect("json event line"))
        .collect();
    assert!(!events.is_empty(), "expected progress events");
    assert!(events.iter().any(|event| {
        event.get("schema_version").and_then(|v| v.as_u64()) == Some(1)
            && event.get("command").and_then(|v| v.as_str()) == Some("check")
            && event.get("event").and_then(|v| v.as_str()) == Some("start")
            && event.get("stage").and_then(|v| v.as_str()) == Some("check_preflight")
    }));
}

#[test]
fn exit_code_mapping_uses_2_for_usage_errors_and_1_for_diagnostics() {
    let usage_status = Command::cargo_bin("clg")
        .unwrap()
        .args(["check"])
        .output()
        .expect("run usage failure")
        .status;
    assert_eq!(usage_status.code(), Some(2));

    let tmp = tempdir().expect("tempdir");
    let root = tmp.path().join("project");
    fs::create_dir_all(&root).expect("create root");
    let file = root.join("main.clear");
    fs::write(&file, "function main() -> Int { 0 }").expect("write source");

    let diagnostic_status = Command::cargo_bin("clg")
        .unwrap()
        .args(["--json-errors", "check"])
        .arg(&file)
        .args(["--root"])
        .arg(&root)
        .output()
        .expect("run check failure")
        .status;
    assert_eq!(diagnostic_status.code(), Some(1));
}

#[test]
fn check_json_errors_failures_write_json_to_stdout_and_keep_stderr_clean() {
    let tmp = tempdir().expect("tempdir");
    let root = tmp.path().join("project");
    fs::create_dir_all(&root).expect("create root");
    let file = root.join("main.clear");
    fs::write(&file, "function main() -> Int { 0 }").expect("write source");

    let output = Command::cargo_bin("clg")
        .unwrap()
        .args(["--json-errors", "check"])
        .arg(&file)
        .args(["--root"])
        .arg(&root)
        .output()
        .expect("run check");
    assert!(!output.status.success(), "check should fail");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
    let parsed: Value = serde_json::from_str(stdout.trim()).expect("json stdout");
    assert_eq!(parsed.get("ok").and_then(|v| v.as_bool()), Some(false));
    assert!(
        stderr.trim().is_empty(),
        "stderr must stay empty when --json-errors is set, got: {stderr}"
    );
}

#[test]
fn check_non_json_failures_write_human_error_to_stderr() {
    let tmp = tempdir().expect("tempdir");
    let root = tmp.path().join("project");
    fs::create_dir_all(&root).expect("create root");
    let file = root.join("main.clear");
    fs::write(&file, "function main() -> Int { 0 }").expect("write source");

    let output = Command::cargo_bin("clg")
        .unwrap()
        .args(["check"])
        .arg(&file)
        .args(["--root"])
        .arg(&root)
        .output()
        .expect("run check");
    assert!(!output.status.success(), "check should fail");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
    assert!(
        stdout.trim().is_empty(),
        "stdout should be empty on non-json failure"
    );
    assert!(
        stderr.contains("strict mode requires"),
        "stderr should contain human failure text, got: {stderr}"
    );
}
