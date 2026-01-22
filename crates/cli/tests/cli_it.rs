use assert_cmd::prelude::*;
use predicates::prelude::*;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::tempdir;

fn repo_sample(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../clearlang-tests")
        .join(name)
}

mod common;

fn wasmtime_run(bytes: &[u8]) -> i32 {
    let engine = common::engine();
    let module = wasmtime::Module::from_binary(engine, bytes).expect("module");
    let mut store = common::store(engine);
    let instance = wasmtime::Instance::new(&mut store, &module, &[]).expect("instantiate");
    let main = instance
        .get_typed_func::<(), i32>(&mut store, "main")
        .expect("main export");
    main.call(&mut store, ()).expect("invoke main")
}

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

#[test]
fn type_error_reports_json_with_span_and_code() {
    // Create a small source with a type mismatch: add(1, true)
    let src = r#"
        pure function add(x: Int, y: Int) -> Int { x + y }
        function main() -> Int { add(1, true) }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("bad.clear");
    fs::write(&file, src).expect("write");
    let out = tmp.path().join("out.wasm");
    let mut cmd = Command::cargo_bin("clg").unwrap();
    cmd.args(["--json-errors", "build"])
        .arg(&file)
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
    let file = tmp.path().join("bad_collections.clear");
    std::fs::write(&file, src).expect("write");
    let out = tmp.path().join("out.wasm");
    let mut cmd = Command::cargo_bin("clg").unwrap();
    cmd.args(["--json-errors", "build"])
        .arg(&file)
        .args(["-o"])
        .arg(&out);
    let output = cmd.assert().failure().get_output().stdout.clone();
    let v: Value = serde_json::from_slice(&output).expect("json");
    assert_eq!(v.get("ok").and_then(|b| b.as_bool()), Some(false));
    let errs = v
        .get("errors")
        .and_then(|e| e.as_array())
        .expect("errors array");
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
    let file = tmp.path().join("bad_if.clear");
    std::fs::write(&file, src).expect("write");
    let out = tmp.path().join("out.wasm");
    let mut cmd = Command::cargo_bin("clg").unwrap();
    cmd.args(["--json-errors", "build"])
        .arg(&file)
        .args(["-o"])
        .arg(&out);
    let output = cmd.assert().failure().get_output().stdout.clone();
    let v: Value = serde_json::from_slice(&output).expect("json");
    assert_eq!(v.get("ok").and_then(|b| b.as_bool()), Some(false));
    let errs = v
        .get("errors")
        .and_then(|e| e.as_array())
        .expect("errors array");
    assert!(!errs.is_empty());
    let e0 = &errs[0];
    assert_eq!(e0.get("code").and_then(|s| s.as_str()), Some("T301"));
    assert_eq!(e0.get("stage").and_then(|s| s.as_str()), Some("type"));
}

#[test]
fn build_emits_vcs_json() {
    let tmp = tempdir().unwrap();
    let src_path = tmp.path().join("contract.clear");
    let wasm_path = tmp.path().join("out.wasm");
    let vcs_path = tmp.path().join("out.vc.json");
    let src = r#"
        pure function inc(x: Int) -> Int
            require { 0 <= x }
            ensure { result > x }
        { x + 1 }
        function main() -> Int { inc(1) }
    "#;
    fs::write(&src_path, src).expect("write contract");

    let mut cmd = Command::cargo_bin("clg").unwrap();
    cmd.args(["build"])
        .arg(&src_path)
        .args(["-o"])
        .arg(&wasm_path)
        .arg("--emit-vcs")
        .arg(&vcs_path);
    cmd.assert().success();

    let data = fs::read_to_string(&vcs_path).expect("read vcs");
    let items: Value = serde_json::from_str(&data).expect("json array");
    let arr = items.as_array().expect("array");
    assert_eq!(arr.len(), 1);
    let first = &arr[0];
    assert_eq!(first.get("version").and_then(|n| n.as_u64()), Some(2));
    assert_eq!(first.get("function").and_then(|s| s.as_str()), Some("inc"));
    assert_eq!(
        first.get("status").and_then(|s| s.as_str()),
        Some("generated")
    );
    let pre = first
        .get("pre")
        .and_then(|o| o.as_object())
        .expect("pre obj");
    assert!(pre
        .get("ast")
        .and_then(|s| s.as_str())
        .unwrap_or("")
        .contains("<="));
    let vc = first.get("vc").and_then(|o| o.as_object()).expect("vc obj");
    let vc_text = vc.get("smt2").and_then(|s| s.as_str()).unwrap_or("");
    assert!(vc_text.contains("=>"));
    assert!(vc_text.contains("(+ x 1)"));
    let positions = first
        .get("positions")
        .and_then(|o| o.as_object())
        .expect("positions obj");
    assert_eq!(
        positions.get("file").and_then(|s| s.as_str()),
        Some(src_path.to_string_lossy().as_ref())
    );
    assert!(first.get("refinements").is_none());
}

#[test]
fn build_emits_refinement_premises_json() {
    let tmp = tempdir().unwrap();
    let src_path = tmp.path().join("refined.clear");
    let wasm_path = tmp.path().join("refined.wasm");
    let vcs_path = tmp.path().join("refined.vc.json");
    let src = r#"
        type Nat = Int where n >= 0;
        pure function inc(a: Nat) -> Nat { a + 1 }
        function main() -> Int { 0 }
    "#;
    fs::write(&src_path, src).expect("write refined source");

    Command::cargo_bin("clg")
        .unwrap()
        .args(["build"])
        .arg(&src_path)
        .args(["-o"])
        .arg(&wasm_path)
        .arg("--emit-vcs")
        .arg(&vcs_path)
        .assert()
        .success();

    let data = fs::read_to_string(&vcs_path).expect("read vcs");
    let items: Value = serde_json::from_str(&data).expect("json array");
    let arr = items.as_array().expect("array");
    assert_eq!(arr.len(), 1);
    let first = &arr[0];
    let refinements = first
        .get("refinements")
        .and_then(|o| o.as_object())
        .expect("refinements obj");
    let premises = refinements
        .get("premises")
        .and_then(|p| p.as_array())
        .expect("premises array");
    assert_eq!(premises.len(), 2);

    let param = &premises[0];
    assert_eq!(param.get("id").and_then(|s| s.as_str()), Some("ref:0"));
    assert_eq!(param.get("alias").and_then(|s| s.as_str()), Some("Nat"));
    assert_eq!(param.get("binder").and_then(|s| s.as_str()), Some("n"));
    let substitution = param
        .get("substitution")
        .and_then(|o| o.as_object())
        .expect("substitution obj");
    assert_eq!(substitution.get("ast").and_then(|s| s.as_str()), Some("a"));
    let predicate = param
        .get("predicate")
        .and_then(|o| o.as_object())
        .expect("predicate obj");
    assert_eq!(
        predicate.get("ast").and_then(|s| s.as_str()),
        Some("a >= 0")
    );
    let attachment = param
        .get("attachment")
        .and_then(|o| o.as_object())
        .expect("attachment obj");
    assert_eq!(
        attachment.get("kind").and_then(|s| s.as_str()),
        Some("param")
    );
    let detail = attachment
        .get("detail")
        .and_then(|o| o.as_object())
        .expect("detail obj");
    assert_eq!(detail.get("param").and_then(|s| s.as_str()), Some("a"));

    let ret = &premises[1];
    assert_eq!(ret.get("id").and_then(|s| s.as_str()), Some("ref:1"));
    let attachment = ret
        .get("attachment")
        .and_then(|o| o.as_object())
        .expect("attachment obj");
    assert_eq!(
        attachment.get("kind").and_then(|s| s.as_str()),
        Some("return")
    );
    let detail = attachment
        .get("detail")
        .and_then(|o| o.as_object())
        .expect("detail obj");
    assert_eq!(
        detail.get("result").and_then(|s| s.as_str()),
        Some("result")
    );
}
#[test]
fn build_emits_variant_vcs_json() {
    let tmp = tempdir().unwrap();
    let src_path = tmp.path().join("coalesce.clear");
    let wasm_path = tmp.path().join("coalesce.wasm");
    let vcs_path = tmp.path().join("coalesce.vc.json");
    let src = r#"
        pure function pick(opt: Option<Int>) -> Option<Int>
            ensure { result == result }
        { Some((opt ?? 7) + 1) }
        function main() -> Int { if let Some(v) = pick(Some(1)) { v } else { 0 } }
    "#;
    fs::write(&src_path, src).expect("write source");

    Command::cargo_bin("clg")
        .unwrap()
        .args(["build"])
        .arg(&src_path)
        .args(["-o"])
        .arg(&wasm_path)
        .arg("--emit-vcs")
        .arg(&vcs_path)
        .assert()
        .success();

    let data = fs::read_to_string(&vcs_path).expect("read vcs");
    let items: Value = serde_json::from_str(&data).expect("json array");
    let arr = items.as_array().expect("array");
    assert_eq!(arr.len(), 1);
    let vc = arr[0]
        .get("vc")
        .and_then(|o| o.as_object())
        .and_then(|o| o.get("smt2"))
        .and_then(|s| s.as_str())
        .expect("vc smt2");
    assert!(vc.contains("cl.variant.tag"));
    assert!(!vc.contains("unsupported"));
}

#[test]
fn runtime_contract_violation_reports_r000_json() {
    let tmp = tempdir().unwrap();
    let src_path = tmp.path().join("require_fail.clear");
    let wasm_path = tmp.path().join("require_fail.wasm");

    let src = r#"
        pure function dec(x: Int) -> Int
            require { x > 0 }
        { x - 1 }
        function main() -> Int { dec(0) }
    "#;
    fs::write(&src_path, src.trim()).expect("write source");

    Command::cargo_bin("clg")
        .unwrap()
        .args(["build"])
        .arg(&src_path)
        .args(["-o"])
        .arg(&wasm_path)
        .assert()
        .success();

    let output = Command::cargo_bin("clg")
        .unwrap()
        .args(["--json-errors", "run"])
        .arg(&wasm_path)
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let v: Value = serde_json::from_slice(&output).expect("json");
    assert!(!v.get("ok").and_then(|b| b.as_bool()).unwrap_or(true));
    let errs = v.get("errors").and_then(|e| e.as_array()).expect("errors");
    assert_eq!(errs.len(), 1);
    let e0 = &errs[0];
    assert_eq!(e0.get("code").and_then(|s| s.as_str()), Some("R000"));
    assert_eq!(e0.get("stage").and_then(|s| s.as_str()), Some("runtime"));
    assert!(e0
        .get("message")
        .and_then(|s| s.as_str())
        .unwrap_or("")
        .contains("contract `require` guard failed"));
    assert_eq!(e0.get("function").and_then(|s| s.as_str()), Some("require"));
}

#[test]
fn runtime_ensure_violation_reports_r000_json() {
    let tmp = tempdir().unwrap();
    let src_path = tmp.path().join("ensure_fail.clear");
    let wasm_path = tmp.path().join("ensure_fail.wasm");

    // Postcondition deliberately false
    let src = r#"
        pure function id1() -> Int
            ensure { result == 0 }
        { 1 }
        function main() -> Int { id1() }
    "#;
    fs::write(&src_path, src.trim()).expect("write source");

    Command::cargo_bin("clg")
        .unwrap()
        .args(["build"])
        .arg(&src_path)
        .args(["-o"])
        .arg(&wasm_path)
        .assert()
        .success();

    let output = Command::cargo_bin("clg")
        .unwrap()
        .args(["--json-errors", "run"])
        .arg(&wasm_path)
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let v: Value = serde_json::from_slice(&output).expect("json");
    assert!(!v.get("ok").and_then(|b| b.as_bool()).unwrap_or(true));
    let errs = v.get("errors").and_then(|e| e.as_array()).expect("errors");
    assert_eq!(errs.len(), 1);
    let e0 = &errs[0];
    assert_eq!(e0.get("code").and_then(|s| s.as_str()), Some("R000"));
    assert_eq!(e0.get("stage").and_then(|s| s.as_str()), Some("runtime"));
    assert!(e0
        .get("message")
        .and_then(|s| s.as_str())
        .unwrap_or("")
        .contains("contract `ensure` guard failed"));
    assert_eq!(e0.get("function").and_then(|s| s.as_str()), Some("ensure"));
}

#[test]
fn runtime_string_concat_oom_reports_r001_json() {
    let tmp = tempdir().unwrap();
    let src_path = tmp.path().join("oom.clear");
    let wasm_path = tmp.path().join("oom.wasm");

    let big_a = "a".repeat(40_000);
    let big_b = "b".repeat(40_000);
    let src = format!(
        "function main() -> Int {{ std::str::len(std::str::concat({a:?}, {b:?})) }}",
        a = big_a,
        b = big_b,
    );
    fs::write(&src_path, src).expect("write source");

    Command::cargo_bin("clg")
        .unwrap()
        .args(["build"])
        .arg(&src_path)
        .args(["-o"])
        .arg(&wasm_path)
        .assert()
        .success();

    let output = Command::cargo_bin("clg")
        .unwrap()
        .args(["--json-errors", "run"])
        .arg(&wasm_path)
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let v: Value = serde_json::from_slice(&output).expect("json");
    assert!(!v.get("ok").and_then(|b| b.as_bool()).unwrap_or(true));
    let errs = v.get("errors").and_then(|e| e.as_array()).expect("errors");
    assert_eq!(errs.len(), 1);
    let e0 = &errs[0];
    assert_eq!(e0.get("code").and_then(|s| s.as_str()), Some("R001"));
    assert_eq!(e0.get("stage").and_then(|s| s.as_str()), Some("runtime"));
    assert!(e0
        .get("message")
        .and_then(|s| s.as_str())
        .unwrap_or("")
        .contains("allocator ran out of memory"));
    assert!(e0.get("function").is_none());
}

#[test]
fn run_wasi_print_emits_output() {
    let tmp = tempdir().unwrap();
    let src_path = tmp.path().join("wasi_print.clear");
    let wasm_path = tmp.path().join("wasi_print.wasm");

    let src = r#"
        io function main() -> Int {
            std::wasi::print(std::bytes::from_string("hi"));
            0
        }
    "#;
    fs::write(&src_path, src.trim()).expect("write source");

    Command::cargo_bin("clg")
        .unwrap()
        .args(["build"])
        .arg(&src_path)
        .args(["-o"])
        .arg(&wasm_path)
        .assert()
        .success();

    Command::cargo_bin("clg")
        .unwrap()
        .args(["run"])
        .arg(&wasm_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("hi"));
}

#[test]
fn run_env_time_and_random_stubs() {
    let tmp = tempdir().unwrap();
    let src_path = tmp.path().join("env_time_random.clear");
    let wasm_path = tmp.path().join("env_time_random.wasm");

    let src = r#"
        io function main() -> Int {
            std::bytes::len(std::env::random(4)) + std::env::time()
        }
    "#;
    fs::write(&src_path, src.trim()).expect("write source");

    Command::cargo_bin("clg")
        .unwrap()
        .args(["build"])
        .arg(&src_path)
        .args(["-o"])
        .arg(&wasm_path)
        .assert()
        .success();

    Command::cargo_bin("clg")
        .unwrap()
        .args(["run"])
        .arg(&wasm_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("4"));
}

#[test]
fn runtime_fuel_exhaustion_reports_r004_json() {
    let tmp = tempdir().unwrap();
    let src_path = tmp.path().join("fuel_exhaust.clear");
    let wasm_path = tmp.path().join("fuel_exhaust.wasm");

    let src = r#"
        mut function main() -> Int {
            while true invariant { true } { }
            0
        }
    "#;
    fs::write(&src_path, src.trim()).expect("write source");

    Command::cargo_bin("clg")
        .unwrap()
        .args(["build"])
        .arg(&src_path)
        .args(["-o"])
        .arg(&wasm_path)
        .assert()
        .success();

    let output = Command::cargo_bin("clg")
        .unwrap()
        .args(["--json-errors", "run"])
        .arg(&wasm_path)
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let v: Value = serde_json::from_slice(&output).expect("json");
    let errs = v.get("errors").and_then(|e| e.as_array()).expect("errors");
    assert_eq!(errs.len(), 1);
    let e0 = &errs[0];
    assert_eq!(e0.get("code").and_then(|s| s.as_str()), Some("R004"));
    assert_eq!(e0.get("stage").and_then(|s| s.as_str()), Some("runtime"));
}

#[test]
fn runtime_recursion_limit_reports_r004_json() {
    let tmp = tempdir().unwrap();
    let src_path = tmp.path().join("recursion_limit.clear");
    let wasm_path = tmp.path().join("recursion_limit.wasm");

    let src = r#"
        mut function spin() -> Int { spin() }
        mut function main() -> Int { spin() }
    "#;
    fs::write(&src_path, src.trim()).expect("write source");

    Command::cargo_bin("clg")
        .unwrap()
        .env("CLG_DISABLE_TOTALITY", "1")
        .args(["build"])
        .arg(&src_path)
        .args(["-o"])
        .arg(&wasm_path)
        .assert()
        .success();

    let output = Command::cargo_bin("clg")
        .unwrap()
        .args(["--json-errors", "run"])
        .arg(&wasm_path)
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let v: Value = serde_json::from_slice(&output).expect("json");
    let errs = v.get("errors").and_then(|e| e.as_array()).expect("errors");
    assert_eq!(errs.len(), 1);
    let e0 = &errs[0];
    assert_eq!(e0.get("code").and_then(|s| s.as_str()), Some("R004"));
    assert_eq!(e0.get("stage").and_then(|s| s.as_str()), Some("runtime"));
}
#[test]
fn run_option_result_success_paths() {
    let tmp = tempdir().unwrap();
    let src_path = tmp.path().join("variants_success.clear");
    let wasm_path = tmp.path().join("variants_success.wasm");

    let src = r#"
        pure function inc(opt: Option<Int>) -> Option<Int> { Some(opt? + 1) }
        pure function twice(res: Result<Int, Int>) -> Result<Int, Int> { Ok(res? * 2) }
        pure function seed_opt() -> Option<Int> { Some(41) }
        pure function seed_res() -> Result<Int, Int> { Ok(21) }
        function main() -> Int {
            if let Some(v) = inc(seed_opt()) {
                if let Ok(w) = twice(seed_res()) { v + w } else { 0 }
            } else { 0 }
        }
    "#;
    fs::write(&src_path, src.trim()).expect("write source");

    Command::cargo_bin("clg")
        .unwrap()
        .args(["build"])
        .arg(&src_path)
        .args(["-o"])
        .arg(&wasm_path)
        .assert()
        .success();

    Command::cargo_bin("clg")
        .unwrap()
        .args(["run"])
        .arg(&wasm_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("84\n"));
}

#[test]
fn run_option_result_propagation_paths() {
    let tmp = tempdir().unwrap();
    let src_path = tmp.path().join("variants_propagation.clear");
    let wasm_path = tmp.path().join("variants_propagation.wasm");

    let src = r#"
        pure function inc(opt: Option<Int>) -> Option<Int> { Some(opt? + 1) }
        pure function guard(res: Result<Int, Int>) -> Result<Int, Int> { Ok(res? + 5) }
        pure function none() -> Option<Int> { None }
        pure function err_res() -> Result<Int, Int> { Err(9) }
        function main() -> Int {
            if let Some(v) = inc(none()) {
                v
            } else {
                if let Ok(w) = guard(err_res()) { w } else { 17 }
            }
        }
    "#;
    fs::write(&src_path, src.trim()).expect("write source");

    Command::cargo_bin("clg")
        .unwrap()
        .args(["build"])
        .arg(&src_path)
        .args(["-o"])
        .arg(&wasm_path)
        .assert()
        .success();

    Command::cargo_bin("clg")
        .unwrap()
        .args(["run"])
        .arg(&wasm_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("17\n"));
}

#[test]
fn run_reports_r003_invalid_variant_json() {
    use clg_codegen_wasm::emit_from_ir;
    use clg_ir::{
        Function as IrFunction, Instr as IrInstr, IrType, Module as IrModule, Value as IrValue,
        VariantKind,
    };

    let main_fn = IrFunction {
        name: "main".to_string(),
        params: vec![],
        ret: Some(IrType::Int),
        body: vec![
            IrInstr::IConst {
                dst: IrValue(0),
                ty: IrType::Int,
                n: 2,
            },
            IrInstr::IConst {
                dst: IrValue(1),
                ty: IrType::Int,
                n: 0,
            },
            IrInstr::IConst {
                dst: IrValue(2),
                ty: IrType::Int,
                n: 0,
            },
            IrInstr::VariantInit {
                dst: IrValue(3),
                tag: IrValue(0),
                payload_lo: IrValue(1),
                payload_hi: IrValue(2),
            },
            IrInstr::VariantLoadTag {
                dst: IrValue(4),
                variant: IrValue(3),
                kind: VariantKind::Option,
            },
            IrInstr::Ret { val: IrValue(4) },
        ],
    };
    let ir = IrModule {
        funcs: vec![main_fn],
    };
    let wasm = emit_from_ir(&ir).expect("codegen ok");

    let tmp = tempdir().unwrap();
    let wasm_path = tmp.path().join("invalid_tag.wasm");
    fs::write(&wasm_path, wasm).expect("write wasm");

    let output = Command::cargo_bin("clg")
        .unwrap()
        .args(["--json-errors", "run"])
        .arg(&wasm_path)
        .output()
        .expect("run command");
    assert!(
        !output.status.success(),
        "invalid variant tag should trap the runtime"
    );
    let v: Value = serde_json::from_slice(&output.stdout).expect("json parse");
    assert_eq!(v.get("ok").and_then(|b| b.as_bool()), Some(false));
    let errs = v
        .get("errors")
        .and_then(|e| e.as_array())
        .expect("errors array");
    assert!(!errs.is_empty());
    let first = &errs[0];
    assert_eq!(first.get("code").and_then(|s| s.as_str()), Some("R003"));
    assert_eq!(first.get("stage").and_then(|s| s.as_str()), Some("runtime"));
    let msg = first.get("message").and_then(|s| s.as_str()).unwrap_or("");
    assert!(
        msg.contains("Option"),
        "message should mention the Option kind"
    );
    assert!(
        msg.contains("tag 2"),
        "message should report the invalid tag"
    );
}
