use assert_cmd::prelude::*;
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

fn run_clg_test_report() -> Value {
    let project_root = repo_root()
        .join("examples")
        .join("projects")
        .join("testing");
    let output = Command::cargo_bin("clg")
        .expect("clg binary")
        .args(["test"])
        .arg(project_root)
        .args(["--report", "json"])
        .output()
        .expect("run clg test for parity summary");
    assert_eq!(
        output.status.code(),
        Some(0),
        "clg test parity fixture must pass"
    );
    serde_json::from_slice(&output.stdout).expect("parse clg test json report")
}

fn parity_summary(report: &Value) -> Value {
    let schema_version = report
        .get("schema_version")
        .and_then(Value::as_u64)
        .expect("report schema_version");
    assert_eq!(
        schema_version, 1,
        "clg test report schema_version must be 1"
    );

    let status = report
        .get("status")
        .and_then(Value::as_str)
        .expect("report status");
    let report_kind = report
        .get("report")
        .and_then(Value::as_str)
        .expect("report kind");
    assert_eq!(report_kind, "json", "parity fixture must use json report");

    let discovered = report
        .get("discovered")
        .and_then(Value::as_u64)
        .expect("report discovered");
    let selected = report
        .get("selected")
        .and_then(Value::as_u64)
        .expect("report selected");
    let executed = report
        .get("executed")
        .and_then(Value::as_u64)
        .expect("report executed");
    let passed = report
        .get("passed")
        .and_then(Value::as_u64)
        .expect("report passed");
    let failed = report
        .get("failed")
        .and_then(Value::as_u64)
        .expect("report failed");

    let tests = report
        .get("tests")
        .and_then(Value::as_array)
        .expect("report tests[]");
    assert_eq!(
        tests.len(),
        executed as usize,
        "tests[] length must match executed count"
    );

    let mut cases = Vec::with_capacity(tests.len());
    let mut failure_codes = Vec::new();
    for case in tests {
        let obj = case.as_object().expect("case object");
        let id = obj
            .get("id")
            .and_then(Value::as_str)
            .expect("case id")
            .to_string();
        let status = obj
            .get("status")
            .and_then(Value::as_str)
            .expect("case status")
            .to_string();
        let timeout_ms = obj
            .get("timeout_ms")
            .and_then(Value::as_u64)
            .expect("case timeout_ms");
        let mock_sets = obj
            .get("mock_sets")
            .and_then(Value::as_array)
            .expect("case mock_sets[]");
        let replay_argv = obj
            .get("replay")
            .and_then(Value::as_object)
            .and_then(|replay| replay.get("argv"))
            .and_then(Value::as_array)
            .expect("case replay.argv[]");
        assert!(
            replay_argv.first().and_then(Value::as_str) == Some("clg")
                && replay_argv.get(1).and_then(Value::as_str) == Some("test"),
            "replay argv must start with clg test"
        );
        cases.push(json!({
            "id": id.clone(),
            "status": status.clone(),
            "timeout_ms": timeout_ms,
            "mock_sets": mock_sets,
        }));
        if let Some(code) = obj.get("failure_code").and_then(Value::as_str) {
            failure_codes.push(json!({
                "id": id,
                "status": status,
                "failure_code": code
            }));
        }
    }
    cases.sort_by(|a, b| a["id"].as_str().cmp(&b["id"].as_str()));

    json!({
        "schema_version": 1,
        "status": status,
        "report": report_kind,
        "discovered": discovered,
        "selected": selected,
        "executed": executed,
        "passed": passed,
        "failed": failed,
        "tests": cases,
        "failure_codes": failure_codes
    })
}

fn write_summary(out_path: &Path, summary: &Value) {
    if let Some(parent) = out_path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).expect("create parity artifact dir");
        }
    }
    fs::write(
        out_path,
        serde_json::to_vec_pretty(summary).expect("serialize parity summary"),
    )
    .expect("write parity summary");
}

#[test]
fn milestone3_test_report_parity_summary_is_deterministic() {
    let report_run1 = run_clg_test_report();
    let report_run2 = run_clg_test_report();

    let summary1 = parity_summary(&report_run1);
    let summary2 = parity_summary(&report_run2);
    assert_eq!(
        summary1, summary2,
        "clg test parity summary should be deterministic for identical inputs"
    );

    if let Ok(out_path) = std::env::var("CLG_TEST_PARITY_OUT") {
        write_summary(Path::new(&out_path), &summary1);
    }
}
