use super::*;

#[test]
fn build_and_run_with_imports() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    let math_dir = root.join("math");
    fs::create_dir_all(&math_dir).expect("create math dir");

    let arith = r#"
        export function add(a: Int, b: Int) -> Int { a + b }
    "#;
    fs::write(math_dir.join("arith.clear"), arith.trim()).expect("write arith");

    let main_src = r#"
        import math::arith

        function main() -> Int {
            arith::add(40, 2)
        }
    "#;
    let main_path = root.join("main.clear");
    fs::write(&main_path, main_src.trim()).expect("write main");

    let wasm_path = root.join("out.wasm");
    Command::cargo_bin("clg")
        .unwrap()
        .args(["build"])
        .arg(&main_path)
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
        .stdout(predicate::str::contains("42"));
}

#[test]
fn import_requires_exported_item() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    let util_dir = root.join("util");
    fs::create_dir_all(&util_dir).expect("create util dir");

    let helper = r#"
        function hidden(a: Int) -> Int { a }
    "#;
    fs::write(util_dir.join("helper.clear"), helper.trim()).expect("write helper");

    let main_src = r#"
        import util::helper::{hidden}

        function main() -> Int { hidden(1) }
    "#;
    let main_path = root.join("main.clear");
    fs::write(&main_path, main_src.trim()).expect("write main");

    let wasm_path = root.join("out.wasm");
    let output = Command::cargo_bin("clg")
        .unwrap()
        .args(["--json-errors", "build"])
        .arg(&main_path)
        .args(["-o"])
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
    assert_eq!(e0.get("code").and_then(|s| s.as_str()), Some("C021"));
    assert_eq!(e0.get("stage").and_then(|s| s.as_str()), Some("build"));
}

#[test]
fn import_missing_module_reports_c020() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();

    let main_src = r#"
        import missing::thing
        function main() -> Int { 0 }
    "#;
    let main_path = root.join("main.clear");
    fs::write(&main_path, main_src.trim()).expect("write main");

    let wasm_path = root.join("out.wasm");
    let output = Command::cargo_bin("clg")
        .unwrap()
        .args(["--json-errors", "build"])
        .arg(&main_path)
        .args(["-o"])
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
    assert_eq!(e0.get("code").and_then(|s| s.as_str()), Some("C020"));
    assert_eq!(e0.get("stage").and_then(|s| s.as_str()), Some("build"));
}

#[test]
fn import_name_conflict_reports_c022() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    let math_dir = root.join("math");
    fs::create_dir_all(&math_dir).expect("create math dir");

    let arith = r#"
        export function add(a: Int, b: Int) -> Int { a + b }
    "#;
    fs::write(math_dir.join("arith.clear"), arith.trim()).expect("write arith");

    let main_src = r#"
        import math::arith
        function arith() -> Int { 1 }
        function main() -> Int { arith() }
    "#;
    let main_path = root.join("main.clear");
    fs::write(&main_path, main_src.trim()).expect("write main");

    let wasm_path = root.join("out.wasm");
    let output = Command::cargo_bin("clg")
        .unwrap()
        .args(["--json-errors", "build"])
        .arg(&main_path)
        .args(["-o"])
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
    assert_eq!(e0.get("code").and_then(|s| s.as_str()), Some("C022"));
    assert_eq!(e0.get("stage").and_then(|s| s.as_str()), Some("build"));
}

#[test]
fn import_cycle_reports_c025() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    let a_dir = root.join("a");
    let b_dir = root.join("b");
    fs::create_dir_all(&a_dir).expect("create a dir");
    fs::create_dir_all(&b_dir).expect("create b dir");

    let a_src = r#"
        import b::modb
        export function ping() -> Int { 1 }
    "#;
    fs::write(a_dir.join("moda.clear"), a_src.trim()).expect("write a");

    let b_src = r#"
        import a::moda
        export function pong() -> Int { 2 }
    "#;
    fs::write(b_dir.join("modb.clear"), b_src.trim()).expect("write b");

    let main_src = r#"
        import a::moda
        function main() -> Int { moda::ping() }
    "#;
    let main_path = root.join("main.clear");
    fs::write(&main_path, main_src.trim()).expect("write main");

    let wasm_path = root.join("out.wasm");
    let output = Command::cargo_bin("clg")
        .unwrap()
        .args(["--json-errors", "build"])
        .arg(&main_path)
        .args(["-o"])
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
    assert_eq!(e0.get("code").and_then(|s| s.as_str()), Some("C025"));
    assert_eq!(e0.get("stage").and_then(|s| s.as_str()), Some("build"));
}

#[test]
fn entry_file_is_root_namespace() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    let util_dir = root.join("util");
    fs::create_dir_all(&util_dir).expect("create util dir");

    let util_src = r#"
        import app::{helper}
        export function use_helper() -> Int { helper() }
    "#;
    fs::write(util_dir.join("consumer.clear"), util_src.trim()).expect("write consumer");

    let entry_src = r#"
        import util::consumer
        export function helper() -> Int { 5 }
        function main() -> Int { 0 }
    "#;
    let entry_path = root.join("app.clear");
    fs::write(&entry_path, entry_src.trim()).expect("write entry");

    let wasm_path = root.join("out.wasm");
    let output = Command::cargo_bin("clg")
        .unwrap()
        .args(["--json-errors", "build"])
        .arg(&entry_path)
        .args(["-o"])
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
    assert_eq!(e0.get("code").and_then(|s| s.as_str()), Some("C020"));
    assert_eq!(e0.get("stage").and_then(|s| s.as_str()), Some("build"));
}

#[test]
fn import_items_and_aliases_work() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    let math_dir = root.join("math");
    fs::create_dir_all(&math_dir).expect("create math dir");

    let arith = r#"
        export function add(a: Int, b: Int) -> Int { a + b }
        export function sub(a: Int, b: Int) -> Int { a - b }
    "#;
    fs::write(math_dir.join("arith.clear"), arith.trim()).expect("write arith");

    let main_src = r#"
        import math::arith as ar
        import math::arith::{add}

        function main() -> Int {
            add(1, 2) + ar::sub(4, 1)
        }
    "#;
    let main_path = root.join("main.clear");
    fs::write(&main_path, main_src.trim()).expect("write main");

    let wasm_path = root.join("out.wasm");
    Command::cargo_bin("clg")
        .unwrap()
        .args(["build"])
        .arg(&main_path)
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
        .stdout(predicate::str::contains("6"));
}

#[test]
fn import_std_module_and_items_work() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();

    let main_src = r#"
        import std::bytes as b
        import std::bytes::{len}

        function main() -> Int {
            len(std::bytes::from_string("hi")) + b::len(std::bytes::from_string("a"))
        }
    "#;
    let main_path = root.join("main.clear");
    fs::write(&main_path, main_src.trim()).expect("write main");

    let wasm_path = root.join("out.wasm");
    Command::cargo_bin("clg")
        .unwrap()
        .args(["build"])
        .arg(&main_path)
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
        .stdout(predicate::str::contains("3"));
}

#[test]
fn import_unknown_std_module_reports_c020() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();

    let main_src = r#"
        import std::missing
        function main() -> Int { 0 }
    "#;
    let main_path = root.join("main.clear");
    fs::write(&main_path, main_src.trim()).expect("write main");

    let wasm_path = root.join("out.wasm");
    let output = Command::cargo_bin("clg")
        .unwrap()
        .args(["--json-errors", "build"])
        .arg(&main_path)
        .args(["-o"])
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
    assert_eq!(e0.get("code").and_then(|s| s.as_str()), Some("C020"));
    assert_eq!(e0.get("stage").and_then(|s| s.as_str()), Some("build"));
}

#[test]
fn import_user_defined_std_module_reports_c026() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    let std_dir = root.join("std");
    fs::create_dir_all(&std_dir).expect("create std dir");

    let std_src = r#"
        export function foo() -> Int { 1 }
    "#;
    fs::write(std_dir.join("user.clear"), std_src.trim()).expect("write std user");

    let main_src = r#"
        import std::user
        function main() -> Int { 0 }
    "#;
    let main_path = root.join("main.clear");
    fs::write(&main_path, main_src.trim()).expect("write main");

    let wasm_path = root.join("out.wasm");
    let output = Command::cargo_bin("clg")
        .unwrap()
        .args(["--json-errors", "build"])
        .arg(&main_path)
        .args(["-o"])
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
    assert_eq!(e0.get("code").and_then(|s| s.as_str()), Some("C026"));
    assert_eq!(e0.get("stage").and_then(|s| s.as_str()), Some("build"));
}

#[test]
fn import_unknown_std_item_reports_c021() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();

    let main_src = r#"
        import std::bytes::{missing}
        function main() -> Int { 0 }
    "#;
    let main_path = root.join("main.clear");
    fs::write(&main_path, main_src.trim()).expect("write main");

    let wasm_path = root.join("out.wasm");
    let output = Command::cargo_bin("clg")
        .unwrap()
        .args(["--json-errors", "build"])
        .arg(&main_path)
        .args(["-o"])
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
    assert_eq!(e0.get("code").and_then(|s| s.as_str()), Some("C021"));
    assert_eq!(e0.get("stage").and_then(|s| s.as_str()), Some("build"));
}

#[test]
fn import_std_chain_type_item_allows_build() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();

    let main_src = r#"
        import std::eth::{Address}
        function id(a: Address) -> Address { a }
        function main() -> Int { 0 }
    "#;
    let main_path = root.join("main.clear");
    fs::write(&main_path, main_src.trim()).expect("write main");

    let wasm_path = root.join("out.wasm");
    Command::cargo_bin("clg")
        .unwrap()
        .args(["build"])
        .arg(&main_path)
        .args(["-o"])
        .arg(&wasm_path)
        .assert()
        .success();
}
