use super::*;
use wasmparser::{Parser, Payload};

fn wasm_import_pairs(wasm: &[u8]) -> Vec<(String, String)> {
    let mut imports = Vec::new();
    for payload in Parser::new(0).parse_all(wasm) {
        if let Payload::ImportSection(reader) = payload.expect("payload") {
            for item in reader {
                let import = item.expect("import");
                imports.push((import.module.to_string(), import.name.to_string()));
            }
        }
    }
    imports
}

fn write_canonical_package_inputs(
    root: &std::path::Path,
    package_name: &str,
    version: &str,
    artifact_path: &str,
    abi_id: &str,
    imports_json: &str,
) {
    let metadata = format!(
        r#"{{
  "schema_version": 0,
  "packages": [
    {{
      "name": "{package_name}",
      "version": "{version}",
      "digest": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "artifact": {{ "format": "wasm", "path": "{artifact_path}" }},
      "abi_id": "{abi_id}"
    }}
  ]
}}"#
    );
    fs::write(root.join("clg.package-metadata.json"), metadata).expect("write metadata");

    let abi = format!(
        r#"{{
  "schema_version": 0,
  "contracts": [
    {{
      "abi_id": "{abi_id}",
      "package": "{package_name}",
      "version": "{version}",
      "imports": {imports_json}
    }}
  ]
}}"#
    );
    fs::write(root.join("clg.package-abi.json"), abi).expect("write abi");
}

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
fn build_with_compiled_package_import_succeeds() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    let pkg_dir = root.join("pkg");
    fs::create_dir_all(&pkg_dir).expect("create pkg dir");
    fs::write(pkg_dir.join("mathpkg.wasm"), [0u8]).expect("write package artifact");

    write_canonical_package_inputs(
        root,
        "mathpkg",
        "1.0.0",
        "pkg/mathpkg.wasm",
        "abi:mathpkg:1.0.0",
        r#"[
  {
    "symbol": "mathpkg::arith::add2",
    "effect": "pure",
    "params": ["Int", "Int"],
    "ret": "Int",
    "capability": null
  }
]"#,
    );

    let main_src = r#"
        import mathpkg::arith::{add2}
        function main() -> Int { add2(40, 2) }
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

    let wasm = fs::read(&wasm_path).expect("read wasm");
    let imports = wasm_import_pairs(&wasm);
    let saw_import = imports
        .iter()
        .any(|(module, name)| module == "mathpkg::arith" && name == "add2");
    assert!(saw_import, "expected external import mathpkg::arith.add2");
}

#[test]
fn build_with_compiled_package_import_prunes_unused_exports() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    let pkg_dir = root.join("pkg");
    fs::create_dir_all(&pkg_dir).expect("create pkg dir");
    fs::write(pkg_dir.join("mathpkg.wasm"), [0u8]).expect("write package artifact");

    write_canonical_package_inputs(
        root,
        "mathpkg",
        "1.0.0",
        "pkg/mathpkg.wasm",
        "abi:mathpkg:1.0.0",
        r#"[
  {
    "symbol": "mathpkg::arith::add2",
    "effect": "pure",
    "params": ["Int", "Int"],
    "ret": "Int",
    "capability": null
  },
  {
    "symbol": "mathpkg::arith::sub2",
    "effect": "pure",
    "params": ["Int", "Int"],
    "ret": "Int",
    "capability": null
  }
]"#,
    );

    let main_src = r#"
        import mathpkg::arith::{add2}
        function main() -> Int { add2(40, 2) }
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

    let wasm = fs::read(&wasm_path).expect("read wasm");
    let imports = wasm_import_pairs(&wasm);
    let has_add2 = imports
        .iter()
        .any(|(module, name)| module == "mathpkg::arith" && name == "add2");
    let has_sub2 = imports
        .iter()
        .any(|(module, name)| module == "mathpkg::arith" && name == "sub2");
    assert!(has_add2, "expected used import mathpkg::arith.add2");
    assert!(
        !has_sub2,
        "unused export mathpkg::arith.sub2 should not be emitted as wasm import"
    );
}

#[test]
fn invalid_package_metadata_reports_c027() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();

    let main_src = r#"
        function main() -> Int { 0 }
    "#;
    let main_path = root.join("main.clear");
    fs::write(&main_path, main_src.trim()).expect("write main");
    fs::write(
        root.join("clg.package-metadata.json"),
        r#"{ "schema_version": 9, "packages": [] }"#,
    )
    .expect("write metadata");
    fs::write(
        root.join("clg.package-abi.json"),
        r#"{ "schema_version": 0, "contracts": [] }"#,
    )
    .expect("write abi");

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
    assert_eq!(e0.get("code").and_then(|s| s.as_str()), Some("C027"));
    assert_eq!(e0.get("stage").and_then(|s| s.as_str()), Some("build"));
}

#[test]
fn legacy_package_metadata_is_rejected_reports_c027() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();

    let main_src = r#"
        function main() -> Int { 0 }
    "#;
    let main_path = root.join("main.clear");
    fs::write(&main_path, main_src.trim()).expect("write main");
    fs::write(
        root.join("clg-packages.json"),
        r#"{ "schema_version": 1, "packages": [] }"#,
    )
    .expect("write legacy metadata");

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
    assert_eq!(e0.get("code").and_then(|s| s.as_str()), Some("C027"));
    assert_eq!(e0.get("stage").and_then(|s| s.as_str()), Some("build"));
}

#[test]
fn source_and_package_module_conflict_reports_c028() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    let pkg_dir = root.join("pkg");
    let src_dir = root.join("mathpkg");
    fs::create_dir_all(&pkg_dir).expect("create pkg dir");
    fs::create_dir_all(&src_dir).expect("create src dir");
    fs::write(pkg_dir.join("mathpkg.wasm"), [0u8]).expect("write package artifact");

    write_canonical_package_inputs(
        root,
        "mathpkg",
        "1.0.0",
        "pkg/mathpkg.wasm",
        "abi:mathpkg:1.0.0",
        r#"[
  {
    "symbol": "mathpkg::arith::add2",
    "effect": "pure",
    "params": ["Int", "Int"],
    "ret": "Int",
    "capability": null
  }
]"#,
    );
    fs::write(
        src_dir.join("arith.clear"),
        "export function add2(a: Int, b: Int) -> Int { a + b }",
    )
    .expect("write local module");

    let main_src = r#"
        import mathpkg::arith
        function main() -> Int { arith::add2(1, 2) }
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
    assert_eq!(e0.get("code").and_then(|s| s.as_str()), Some("C028"));
    assert_eq!(e0.get("stage").and_then(|s| s.as_str()), Some("build"));
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
fn import_std_list_remove_take_item_works() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();

    let src = r#"
        import std::list::{remove_take}

        function list_next(l: List<Int>) -> List<Int> {
            remove_take(l, 0)[0]
        }

        function main() -> Int { 0 }
    "#;
    let main_path = root.join("main.clear");
    fs::write(&main_path, src.trim()).expect("write main");

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

#[test]
fn import_std_map_take_items_work() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();

    let src = r#"
        import std::map::{insert_take, remove_take}

        function map_next(m: Map<Int, Int>) -> Map<Int, Int> {
            remove_take(insert_take(m, 1, 99)[0], 1)[0]
        }

        function main() -> Int { 0 }
    "#;
    let main_path = root.join("main.clear");
    fs::write(&main_path, src.trim()).expect("write main");

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
