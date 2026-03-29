use std::fs;
use std::path::Path;

use serde_json::Value;

fn read_solver_profile_lock() -> Value {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let bytes = fs::read(root.join("docs/design/phase-25.1.4-solver-profile.lock.json"))
        .expect("read solver profile lock");
    serde_json::from_slice(bytes.as_slice()).expect("parse solver profile lock json")
}

#[test]
fn solver_profile_lock_has_expected_schema_and_policy_version() {
    let lock = read_solver_profile_lock();
    assert_eq!(lock["schema_version"], Value::from(1));
    assert_eq!(lock["policy_version"], Value::String("25.1.4".to_string()));
}

#[test]
fn solver_profile_lock_pins_solver_identity_options_and_timeouts() {
    let lock = read_solver_profile_lock();
    assert_eq!(
        lock["solver_profile"]["solver_family"],
        Value::String("z3".to_string())
    );
    assert!(
        lock["solver_profile"]["solver_version"]
            .as_str()
            .map(str::is_empty)
            == Some(false),
        "solver_version should be non-empty"
    );
    let options = lock["solver_profile"]["options"]
        .as_array()
        .expect("solver_profile.options should be an array");
    assert!(
        !options.is_empty(),
        "solver_profile.options should not be empty"
    );
    assert_eq!(
        lock["solver_profile"]["timeouts_ms"]["per_vc"],
        Value::from(5000)
    );
    assert_eq!(
        lock["solver_profile"]["timeouts_ms"]["total"],
        Value::from(120000)
    );
}

#[test]
fn solver_profile_lock_pins_replay_requirements() {
    let lock = read_solver_profile_lock();
    assert_eq!(
        lock["replay_policy"]["require_identical_profile_hash"],
        Value::Bool(true)
    );
    assert_eq!(
        lock["replay_policy"]["require_identical_vc_outcome_vector"],
        Value::Bool(true)
    );
    assert_eq!(
        lock["replay_policy"]["require_identical_proof_artifact_hash"],
        Value::Bool(true)
    );
}
