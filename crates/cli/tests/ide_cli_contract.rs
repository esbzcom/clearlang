use assert_cmd::prelude::*;
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::tempdir;

fn write_sample_program(root: &Path) -> PathBuf {
    fs::create_dir_all(root).expect("create root");
    let file = root.join("main.clear");
    fs::write(&file, "function main() -> Int { 0 }").expect("write source");
    file
}

fn strict_init(root: &Path) {
    Command::cargo_bin("clg")
        .expect("bin")
        .args(["strict", "init"])
        .arg(root)
        .assert()
        .success();
}

fn current_clg_version_requirement() -> String {
    let current_full = env!("CARGO_PKG_VERSION");
    let current_core = current_full
        .split(['-', '+'])
        .next()
        .unwrap_or(current_full);
    let mut parts = current_core.split('.');
    let major = parts
        .next()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(0);
    let minor = parts
        .next()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(1);
    format!("^{}.{}.0", major, minor)
}

fn write_release_project_defaults(
    path: &Path,
    advisory_as_of: &str,
    key_id: &str,
    entry: &str,
    out_dir: &str,
    trust_policy: &str,
) {
    let value = json!({
        "schema_version": 1,
        "project": {
            "name": "example-app",
            "description": "Example project",
            "version": "0.1.0",
            "clg_version": current_clg_version_requirement(),
            "entry": entry,
            "website": "https://example.com",
            "contact": {
                "name": "Example Maintainer",
                "email": "maintainer@example.com"
            }
        },
        "dependencies": [],
        "release_defaults": {
            "advisory_as_of": advisory_as_of,
            "key_id": key_id,
            "out_dir": out_dir,
            "trust_policy": trust_policy,
        }
    });
    fs::write(
        path,
        serde_json::to_vec_pretty(&value).expect("serialize project defaults"),
    )
    .expect("write project defaults");
}

fn parse_json_lines(lines: &str) -> Vec<Value> {
    lines
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str::<Value>(line).expect("json line"))
        .collect()
}

#[test]
fn ide_contract_check_json_errors_is_machine_readable_on_stdout() {
    let tmp = tempdir().expect("tempdir");
    let root = tmp.path().join("project");
    let file = write_sample_program(&root);

    let output = Command::cargo_bin("clg")
        .expect("bin")
        .args(["--non-interactive", "--json-errors", "check"])
        .arg(&file)
        .args(["--root"])
        .arg(&root)
        .output()
        .expect("run check");
    assert_eq!(output.status.code(), Some(1));

    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    let stderr = String::from_utf8(output.stderr).expect("stderr utf8");
    let payload: Value = serde_json::from_str(stdout.trim()).expect("json payload");
    assert_eq!(payload.get("ok").and_then(|v| v.as_bool()), Some(false));
    let errors = payload
        .get("errors")
        .and_then(|v| v.as_array())
        .expect("errors array");
    assert!(!errors.is_empty(), "expected at least one diagnostic");
    assert!(
        stderr.trim().is_empty(),
        "stderr must remain empty when only --json-errors is enabled"
    );
}

#[test]
fn ide_contract_check_json_events_are_ndjson_schema_v1_on_stderr() {
    let tmp = tempdir().expect("tempdir");
    let root = tmp.path().join("project");
    let file = write_sample_program(&root);
    strict_init(&root);

    let output = Command::cargo_bin("clg")
        .expect("bin")
        .args(["--non-interactive", "--json-events", "check"])
        .arg(&file)
        .args(["--root"])
        .arg(&root)
        .output()
        .expect("run check");
    assert_eq!(output.status.code(), Some(0));

    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    let stderr = String::from_utf8(output.stderr).expect("stderr utf8");
    assert!(stdout.contains("check ok:"));

    let events = parse_json_lines(&stderr);
    assert!(!events.is_empty(), "expected json event stream on stderr");
    for event in &events {
        assert_eq!(
            event.get("schema_version").and_then(|v| v.as_u64()),
            Some(1)
        );
        assert_eq!(event.get("command").and_then(|v| v.as_str()), Some("check"));
    }
}

#[test]
fn ide_contract_release_failure_preserves_stdout_stderr_machine_contract() {
    let tmp = tempdir().expect("tempdir");
    let root = tmp.path().join("project");
    write_sample_program(&root);
    strict_init(&root);
    write_release_project_defaults(
        root.join("clg.project.json").as_path(),
        "2026-03-31T00:00:00Z",
        "ide-contract",
        "main.clear",
        "out/release",
        "trust-policy.json",
    );

    // Force a deterministic release-lock stage failure before key loading/prove/sign.
    fs::remove_file(root.join("clg.package-metadata.json")).expect("remove package metadata");

    let output = Command::cargo_bin("clg")
        .expect("bin")
        .args([
            "--non-interactive",
            "--json-errors",
            "--json-events",
            "release",
        ])
        .args(["--key"])
        .arg(root.join("missing.key.json"))
        .args(["--pubkey"])
        .arg(root.join("missing.pub.json"))
        .args(["--root"])
        .arg(&root)
        .output()
        .expect("run release");
    assert_eq!(output.status.code(), Some(1));

    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    let stderr = String::from_utf8(output.stderr).expect("stderr utf8");
    let payload: Value = serde_json::from_str(stdout.trim()).expect("release json error payload");
    assert_eq!(payload.get("ok").and_then(|v| v.as_bool()), Some(false));
    assert!(
        payload
            .get("errors")
            .and_then(|v| v.as_array())
            .map(|errs| !errs.is_empty())
            .unwrap_or(false),
        "release json-errors payload must include errors[]"
    );

    let events = parse_json_lines(&stderr);
    assert!(!events.is_empty(), "expected release json event stream");
    assert!(events.iter().any(|event| {
        event.get("schema_version").and_then(|v| v.as_u64()) == Some(1)
            && event.get("command").and_then(|v| v.as_str()) == Some("release")
            && event.get("event").and_then(|v| v.as_str()) == Some("start")
            && event.get("stage").and_then(|v| v.as_str()) == Some("release_lock")
    }));
}

#[test]
fn ide_contract_usage_failures_keep_exit_code_2() {
    let usage_status = Command::cargo_bin("clg")
        .expect("bin")
        .args(["check"])
        .output()
        .expect("run usage failure")
        .status;
    assert_eq!(usage_status.code(), Some(2));

    let tmp = tempdir().expect("tempdir");
    let root = tmp.path().join("project");
    write_sample_program(&root);
    let release_usage_status = Command::cargo_bin("clg")
        .expect("bin")
        .args(["release"])
        .output()
        .expect("run release usage failure")
        .status;
    assert_eq!(release_usage_status.code(), Some(2));
}
