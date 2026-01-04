use assert_cmd::prelude::*;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::tempdir;

fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/proofs/fixtures")
}

fn normalize_vcs(value: &mut Value) {
    let arr = value.as_array_mut().expect("vc array");
    for item in arr {
        if let Some(obj) = item.as_object_mut() {
            obj.remove("positions");
        }
    }
}

fn assert_vcs_fixture(source: &str, fixture: &str) {
    let tmp = tempdir().expect("tempdir");
    let src_path = tmp.path().join("fixture.clear");
    let wasm_path = tmp.path().join("fixture.wasm");
    let vcs_path = tmp.path().join("fixture.vc.json");
    fs::write(&src_path, source).expect("write source");

    Command::cargo_bin("clg")
        .unwrap()
        .args(["build"])
        .arg(&src_path)
        .args(["-o"])
        .arg(&wasm_path)
        .arg("--emit-vcs")
        .arg(&vcs_path)
        .assert()
        .success();

    let mut actual: Value =
        serde_json::from_str(&fs::read_to_string(&vcs_path).expect("read vcs")).expect("vcs json");
    normalize_vcs(&mut actual);

    let mut expected: Value = serde_json::from_str(
        &fs::read_to_string(fixtures_dir().join(fixture)).expect("read fixture"),
    )
    .expect("fixture json");
    normalize_vcs(&mut expected);

    assert_eq!(actual, expected, "fixture mismatch for {}", fixture);
}

#[test]
fn emit_vcs_matches_refinement_fixtures() {
    let cases = [
        (
            "refinement-basic.vc.json",
            r#"
            type Nat = Int where n >= 0;
            pure function inc(a: Nat) -> Nat { a + 1 }
            function main() -> Int { 0 }
        "#,
        ),
        (
            "refinement-contracts-loops.vc.json",
            r#"
            type Nat = Int where n >= 0;
            pure function countdown(n: Nat) -> Int
                require { n > 1 }
                ensure { result >= 0 }
            {
                while n > 0 invariant { n >= 0 } variant { n } { n; }
                n + 0
            }
            function main() -> Int { 0 }
        "#,
        ),
        (
            "refinement-call-site.vc.json",
            r#"
            type Nat = Int where n >= 0;
            pure function takes(n: Nat) -> Nat { n }
            pure function caller(x: Int) -> Nat
                require { x >= 0 }
                ensure { result >= 0 }
            { takes(x + 1) }
            function main() -> Int { 0 }
        "#,
        ),
    ];

    for (fixture, source) in cases {
        assert_vcs_fixture(source, fixture);
    }
}
