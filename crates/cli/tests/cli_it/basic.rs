use super::*;

#[test]
fn parse_command_succeeds_on_hello() {
    let input = repo_sample("01_hello.clear");
    let mut cmd = Command::cargo_bin("clg").expect("bin");
    cmd.args(["parse"]).arg(&input);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Program"));
}

#[test]
fn run_subcommand_executes_main() {
    // Minimal source that returns 42
    let src = r#"
        pure function add2(a: Int, b: Int) -> Int { a + b }
        function main() -> Int { add2(40, 2) }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("hello.clear");
    fs::write(&file, src).expect("write");
    let out = tmp.path().join("out.wasm");

    // Build the wasm
    let mut build = Command::cargo_bin("clg").expect("bin");
    build.args(["build"]).arg(&file).args(["-o"]).arg(&out);
    build.assert().success();

    // Run it via CLI
    let mut run = Command::cargo_bin("clg").expect("bin");
    run.args(["run"]).arg(&out);
    run.assert()
        .success()
        .stdout(predicate::str::is_match(r"(?m)^\s*42\s*$").unwrap());
}

#[test]
fn emit_hello_writes_valid_wasm_and_runs() {
    let tmp = tempdir().unwrap();
    let out = tmp.path().join("hello.wasm");
    let mut cmd = Command::cargo_bin("clg").unwrap();
    cmd.args(["emit-hello", "-o"]).arg(&out);
    cmd.assert().success();

    let bytes = fs::read(&out).expect("read wasm");
    // Validate via wasmparser
    wasmparser::Validator::new()
        .validate_all(&bytes)
        .expect("valid wasm");

    let res = wasmtime_run(&bytes);
    assert_eq!(res, 42);
}

#[test]
fn build_and_run_samples() {
    let cases = [
        ("01_hello.clear", 42),
        ("02_arith.clear", 15),
        ("03_nested_calls.clear", 7),
        ("04_multiline_call.clear", 30),
        ("05_trailing_param_comma.clear", 3),
        ("08_main_const.clear", 42),
        ("18_return_simple.clear", 42),
    ];
    for (file, expect) in cases {
        let tmp = tempdir().unwrap();
        let out = tmp.path().join("out.wasm");
        let mut cmd = Command::cargo_bin("clg").unwrap();
        cmd.args(["build"]) // parse -> type-check -> const-eval emit
            .arg(repo_sample(file))
            .args(["-o"])
            .arg(&out);
        cmd.assert().success();
        let bytes = fs::read(&out).expect("read wasm");
        wasmparser::Validator::new()
            .validate_all(&bytes)
            .expect("valid wasm");
        let res = wasmtime_run(&bytes);
        assert_eq!(res, expect, "output mismatch for {}", file);
    }
}

#[test]
fn parse_failure_exits_nonzero() {
    let mut cmd = Command::cargo_bin("clg").unwrap();
    cmd.args(["--json-errors", "parse"])
        .arg(repo_sample("06_trailing_call_comma.clear"));
    let output = cmd.assert().failure().get_output().stdout.clone();
    let v: Value = serde_json::from_slice(&output).expect("json");
    assert_eq!(v.get("ok").and_then(|b| b.as_bool()), Some(false));
    let errs = v
        .get("errors")
        .and_then(|e| e.as_array())
        .expect("errors array");
    assert!(!errs.is_empty());
    let first = &errs[0];
    assert_eq!(first.get("code").and_then(|s| s.as_str()), Some("P001"));
    assert_eq!(first.get("stage").and_then(|s| s.as_str()), Some("parse"));
    assert!(first
        .get("message")
        .and_then(|s| s.as_str())
        .unwrap_or("")
        .contains("error"));
    // start/end should be present (>= 0)
    assert!(first.get("start").and_then(|n| n.as_u64()).is_some());
    assert!(first.get("end").and_then(|n| n.as_u64()).is_some());
}

#[test]
fn parse_rejects_export_import_with_explicit_code() {
    let src = r#"
        export import foo::bar::{Baz};
        function main() -> Int { 0 }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("reexport.clear");
    fs::write(&file, src).expect("write");

    let mut cmd = Command::cargo_bin("clg").unwrap();
    cmd.args(["--json-errors", "parse"]).arg(&file);
    let output = cmd.assert().failure().get_output().stdout.clone();
    let v: Value = serde_json::from_slice(&output).expect("json");
    assert_eq!(v.get("ok").and_then(|b| b.as_bool()), Some(false));
    let errs = v
        .get("errors")
        .and_then(|e| e.as_array())
        .expect("errors array");
    assert!(!errs.is_empty(), "expected parse errors");
    assert!(
        errs.iter()
            .any(|e| e.get("code").and_then(|s| s.as_str()) == Some("P011")),
        "expected P011 in {:?}",
        errs
    );
    assert!(
        errs.iter().any(|e| {
            e.get("message")
                .and_then(|s| s.as_str())
                .unwrap_or("")
                .contains("`export import` is not supported in v1")
        }),
        "expected explicit export import message"
    );
}

#[test]
fn parse_rejects_capture_list_with_explicit_code() {
    let src = r#"
        function bad() -> function(Int) -> Int {
            [x](y: Int) => y + x
        }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("capture_list.clear");
    fs::write(&file, src).expect("write");

    let mut cmd = Command::cargo_bin("clg").unwrap();
    cmd.args(["--json-errors", "parse"]).arg(&file);
    let output = cmd.assert().failure().get_output().stdout.clone();
    let v: Value = serde_json::from_slice(&output).expect("json");
    assert_eq!(v.get("ok").and_then(|b| b.as_bool()), Some(false));
    let errs = v
        .get("errors")
        .and_then(|e| e.as_array())
        .expect("errors array");
    assert!(!errs.is_empty(), "expected parse errors");
    assert!(
        errs.iter()
            .any(|e| e.get("code").and_then(|s| s.as_str()) == Some("P012")),
        "expected P012 in {:?}",
        errs
    );
    assert!(
        errs.iter().any(|e| {
            e.get("message")
                .and_then(|s| s.as_str())
                .unwrap_or("")
                .contains("capture-list syntax is not supported in v1")
        }),
        "expected explicit capture-list message"
    );
}

#[test]
fn build_failure_for_non_int_or_missing_main() {
    let tmp = tempdir().unwrap();
    let out = tmp.path().join("bad.wasm");
    let mut cmd = Command::cargo_bin("clg").unwrap();
    // 07_bools has no main; codegen should fail
    cmd.args(["--json-errors", "build"])
        .arg(repo_sample("07_bools.clear"))
        .args(["-o"])
        .arg(&out);
    let output = cmd.assert().failure().get_output().stdout.clone();
    let v: Value = serde_json::from_slice(&output).expect("json");
    assert_eq!(v.get("ok").and_then(|b| b.as_bool()), Some(false));
    let errs = v
        .get("errors")
        .and_then(|e| e.as_array())
        .expect("errors array");
    assert_eq!(errs.len(), 1);
    let e0 = &errs[0];
    // Missing main should map to C002
    assert_eq!(e0.get("code").and_then(|s| s.as_str()), Some("C002"));
    assert_eq!(e0.get("stage").and_then(|s| s.as_str()), Some("build"));
}

#[test]
fn build_contract_exports_apply_and_query() {
    let src = r#"
        pure function apply(state: Bytes, msg: Bytes) -> Bytes { msg }
        pure function query(state: Bytes, msg: Bytes) -> Bytes { state }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("contract.clear");
    fs::write(&file, src).expect("write");
    let out = tmp.path().join("contract.wasm");

    Command::cargo_bin("clg")
        .unwrap()
        .args(["build", "--contract"])
        .arg(&file)
        .args(["-o"])
        .arg(&out)
        .assert()
        .success();

    let bytes = fs::read(&out).expect("read wasm");
    let mut exports = Vec::new();
    for payload in wasmparser::Parser::new(0).parse_all(&bytes) {
        if let wasmparser::Payload::ExportSection(reader) = payload.expect("payload") {
            for export in reader {
                let export = export.expect("export");
                exports.push(export.name.to_string());
            }
        }
    }
    assert!(exports.contains(&"init".to_string()));
    assert!(exports.contains(&"handle".to_string()));
    assert!(exports.contains(&"query".to_string()));
}

#[test]
fn build_contract_requires_query_function() {
    let src = r#"
        pure function apply(state: Bytes, msg: Bytes) -> Bytes { msg }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("contract_missing_query.clear");
    fs::write(&file, src).expect("write");
    let out = tmp.path().join("contract_missing_query.wasm");

    let output = Command::cargo_bin("clg")
        .unwrap()
        .args(["--json-errors", "build", "--contract"])
        .arg(&file)
        .args(["-o"])
        .arg(&out)
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
    assert_eq!(e0.get("code").and_then(|s| s.as_str()), Some("C011"));
    assert_eq!(e0.get("stage").and_then(|s| s.as_str()), Some("build"));
}
