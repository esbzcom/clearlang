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
            obj.remove("proof_status");
            if let Some(diagnostics) = obj.get_mut("diagnostics").and_then(|d| d.as_object_mut()) {
                if let Some(failure_slice) = diagnostics
                    .get_mut("failure_slice")
                    .and_then(|s| s.as_object_mut())
                {
                    if let Some(focus_span) = failure_slice
                        .get_mut("focus_span")
                        .and_then(|s| s.as_object_mut())
                    {
                        focus_span.remove("file");
                        focus_span.remove("start");
                        focus_span.remove("end");
                    }
                    if let Some(related_spans) = failure_slice
                        .get_mut("related_spans")
                        .and_then(|s| s.as_array_mut())
                    {
                        for span in related_spans {
                            if let Some(span_obj) = span.as_object_mut() {
                                span_obj.remove("file");
                                span_obj.remove("start");
                                span_obj.remove("end");
                            }
                        }
                    }
                }
                if let Some(counterexample) = diagnostics
                    .get_mut("counterexample")
                    .and_then(|s| s.as_object_mut())
                {
                    if let Some(focus_span) = counterexample
                        .get_mut("focus_span")
                        .and_then(|s| s.as_object_mut())
                    {
                        focus_span.remove("file");
                        focus_span.remove("start");
                        focus_span.remove("end");
                    }
                }
                if let Some(proof_context) = diagnostics
                    .get_mut("proof_context")
                    .and_then(|s| s.as_object_mut())
                {
                    if let Some(span_map) = proof_context
                        .get_mut("span_map")
                        .and_then(|s| s.as_object_mut())
                    {
                        span_map.remove("pre");
                        span_map.remove("post");
                    }
                }
            }
        }
    }
}

fn assert_vcs_fixture(source: &str, fixture: &str) {
    let tmp = tempdir().expect("tempdir");
    let src_path = tmp.path().join("fixture.clear");
    let wasm_path = tmp.path().join("fixture.wasm");
    let vcs_path = tmp.path().join("fixture.vc.json");
    fs::write(&src_path, source).expect("write source");
    let missing_solver = if cfg!(windows) {
        tmp.path().join("missing-z3.exe")
    } else {
        tmp.path().join("missing-z3")
    };

    let mut cmd = Command::cargo_bin("clg").unwrap();
    cmd.env("CLG_SOLVER_BIN", &missing_solver)
        .args(["build"])
        .arg(&src_path)
        .args(["-o"])
        .arg(&wasm_path)
        .arg("--emit-vcs")
        .arg(&vcs_path);
    cmd.assert().success();

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
        (
            "linear-collections-branch.vc.json",
            r#"
            resource File { drop {} }
            pure function choose(flag: Bool, consume files: List<File>) -> List<File> {
                if flag {
                    std::list::remove_take(files, 0)[0]
                } else {
                    std::list::remove_take(files, 0)[0]
                }
            }
            function main() -> Int { 0 }
        "#,
        ),
        (
            "linear-collections-branch-inline.vc.json",
            r#"
            resource File { drop {} }
            pure function id(consume files: List<File>) -> List<File> { files }
            pure function choose(flag: Bool, consume files: List<File>) -> List<File> {
                if flag {
                    std::list::remove_take(id(files), 0)[0]
                } else {
                    std::list::remove_take(id(files), 0)[0]
                }
            }
            function main() -> Int { 0 }
        "#,
        ),
        (
            "linear-collections-loop.vc.json",
            r#"
            resource File { drop {} }
            pure function loop_step(consume files: List<File>, n: Int) -> List<File> {
                while n > 0 invariant { n >= 0 } variant { n } {
                    let out = std::list::remove_take(files, 0);
                    let files = out[0];
                }
                files
            }
            function main() -> Int { 0 }
        "#,
        ),
        (
            "proof-model-assumptions.vc.json",
            r#"
            pure function check(a: Bytes, b: Bytes, x: U64, y: U64) -> Bool
                ensure { result == std::bytes::eq_ct(a, b) }
                ensure { (x & y) == x }
            {
                std::bytes::eq_ct(a, b)
            }
            function main() -> Int { 0 }
        "#,
        ),
    ];

    for (fixture, source) in cases {
        assert_vcs_fixture(source, fixture);
    }
}
