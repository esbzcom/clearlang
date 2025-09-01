use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::tempdir;

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
    let mut cmd = Command::cargo_bin("lumi-cli").expect("bin");
    cmd.args(["parse"]).arg(&input);
    cmd.assert().success().stdout(predicate::str::contains("Program"));
}

#[test]
fn emit_hello_writes_valid_wasm_and_runs() {
    let tmp = tempdir().unwrap();
    let out = tmp.path().join("hello.wasm");
    let mut cmd = Command::cargo_bin("lumi-cli").unwrap();
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
    ];
    for (file, expect) in cases {
        let tmp = tempdir().unwrap();
        let out = tmp.path().join("out.wasm");
        let mut cmd = Command::cargo_bin("lumi-cli").unwrap();
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
    let mut cmd = Command::cargo_bin("lumi-cli").unwrap();
    cmd.args(["parse"]).arg(repo_sample("06_trailing_call_comma.lumi"));
    cmd.assert().failure();
}

#[test]
fn build_failure_for_non_int_or_missing_main() {
    let tmp = tempdir().unwrap();
    let out = tmp.path().join("bad.wasm");
    let mut cmd = Command::cargo_bin("lumi-cli").unwrap();
    // 07_bools has no main; codegen should fail
    cmd.args(["build"]).arg(repo_sample("07_bools.lumi")).args(["-o"]).arg(&out);
    cmd.assert().failure();
}

