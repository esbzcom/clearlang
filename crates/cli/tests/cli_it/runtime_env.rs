use super::*;

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
fn run_env_random_negative_length_reports_r002() {
    let tmp = tempdir().unwrap();
    let src_path = tmp.path().join("env_random_neg.clear");
    let wasm_path = tmp.path().join("env_random_neg.wasm");

    let src = r#"
        io function main() -> Int {
            std::bytes::len(std::env::random(0 - 1))
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
    assert!(!v.get("ok").and_then(|b| b.as_bool()).unwrap_or(true));
    let errs = v.get("errors").and_then(|e| e.as_array()).expect("errors");
    assert_eq!(errs.len(), 1);
    let e0 = &errs[0];
    assert_eq!(e0.get("code").and_then(|s| s.as_str()), Some("R002"));
    assert_eq!(e0.get("stage").and_then(|s| s.as_str()), Some("runtime"));
}

#[test]
fn run_env_stubs_are_replay_deterministic() {
    let tmp = tempdir().unwrap();
    let src_path = tmp.path().join("env_replay.clear");
    let wasm_path = tmp.path().join("env_replay.wasm");

    let src = r#"
        io function main() -> Int {
            std::bytes::len(std::env::random(8)) + std::env::time()
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

    let first = Command::cargo_bin("clg")
        .unwrap()
        .args(["run"])
        .arg(&wasm_path)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let second = Command::cargo_bin("clg")
        .unwrap()
        .args(["run"])
        .arg(&wasm_path)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    assert_eq!(
        first, second,
        "expected identical stdout on replay for deterministic env stubs"
    );
    assert_eq!(
        String::from_utf8_lossy(&first).trim(),
        "8",
        "expected deterministic env output to remain time=0 + len(random(8))=8"
    );
}
