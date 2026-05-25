use std::fs;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

#[test]
fn gate_e_lockfile_tool_owned_drift_gate_doc_is_published() {
    let root = repo_root();
    let path = root
        .join("docs")
        .join("design")
        .join("phase-25.4.10-lockfile-tool-owned-drift-gate.md");
    let doc = fs::read_to_string(&path).expect("read gate e lockfile tool-owned drift gate");
    assert!(doc.contains("25.4.10"));
    assert!(doc.contains("manifest-lock-drift-check"));
    assert!(doc.contains("canonical tool serializer"));
    assert!(doc.contains("clg.resolved-graph.sha256"));
}
