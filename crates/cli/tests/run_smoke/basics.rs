#[test]
fn emit_hello_and_run() {
    let out = tempfile::Builder::new()
        .prefix("emit_hello_")
        .suffix(".wasm")
        .tempfile()
        .unwrap();
    let out_path = out.path().to_path_buf();

    Command::cargo_bin("clg")
        .expect("bin")
        .args(["emit-hello", "-o"])
        .arg(&out_path)
        .assert()
        .success();

    Command::cargo_bin("clg")
        .expect("bin")
        .args(["run"])
        .arg(&out_path)
        .assert()
        .success()
        .stdout(predicates::str::contains("42\n"));
}

#[test]
fn build_and_run_samples() {
    // 01_hello
    let out1 = tempfile::tempdir().unwrap().path().join("hello.wasm");
    Command::cargo_bin("clg")
        .unwrap()
        .args(["build"])
        .arg(sample("01_hello.clear"))
        .args(["-o"])
        .arg(&out1)
        .assert()
        .success();
    Command::cargo_bin("clg")
        .unwrap()
        .args(["run"])
        .arg(&out1)
        .assert()
        .success()
        .stdout(predicates::str::contains("42\n"));

    // 12_zero_arg_fn
    let out2 = tempfile::tempdir().unwrap().path().join("zfn.wasm");
    Command::cargo_bin("clg")
        .unwrap()
        .args(["build"])
        .arg(sample("12_zero_arg_fn.clear"))
        .args(["-o"])
        .arg(&out2)
        .assert()
        .success();
    Command::cargo_bin("clg")
        .unwrap()
        .args(["run"])
        .arg(&out2)
        .assert()
        .success()
        .stdout(predicates::str::contains("7\n"));
}

#[test]
fn run_can_execute_clear_source_directly() {
    Command::cargo_bin("clg")
        .unwrap()
        .args(["run"])
        .arg(sample("02_arith.clear"))
        .assert()
        .success()
        .stdout(predicates::str::contains("15\n"));
}

#[test]
fn run_supports_comments_and_numeric_separators() {
    Command::cargo_bin("clg")
        .unwrap()
        .args(["run"])
        .arg(sample("19_comments_numeric_separator.clear"))
        .assert()
        .success()
        .stdout(predicates::str::contains("1000\n"));
}

#[test]
fn run_surfaces_build_type_errors_for_clear_source_json() {
    let output = Command::cargo_bin("clg")
        .unwrap()
        .args(["--json-errors", "run"])
        .arg(sample("14_arity_mismatch.clear"))
        .output()
        .expect("run command");
    assert!(!output.status.success(), "run should fail");
    let v: Value = serde_json::from_slice(&output.stdout).expect("json parse");
    assert_eq!(v.get("ok").and_then(|b| b.as_bool()), Some(false));
    let errors = v
        .get("errors")
        .and_then(|e| e.as_array())
        .expect("errors array");
    assert!(!errors.is_empty());
    let first = &errors[0];
    assert_eq!(first.get("code").and_then(|s| s.as_str()), Some("T002"));
    assert_eq!(first.get("stage").and_then(|s| s.as_str()), Some("type"));
}

#[test]
fn parse_command_prints_ast() {
    let sample = sample("01_hello.clear");
    Command::cargo_bin("clg")
        .unwrap()
        .args(["parse"])
        .arg(&sample)
        .assert()
        .success()
        .stdout(predicates::str::contains("add2").and(predicates::str::contains("main")));
}

#[test]
fn build_fails_on_parse_error() {
    let out = tempfile::tempdir().unwrap().path().join("bad.wasm");
    Command::cargo_bin("clg")
        .unwrap()
        .args(["build"])
        .arg(sample("06_trailing_call_comma.clear"))
        .args(["-o"])
        .arg(&out)
        .assert()
        .failure();
}

#[test]
fn build_fails_on_type_error() {
    let out = tempfile::tempdir().unwrap().path().join("type_err.wasm");
    Command::cargo_bin("clg")
        .unwrap()
        .args(["build"])
        .arg(sample("14_arity_mismatch.clear"))
        .args(["-o"])
        .arg(&out)
        .assert()
        .failure();
}

#[test]
fn parse_json_errors_are_structured() {
    let output = Command::cargo_bin("clg")
        .unwrap()
        .args(["--json-errors", "parse"])
        .arg(sample("06_trailing_call_comma.clear"))
        .output()
        .expect("run parse command");
    assert!(!output.status.success(), "parse should fail");
    let v: Value = serde_json::from_slice(&output.stdout).expect("json parse");
    assert_eq!(v.get("ok").and_then(|b| b.as_bool()), Some(false));
    let errors = v
        .get("errors")
        .and_then(|e| e.as_array())
        .expect("errors array");
    assert!(!errors.is_empty());
    let first = &errors[0];
    assert_eq!(first.get("code").and_then(|s| s.as_str()), Some("P001"));
    assert_eq!(first.get("stage").and_then(|s| s.as_str()), Some("parse"));
}

#[test]
fn build_json_errors_surface_type_failures() {
    let tmp = tempfile::tempdir().unwrap();
    let out_path = tmp.path().join("type_err.wasm");
    let output = Command::cargo_bin("clg")
        .unwrap()
        .args(["--json-errors", "build"])
        .arg(sample("14_arity_mismatch.clear"))
        .args(["-o"])
        .arg(&out_path)
        .output()
        .expect("run build command");
    assert!(!output.status.success(), "build should fail");
    let v: Value = serde_json::from_slice(&output.stdout).expect("json parse");
    assert_eq!(v.get("ok").and_then(|b| b.as_bool()), Some(false));
    let errors = v
        .get("errors")
        .and_then(|e| e.as_array())
        .expect("errors array");
    assert!(!errors.is_empty());
    let first = &errors[0];
    assert_eq!(first.get("code").and_then(|s| s.as_str()), Some("T002"));
    assert_eq!(first.get("stage").and_then(|s| s.as_str()), Some("type"));
}

#[test]
fn parse_json_errors_suggest_underscore_for_comma_grouped_number() {
    let output = Command::cargo_bin("clg")
        .unwrap()
        .args(["--json-errors", "parse"])
        .arg(sample("20_comma_numeric_separator_invalid.clear"))
        .output()
        .expect("run parse command");
    assert!(!output.status.success(), "parse should fail");
    let v: Value = serde_json::from_slice(&output.stdout).expect("json parse");
    assert_eq!(v.get("ok").and_then(|b| b.as_bool()), Some(false));
    let errors = v
        .get("errors")
        .and_then(|e| e.as_array())
        .expect("errors array");
    assert!(
        errors.iter().any(|e| {
            e.get("message")
                .and_then(|m| m.as_str())
                .map(|m| m.contains("use `_`"))
                .unwrap_or(false)
        }),
        "expected underscore separator suggestion in parse diagnostics"
    );
}

#[test]
fn run_generic_project_fixture() {
    Command::cargo_bin("clg")
        .unwrap()
        .args(["run"])
        .arg(example_project("generic/main.clear"))
        .assert()
        .success()
        .stdout(predicates::str::contains("generic: checkout pipeline"))
        .stdout(predicates::str::contains("900"));
}

#[test]
fn run_crypto_project_fixture() {
    Command::cargo_bin("clg")
        .unwrap()
        .args(["run"])
        .arg(example_project("crypto/main.clear"))
        .assert()
        .success()
        .stdout(predicates::str::contains("crypto: auth pipeline"))
        .stdout(predicates::str::contains("1"));
}

