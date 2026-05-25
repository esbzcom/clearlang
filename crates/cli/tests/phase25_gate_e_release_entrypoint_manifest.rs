use std::fs;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

#[test]
fn gate_e_release_entrypoint_manifest_doc_is_published() {
    let root = repo_root();
    let path = root
        .join("docs")
        .join("design")
        .join("phase-25.4.12-release-entrypoint-manifest-cutover.md");
    let doc = fs::read_to_string(&path).expect("read gate e release-entrypoint manifest doc");
    assert!(doc.contains("25.4.12"));
    assert!(doc.contains("project.entry"));
    assert!(doc.contains("clg release --key"));
    assert!(doc.contains("C130"));
}
