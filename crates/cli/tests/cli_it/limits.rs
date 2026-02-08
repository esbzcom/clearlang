use super::*;

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
