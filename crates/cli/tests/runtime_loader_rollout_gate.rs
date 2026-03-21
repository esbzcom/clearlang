use std::fs;
use std::path::Path;

#[test]
fn runtime_loader_rollout_gate_doc_covers_canary_rollback_and_release_evidence() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let path = root
        .join("docs")
        .join("runtime")
        .join("runtime-loader-rollout-gate.md");
    let content = fs::read_to_string(&path).expect("read rollout gate doc");

    assert!(
        content.contains("Canary"),
        "rollout gate doc should include canary staging guidance"
    );
    assert!(
        content.contains("Rollback"),
        "rollout gate doc should include rollback criteria/procedure"
    );
    assert!(
        content.contains("Release Gate Evidence"),
        "rollout gate doc should include release evidence requirements"
    );
}
