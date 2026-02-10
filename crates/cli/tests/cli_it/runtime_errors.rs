use super::*;

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
fn runtime_array_index_guard_reports_r000_json() {
    let tmp = tempdir().unwrap();
    let src_path = tmp.path().join("array_oob.clear");
    let wasm_path = tmp.path().join("array_oob.wasm");

    let src = r#"
        function main() -> Int { let i = 3; [1, 2][i] }
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
