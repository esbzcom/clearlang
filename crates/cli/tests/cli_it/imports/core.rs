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
fn package_metadata_prerelease_version_reports_c027() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    let pkg_dir = root.join("pkg");
    fs::create_dir_all(&pkg_dir).expect("create pkg dir");
    fs::write(pkg_dir.join("mathpkg.wasm"), [0u8]).expect("write package artifact");

    let main_src = r#"
        function main() -> Int { 0 }
    "#;
    let main_path = root.join("main.clear");
    fs::write(&main_path, main_src.trim()).expect("write main");
    fs::write(
        root.join("clg.package-metadata.json"),
        r#"{
  "schema_version": 1,
  "packages": [
    {
      "name": "mathpkg",
      "version": "1.0.0-alpha",
      "digest": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "artifact": { "format": "wasm", "path": "pkg/mathpkg.wasm" },
      "abi_id": "abi:mathpkg:1.0.0-alpha"
    }
  ]
}"#,
    )
    .expect("write metadata");
    fs::write(
        root.join("clg.package-abi.json"),
        r#"{
  "schema_version": 0,
  "contracts": [
    {
      "abi_id": "abi:mathpkg:1.0.0-alpha",
      "package": "mathpkg",
      "version": "1.0.0-alpha",
      "imports": []
    }
  ]
}"#,
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
fn package_metadata_artifact_parent_traversal_reports_c027() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    let pkg_dir = root.join("pkg");
    fs::create_dir_all(&pkg_dir).expect("create pkg dir");
    fs::write(pkg_dir.join("mathpkg.wasm"), [0u8]).expect("write package artifact");

    let main_src = r#"
        function main() -> Int { 0 }
    "#;
    let main_path = root.join("main.clear");
    fs::write(&main_path, main_src.trim()).expect("write main");
    fs::write(
        root.join("clg.package-metadata.json"),
        r#"{
  "schema_version": 1,
  "packages": [
    {
      "name": "mathpkg",
      "version": "1.0.0",
      "digest": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "artifact": { "format": "wasm", "path": "../pkg/mathpkg.wasm" },
      "abi_id": "abi:mathpkg:1.0.0"
    }
  ]
}"#,
    )
    .expect("write metadata");
    fs::write(
        root.join("clg.package-abi.json"),
        r#"{
  "schema_version": 0,
  "contracts": [
    {
      "abi_id": "abi:mathpkg:1.0.0",
      "package": "mathpkg",
      "version": "1.0.0",
      "imports": []
    }
  ]
}"#,
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

