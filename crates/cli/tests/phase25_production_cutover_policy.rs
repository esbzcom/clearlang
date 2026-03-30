use serde_json::Value;
use std::fs;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

fn read_json(path: PathBuf) -> Value {
    let raw = fs::read(path).expect("read json file");
    serde_json::from_slice(raw.as_slice()).expect("parse json file")
}

#[test]
fn production_cutover_lock_pins_fail_closed_placeholder_policy() {
    let root = repo_root();
    let lock = read_json(
        root.join("docs")
            .join("design")
            .join("phase-25.1.20-production-cutover-fail-closed.lock.json"),
    );
    assert_eq!(lock["schema_version"], Value::from(1));
    assert_eq!(lock["policy_version"], Value::String("25.1.20".to_string()));
    assert_eq!(
        lock["release_profile"],
        Value::String("production".to_string())
    );
    assert_eq!(
        lock["required_compiler_mode"],
        Value::String("strict".to_string())
    );
    assert_eq!(
        lock["required_assurance"],
        Value::String("proved_all".to_string())
    );
    assert_eq!(lock["enforced_by"], Value::String("C121".to_string()));
    let statuses: Vec<&str> = lock["placeholder_statuses_blocked"]
        .as_array()
        .expect("placeholder_statuses_blocked[]")
        .iter()
        .map(|entry| entry.as_str().expect("placeholder status string"))
        .collect();
    assert_eq!(
        statuses,
        vec!["generated", "failed", "unknown", "timeout"],
        "placeholder statuses should be deterministic and complete for cutover gate"
    );
}

#[test]
fn production_cutover_policy_is_present_in_build_gate_logic() {
    let root = repo_root();
    let run_rs = fs::read_to_string(
        root.join("crates")
            .join("cli")
            .join("src")
            .join("commands")
            .join("build")
            .join("run.rs"),
    )
    .expect("read build/run.rs");
    assert!(
        run_rs.contains("release profile `production` requires theorem-grade assurance"),
        "production cutover should fail closed when proof_status is not proved_all"
    );
    assert!(
        run_rs.contains("C121"),
        "production cutover should map non-proved release outputs to C121"
    );
}

#[test]
fn production_cutover_required_commands_are_wired_in_ci() {
    let root = repo_root();
    let lock = read_json(
        root.join("docs")
            .join("design")
            .join("phase-25.1.20-production-cutover-fail-closed.lock.json"),
    );
    let commands: Vec<&str> = lock["required_ci_commands"]
        .as_array()
        .expect("required_ci_commands[]")
        .iter()
        .map(|entry| {
            entry
                .as_str()
                .expect("required_ci_commands[] should be strings")
        })
        .collect();
    let workflow = fs::read_to_string(root.join(".github").join("workflows").join("ci.yml"))
        .expect("read ci workflow");
    for command in commands {
        assert!(
            workflow.contains(command),
            "ci workflow should include required production cutover command `{command}`"
        );
    }
}
