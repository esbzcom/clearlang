use std::fs;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

#[test]
fn gate_e_trust_policy_ux_clarification_doc_is_published() {
    let root = repo_root();
    let path = root
        .join("docs")
        .join("design")
        .join("phase-25.4.11-trust-policy-ux-clarification.md");
    let doc = fs::read_to_string(&path).expect("read gate e trust-policy clarification doc");
    assert!(doc.contains("25.4.11"));
    assert!(doc.contains("trust-policy.json"));
    assert!(doc.contains("clg.trust-policy.json"));
    assert!(doc.contains("release_defaults.trust_policy"));
    assert!(doc.contains("V004"));
}
