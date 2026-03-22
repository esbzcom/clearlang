use std::fs;
use std::path::Path;

#[test]
fn runtime_loader_rollout_gate_doc_covers_canary_rollback_and_release_evidence() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let doc_path = root
        .join("docs")
        .join("runtime")
        .join("runtime-loader-rollout-gate.md");
    let content = fs::read_to_string(&doc_path).expect("read rollout gate doc");
    assert!(
        content.contains("runtime-loader-rollout-gate.lock.json"),
        "rollout gate doc should reference the machine-readable lock artifact"
    );

    let lock_path = root
        .join("docs")
        .join("runtime")
        .join("runtime-loader-rollout-gate.lock.json");
    let lock: serde_json::Value =
        serde_json::from_slice(&fs::read(&lock_path).expect("read rollout lock artifact"))
            .expect("parse rollout lock artifact");
    assert!(
        lock.get("schema_version").and_then(|v| v.as_u64()) == Some(1),
        "rollout lock artifact must use schema_version=1"
    );
    assert!(
        lock.get("required_diagnostics")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>()
                    == vec!["R012", "R013", "R014", "R015", "R016", "R017"]
            })
            .unwrap_or(false),
        "rollout lock artifact should pin required runtime diagnostics R012-R017"
    );
    assert!(
        lock.get("canary")
            .and_then(|v| v.get("requirements"))
            .and_then(|v| v.as_array())
            .map(|arr| !arr.is_empty())
            .unwrap_or(false),
        "rollout lock artifact should define non-empty canary requirements"
    );
    assert!(
        lock.get("rollback")
            .and_then(|v| v.get("triggers"))
            .and_then(|v| v.as_array())
            .map(|arr| !arr.is_empty())
            .unwrap_or(false),
        "rollout lock artifact should define rollback triggers"
    );
    assert!(
        lock.get("release_gate_evidence")
            .and_then(|v| v.as_array())
            .map(|arr| !arr.is_empty())
            .unwrap_or(false),
        "rollout lock artifact should define required release evidence artifacts"
    );
}
