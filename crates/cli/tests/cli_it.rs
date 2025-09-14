use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::tempdir;
use serde_json::Value;

fn repo_sample(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../lumi-tests").join(name)
}

fn wasmtime_run(bytes: &[u8]) -> i32 {
    let engine = wasmtime::Engine::default();
    let module = wasmtime::Module::from_binary(&engine, bytes).expect("module");
    let mut store = wasmtime::Store::new(&engine, ());
    let instance = wasmtime::Instance::new(&mut store, &module, &[]).expect("instantiate");
    let main = instance
        .get_typed_func::<(), i32>(&mut store, "main")
        .expect("main export");
    main.call(&mut store, ()).expect("invoke main")
}

#[test]
fn parse_command_succeeds_on_hello() {
    let input = repo_sample("01_hello.lumi");
    let mut cmd = Command::cargo_bin("lumi").expect("bin");
    cmd.args(["parse"]).arg(&input);
    cmd.assert().success().stdout(predicate::str::contains("Program"));
}

#[test]
fn emit_hello_writes_valid_wasm_and_runs() {
    let tmp = tempdir().unwrap();
    let out = tmp.path().join("hello.wasm");
    let mut cmd = Command::cargo_bin("lumi").unwrap();
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
        ("01_hello.lumi", 42),
        ("02_arith.lumi", 15),
        ("03_nested_calls.lumi", 7),
        ("04_multiline_call.lumi", 30),
        ("05_trailing_param_comma.lumi", 3),
        ("08_main_const.lumi", 42),
        ("18_return_simple.lumi", 42),
    ];
    for (file, expect) in cases {
        let tmp = tempdir().unwrap();
        let out = tmp.path().join("out.wasm");
        let mut cmd = Command::cargo_bin("lumi").unwrap();
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
    let mut cmd = Command::cargo_bin("lumi").unwrap();
    cmd.args(["--json-errors", "parse"]).arg(repo_sample("06_trailing_call_comma.lumi"));
    let output = cmd.assert().failure().get_output().stdout.clone();
    let v: Value = serde_json::from_slice(&output).expect("json");
    assert_eq!(v.get("ok").and_then(|b| b.as_bool()), Some(false));
    let errs = v.get("errors").and_then(|e| e.as_array()).expect("errors array");
    assert!(!errs.is_empty());
    let first = &errs[0];
    assert_eq!(first.get("code").and_then(|s| s.as_str()), Some("P001"));
    assert_eq!(first.get("stage").and_then(|s| s.as_str()), Some("parse"));
    assert!(first.get("message").and_then(|s| s.as_str()).unwrap_or("").contains("error"));
    // start/end should be present (>= 0)
    assert!(first.get("start").and_then(|n| n.as_u64()).is_some());
    assert!(first.get("end").and_then(|n| n.as_u64()).is_some());
}

#[test]
fn build_failure_for_non_int_or_missing_main() {
    let tmp = tempdir().unwrap();
    let out = tmp.path().join("bad.wasm");
    let mut cmd = Command::cargo_bin("lumi").unwrap();
    // 07_bools has no main; codegen should fail
    cmd.args(["--json-errors", "build"]).arg(repo_sample("07_bools.lumi")).args(["-o"]).arg(&out);
    let output = cmd.assert().failure().get_output().stdout.clone();
    let v: Value = serde_json::from_slice(&output).expect("json");
    assert_eq!(v.get("ok").and_then(|b| b.as_bool()), Some(false));
    let errs = v.get("errors").and_then(|e| e.as_array()).expect("errors array");
    assert_eq!(errs.len(), 1);
    let e0 = &errs[0];
    // Missing main should map to C002
    assert_eq!(e0.get("code").and_then(|s| s.as_str()), Some("C002"));
    assert_eq!(e0.get("stage").and_then(|s| s.as_str()), Some("build"));
}

#[test]
fn type_error_reports_json_with_span_and_code() {
    // Create a small source with a type mismatch: add(1, true)
    let src = r#"
        pure function add(x: Int, y: Int) -> Int { x + y }
        function main() -> Int { add(1, true) }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("bad.lumi");
    fs::write(&file, src).expect("write");
    let out = tmp.path().join("out.wasm");
    let mut cmd = Command::cargo_bin("lumi").unwrap();
    cmd.args(["--json-errors", "build"]).arg(&file).args(["-o"]).arg(&out);
    let output = cmd.assert().failure().get_output().stdout.clone();
    let v: Value = serde_json::from_slice(&output).expect("json");
    assert_eq!(v.get("ok").and_then(|b| b.as_bool()), Some(false));
    let errs = v.get("errors").and_then(|e| e.as_array()).expect("errors array");
    assert_eq!(errs.len(), 1);
    let e0 = &errs[0];
    // Arg type mismatch should map to T003
    assert_eq!(e0.get("code").and_then(|s| s.as_str()), Some("T003"));
    assert_eq!(e0.get("stage").and_then(|s| s.as_str()), Some("type"));
    // Span presence
    assert!(e0.get("start").and_then(|n| n.as_u64()).is_some());
    assert!(e0.get("end").and_then(|n| n.as_u64()).is_some());
}

#[test]
fn collections_error_reports_collection_kind_error_in_json() {
    // Calling a collection API with wrong kind should yield T207 via --json-errors
    let src = r#"
        function main() -> Int { std::map::len(0) }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("bad_collections.lumi");
    std::fs::write(&file, src).expect("write");
    let out = tmp.path().join("out.wasm");
    let mut cmd = Command::cargo_bin("lumi").unwrap();
    cmd.args(["--json-errors", "build"]).arg(&file).args(["-o"]).arg(&out);
    let output = cmd.assert().failure().get_output().stdout.clone();
    let v: Value = serde_json::from_slice(&output).expect("json");
    assert_eq!(v.get("ok").and_then(|b| b.as_bool()), Some(false));
    let errs = v.get("errors").and_then(|e| e.as_array()).expect("errors array");
    assert!(!errs.is_empty());
    let e0 = &errs[0];
    assert_eq!(e0.get("code").and_then(|s| s.as_str()), Some("T207"));
    assert_eq!(e0.get("stage").and_then(|s| s.as_str()), Some("type"));
}

#[test]
fn if_branch_mismatch_reports_t301_in_json() {
    let src = r#"
        function main() -> Int { if true { 1 } else { false } }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("bad_if.lumi");
    std::fs::write(&file, src).expect("write");
    let out = tmp.path().join("out.wasm");
    let mut cmd = Command::cargo_bin("lumi").unwrap();
    cmd.args(["--json-errors", "build"]).arg(&file).args(["-o"]).arg(&out);
    let output = cmd.assert().failure().get_output().stdout.clone();
    let v: Value = serde_json::from_slice(&output).expect("json");
    assert_eq!(v.get("ok").and_then(|b| b.as_bool()), Some(false));
    let errs = v.get("errors").and_then(|e| e.as_array()).expect("errors array");
    assert!(!errs.is_empty());
    let e0 = &errs[0];
    assert_eq!(e0.get("code").and_then(|s| s.as_str()), Some("T301"));
    assert_eq!(e0.get("stage").and_then(|s| s.as_str()), Some("type"));
}
