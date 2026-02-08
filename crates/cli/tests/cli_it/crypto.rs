use super::*;

#[test]
fn run_crypto_hash_stub_returns_len() {
    let tmp = tempdir().unwrap();
    let src_path = tmp.path().join("crypto_hash.clear");
    let wasm_path = tmp.path().join("crypto_hash.wasm");

    let src = r#"
        io function main() -> Int {
            std::bytes::len(std::crypto::hash("sha256", std::bytes::from_string("hi")))
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
        .stdout(predicate::str::contains("32"));
}

#[test]
fn runtime_crypto_unknown_alg_reports_r006_json() {
    let tmp = tempdir().unwrap();
    let src_path = tmp.path().join("crypto_bad_alg.clear");
    let wasm_path = tmp.path().join("crypto_bad_alg.wasm");
    let wasm_display = wasm_path.display().to_string();

    let src = r#"
        io function main() -> Int {
            std::bytes::len(std::crypto::hash("sha999", std::bytes::from_string("hi")))
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
    assert_eq!(e0.get("code").and_then(|s| s.as_str()), Some("R006"));
    assert_eq!(e0.get("stage").and_then(|s| s.as_str()), Some("runtime"));
    assert_eq!(
        e0.get("message").and_then(|s| s.as_str()),
        Some("crypto algorithm unsupported or unknown")
    );
    assert_eq!(
        e0.get("file").and_then(|s| s.as_str()),
        Some(wasm_display.as_str())
    );
    assert_eq!(e0.get("start").and_then(|n| n.as_u64()), Some(0));
    assert_eq!(e0.get("end").and_then(|n| n.as_u64()), Some(0));
    assert!(e0.get("function").is_none());
}

#[test]
fn runtime_crypto_invalid_length_reports_r007_json() {
    let tmp = tempdir().unwrap();
    let src_path = tmp.path().join("crypto_bad_len.clear");
    let wasm_path = tmp.path().join("crypto_bad_len.wasm");
    let wasm_display = wasm_path.display().to_string();

    let src = r#"
        io function main() -> Int {
            std::crypto::verify(
                "ed25519",
                std::bytes::from_string(""),
                std::bytes::from_string("short"),
                std::bytes::from_string("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
            );
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
    assert!(!v.get("ok").and_then(|b| b.as_bool()).unwrap_or(true));
    let errs = v.get("errors").and_then(|e| e.as_array()).expect("errors");
    assert_eq!(errs.len(), 1);
    let e0 = &errs[0];
    assert_eq!(e0.get("code").and_then(|s| s.as_str()), Some("R007"));
    assert_eq!(e0.get("stage").and_then(|s| s.as_str()), Some("runtime"));
    assert_eq!(
        e0.get("message").and_then(|s| s.as_str()),
        Some("crypto input length is invalid for selected algorithm")
    );
    assert_eq!(
        e0.get("file").and_then(|s| s.as_str()),
        Some(wasm_display.as_str())
    );
    assert_eq!(e0.get("start").and_then(|n| n.as_u64()), Some(0));
    assert_eq!(e0.get("end").and_then(|n| n.as_u64()), Some(0));
    assert!(e0.get("function").is_none());
}

#[test]
fn runtime_crypto_malformed_reports_r008_json() {
    let tmp = tempdir().unwrap();
    let src_path = tmp.path().join("crypto_bad_encoding.clear");
    let wasm_path = tmp.path().join("crypto_bad_encoding.wasm");
    let wasm_display = wasm_path.display().to_string();

    let src = r#"
        io function main() -> Int {
            std::crypto::verify(
                "secp256k1",
                std::bytes::from_string("msg"),
                std::bytes::from_string("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"),
                std::bytes::from_string("!aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
            );
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
    assert!(!v.get("ok").and_then(|b| b.as_bool()).unwrap_or(true));
    let errs = v.get("errors").and_then(|e| e.as_array()).expect("errors");
    assert_eq!(errs.len(), 1);
    let e0 = &errs[0];
    assert_eq!(e0.get("code").and_then(|s| s.as_str()), Some("R008"));
    assert_eq!(e0.get("stage").and_then(|s| s.as_str()), Some("runtime"));
    assert_eq!(
        e0.get("message").and_then(|s| s.as_str()),
        Some("crypto input is malformed")
    );
    assert_eq!(
        e0.get("file").and_then(|s| s.as_str()),
        Some(wasm_display.as_str())
    );
    assert_eq!(e0.get("start").and_then(|n| n.as_u64()), Some(0));
    assert_eq!(e0.get("end").and_then(|n| n.as_u64()), Some(0));
    assert!(e0.get("function").is_none());
}

#[test]
fn run_bytes_eq_ct_returns_true() {
    let tmp = tempdir().unwrap();
    let src_path = tmp.path().join("bytes_eq_ct.clear");
    let wasm_path = tmp.path().join("bytes_eq_ct.wasm");

    let src = r#"
        function main() -> Int {
            if std::bytes::eq_ct(std::bytes::from_string("hi"), std::bytes::from_string("hi")) {
                1
            } else {
                0
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
        .stdout(predicate::str::contains("1"));
}

