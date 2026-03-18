use std::fs;
use std::path::Path;

use serde_json::Value;

fn read_policy_lock() -> Value {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let bytes = fs::read(root.join("docs/design/phase-22.1.0-resolver-policy.lock.json"))
        .expect("read resolver policy lock");
    serde_json::from_slice(bytes.as_slice()).expect("parse resolver policy lock json")
}

#[test]
fn resolver_policy_lock_has_expected_schema_and_version() {
    let lock = read_policy_lock();
    assert_eq!(lock["schema_version"], Value::from(1));
    assert_eq!(lock["policy_version"], Value::String("22.1.0".to_string()));
}

#[test]
fn resolver_policy_lock_pins_candidate_sort_and_backtracking_order() {
    let lock = read_policy_lock();
    assert_eq!(
        lock["resolver"]["candidate_sort"],
        serde_json::json!(["semver_desc", "digest_lex_asc", "artifact_path_lex_asc"])
    );
    assert_eq!(
        lock["resolver"]["backtracking_order"]["unresolved_package"],
        Value::String("earliest_name_asc".to_string())
    );
    assert_eq!(
        lock["resolver"]["backtracking_order"]["candidate_choice"],
        Value::String("first_sorted_candidate".to_string())
    );
}

#[test]
fn resolver_policy_lock_pins_conflict_precedence_and_diagnostics_order() {
    let lock = read_policy_lock();
    assert_eq!(
        lock["conflict_precedence"],
        serde_json::json!([
            "advisory_deny",
            "advisory_forced_upgrade",
            "semver_unsat",
            "determinism_tie_break_failure"
        ])
    );
    assert_eq!(
        lock["diagnostics_order"],
        serde_json::json!([
            "code",
            "package_name",
            "package_version",
            "symbol_or_import"
        ])
    );
}
