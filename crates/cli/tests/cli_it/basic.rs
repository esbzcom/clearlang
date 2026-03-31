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
fn run_namespaced_fixture_clear_source_succeeds() {
    let mut cmd = Command::cargo_bin("clg").unwrap();
    cmd.args(["run"])
        .arg(repo_sample("16_namespaced_call.clear"));
    cmd.assert()
        .success()
        .stdout(predicate::str::is_match(r"(?m)^\s*3\s*$").unwrap());
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
fn parse_rejects_theorem_keyword_with_explicit_code() {
    let src = r#"
        theorem function add(x: Int, y: Int) -> Int { x + y }
        function main() -> Int { add(1, 2) }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("theorem_keyword.clear");
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
            .any(|e| e.get("code").and_then(|s| s.as_str()) == Some("P014")),
        "expected P014 in {:?}",
        errs
    );
    assert!(
        errs.iter().any(|e| {
            e.get("message")
                .and_then(|s| s.as_str())
                .unwrap_or("")
                .contains("theorem-grade is a release certification status")
        }),
        "expected explicit deferred theorem syntax message"
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

#[test]
fn release_help_exposes_gate_c_primary_shape() {
    let output = Command::cargo_bin("clg")
        .unwrap()
        .args(["release", "--help"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let help = String::from_utf8(output).expect("utf8");
    assert!(help.contains("release [OPTIONS]"));
    assert!(help.contains("--advisory-as-of"));
    assert!(help.contains("--key"));
    assert!(help.contains("--key-id"));
    assert!(help.contains("--pubkey"));
    assert!(help.contains("--root"));
    assert!(help.contains("--out-dir"));
    assert!(help.contains("--trust-policy"));
    assert!(
        !help.contains("--compiler-mode"),
        "release command shape should hide build internals from primary help"
    );
    assert!(
        !help.contains("--release-profile"),
        "release command should not expose release profile downgrades"
    );
    assert!(
        !help.contains("--proof-strict"),
        "release command should not expose proof strictness downgrade controls"
    );
    assert!(
        !help.contains("--require-assurance"),
        "release command should not expose assurance downgrade controls"
    );
}

#[test]
fn release_requires_minimal_required_flags() {
    let output = Command::cargo_bin("clg")
        .unwrap()
        .args(["release", "clearlang-tests/01_hello.clear"])
        .assert()
        .failure()
        .get_output()
        .stderr
        .clone();
    let text = String::from_utf8(output).expect("utf8");
    assert!(text.contains("--key"));
    assert!(text.contains("--pubkey"));
}

#[test]
fn strict_help_exposes_init_subcommand() {
    let output = Command::cargo_bin("clg")
        .unwrap()
        .args(["strict", "--help"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let help = String::from_utf8(output).expect("utf8");
    assert!(help.contains("strict"));
    assert!(help.contains("init"));
}

#[test]
fn strict_init_creates_required_preflight_files() {
    let tmp = tempdir().expect("tempdir");
    let root = tmp.path().join("project");
    Command::cargo_bin("clg")
        .unwrap()
        .args(["strict", "init"])
        .arg(&root)
        .assert()
        .success();

    for file in [
        "clg.project.json",
        "clg.lock.json",
        "clg.trust-policy.json",
        "clg.host-profile.json",
        "clg.package-metadata.json",
        "clg.package-abi.json",
        "trust-policy.json",
    ] {
        assert!(
            root.join(file).exists(),
            "strict init should create `{}`",
            file
        );
    }
}

#[test]
fn strict_init_fails_when_existing_preflight_file_is_invalid() {
    let tmp = tempdir().expect("tempdir");
    let root = tmp.path().join("project");
    fs::create_dir_all(&root).expect("create root");
    fs::write(root.join("clg.lock.json"), r#"{"schema_version":"bad"}"#).expect("write bad lock");

    let output = Command::cargo_bin("clg")
        .unwrap()
        .args(["--json-errors", "strict", "init"])
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
    assert_eq!(e0.get("code").and_then(|s| s.as_str()), Some("C104"));
    assert_eq!(e0.get("stage").and_then(|s| s.as_str()), Some("strict"));
}
