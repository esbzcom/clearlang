use assert_cmd::Command;
use std::path::PathBuf;
use predicates::prelude::PredicateBooleanExt;

fn sample(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../lumi-tests").join(name)
}

#[test]
fn emit_hello_and_run() {
    let out = tempfile::Builder::new().prefix("emit_hello_").suffix(".wasm").tempfile().unwrap();
    let out_path = out.path().to_path_buf();

    Command::cargo_bin("lumi")
        .expect("bin")
        .args(["emit-hello", "-o"])
        .arg(&out_path)
        .assert()
        .success();

    Command::cargo_bin("lumi")
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
    Command::cargo_bin("lumi").unwrap()
        .args(["build"]).arg(sample("01_hello.lumi")).args(["-o"]).arg(&out1)
        .assert().success();
    Command::cargo_bin("lumi").unwrap()
        .args(["run"]).arg(&out1)
        .assert().success()
        .stdout(predicates::str::contains("42\n"));

    // 12_zero_arg_fn
    let out2 = tempfile::tempdir().unwrap().path().join("zfn.wasm");
    Command::cargo_bin("lumi").unwrap()
        .args(["build"]).arg(sample("12_zero_arg_fn.lumi")).args(["-o"]).arg(&out2)
        .assert().success();
    Command::cargo_bin("lumi").unwrap()
        .args(["run"]).arg(&out2)
        .assert().success()
        .stdout(predicates::str::contains("7\n"));
}

#[test]
fn parse_command_prints_ast() {
    let sample = sample("01_hello.lumi");
    Command::cargo_bin("lumi").unwrap()
        .args(["parse"])
        .arg(&sample)
        .assert()
        .success()
        .stdout(predicates::str::contains("add2").and(predicates::str::contains("main")));
}

#[test]
fn build_fails_on_parse_error() {
    let out = tempfile::tempdir().unwrap().path().join("bad.wasm");
    Command::cargo_bin("lumi").unwrap()
        .args(["build"])
        .arg(sample("06_trailing_call_comma.lumi"))
        .args(["-o"]).arg(&out)
        .assert()
        .failure();
}

#[test]
fn build_fails_on_type_error() {
    let out = tempfile::tempdir().unwrap().path().join("type_err.wasm");
    Command::cargo_bin("lumi").unwrap()
        .args(["build"])
        .arg(sample("14_arity_mismatch.lumi"))
        .args(["-o"]).arg(&out)
        .assert()
        .failure();
}
