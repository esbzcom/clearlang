use std::fs;
use std::path::Path;

#[test]
fn milestone2_performance_evidence_doc_references_gate_script_and_artifact() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let path = root
        .join("docs")
        .join("evidence")
        .join("milestone_2-performance.md");
    let content = fs::read_to_string(&path).expect("read milestone_2 performance evidence doc");

    assert!(
        content.contains("milestone2_perf_gate.sh"),
        "performance evidence doc should reference CI gate script"
    );
    assert!(
        content.contains("milestone2-performance.json"),
        "performance evidence doc should reference machine-readable performance artifact"
    );
    assert!(
        content.contains("startup overhead") && content.contains("package resolution/link"),
        "performance evidence doc should include startup and package-resolution metrics"
    );
    assert!(
        content.contains("runtime-link startup latency"),
        "performance evidence doc should include runtime-link startup metric"
    );
}
