#[test]
fn fmt_formats_single_file_deterministically() {
    let tmp = tempdir().expect("tempdir");
    let file = tmp.path().join("main.clear");
    fs::write(&file, b"function main() -> Int { 0 }  \r\n\t\r\n").expect("write source");

    Command::cargo_bin("clg")
        .unwrap()
        .args(["fmt"])
        .arg(&file)
        .assert()
        .success();

    let formatted = fs::read_to_string(&file).expect("read formatted file");
    assert_eq!(formatted, "function main() -> Int { 0 }\n\n");
}

#[test]
fn fmt_check_fails_when_changes_are_required_and_keeps_file_bytes() {
    let tmp = tempdir().expect("tempdir");
    let file = tmp.path().join("main.clear");
    let original = b"function main() -> Int { 0 }  \r\n";
    fs::write(&file, original).expect("write source");

    let output = Command::cargo_bin("clg")
        .unwrap()
        .args(["--json-errors", "fmt", "--check"])
        .arg(&file)
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
    assert_eq!(e0.get("code").and_then(|s| s.as_str()), Some("C132"));
    assert_eq!(e0.get("stage").and_then(|s| s.as_str()), Some("fmt"));

    let after = fs::read(&file).expect("read source after --check");
    assert_eq!(after.as_slice(), original);
}

#[test]
fn fmt_directory_formats_clear_files_only() {
    let tmp = tempdir().expect("tempdir");
    let root = tmp.path().join("project");
    let nested = root.join("nested");
    fs::create_dir_all(&nested).expect("create dirs");
    let a = root.join("a.clear");
    let b = nested.join("b.clear");
    let note = root.join("note.txt");
    fs::write(&a, "function main() -> Int { 0 }  ").expect("write a");
    fs::write(&b, "function helper() -> Int { 1 }\r\n").expect("write b");
    fs::write(&note, "keep me").expect("write note");

    Command::cargo_bin("clg")
        .unwrap()
        .args(["fmt"])
        .arg(&root)
        .assert()
        .success();

    assert_eq!(
        fs::read_to_string(&a).expect("read a"),
        "function main() -> Int { 0 }\n"
    );
    assert_eq!(
        fs::read_to_string(&b).expect("read b"),
        "function helper() -> Int { 1 }\n"
    );
    assert_eq!(fs::read_to_string(&note).expect("read note"), "keep me");
}

#[test]
fn lint_reports_warnings_and_exits_zero_by_default() {
    let tmp = tempdir().expect("tempdir");
    let file = tmp.path().join("main.clear");
    fs::write(&file, b"function main() -> Int { 0 }  \r\n").expect("write source");

    let output = Command::cargo_bin("clg")
        .unwrap()
        .args(["lint"])
        .arg(&file)
        .output()
        .expect("run lint");
    assert_eq!(output.status.code(), Some(0));

    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    let stderr = String::from_utf8(output.stderr).expect("stderr utf8");
    assert!(stdout.contains("lint ok: 1 file(s),"));
    assert!(stderr.contains("lint warning [lint.trailing_whitespace]"));
    assert!(stderr.contains("lint warning [lint.crlf_line_endings]"));
}

#[test]
fn lint_deny_warnings_fails_with_json_error() {
    let tmp = tempdir().expect("tempdir");
    let file = tmp.path().join("main.clear");
    fs::write(&file, b"function main() -> Int { 0 }  \r\n").expect("write source");

    let output = Command::cargo_bin("clg")
        .unwrap()
        .args(["--json-errors", "lint", "--deny-warnings"])
        .arg(&file)
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
    assert_eq!(e0.get("code").and_then(|s| s.as_str()), Some("C133"));
    assert_eq!(e0.get("stage").and_then(|s| s.as_str()), Some("lint"));
}

#[test]
fn lint_deny_warnings_succeeds_when_file_is_clean() {
    let tmp = tempdir().expect("tempdir");
    let file = tmp.path().join("main.clear");
    fs::write(&file, "function main() -> Int { 0 }\n").expect("write source");

    Command::cargo_bin("clg")
        .unwrap()
        .args(["lint", "--deny-warnings"])
        .arg(&file)
        .assert()
        .success()
        .stdout(predicate::str::contains("lint ok: 1 file(s), 0 warning(s)"));
}
