use std::fs;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

#[test]
fn gate_e_pre_ga_cutover_policy_doc_is_published() {
    let root = repo_root();
    let path = root
        .join("docs")
        .join("design")
        .join("phase-25.4.7-pre-ga-cutover-policy.md");
    let doc = fs::read_to_string(&path).expect("read gate e pre-ga cutover policy");
    assert!(doc.contains("25.4.7"));
    assert!(doc.contains("fail closed"));
    assert!(doc.contains("no backward-compatibility commitment"));
    assert!(doc.contains("25.4.8"));
    assert!(doc.contains("25.4.9"));
    assert!(doc.contains("25.4.10"));
    assert!(doc.contains("25.4.11"));
    assert!(doc.contains("25.4.12"));
}
