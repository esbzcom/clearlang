use std::fs;
use std::path::Path;

#[test]
fn milestone2_readiness_evidence_includes_security_runbook_signing_and_release_notes() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let path = root.join("docs").join("evidence").join("milestone_2-readiness.md");
    let content = fs::read_to_string(&path).expect("read milestone_2 readiness evidence");

    assert!(
        content.contains("## Security Review"),
        "readiness evidence should include security review section"
    );
    assert!(
        content.contains("## Runbooks"),
        "readiness evidence should include runbooks section"
    );
    assert!(
        content.contains("## Signed Artifacts"),
        "readiness evidence should include signed artifacts section"
    );
    assert!(
        content.contains("## Release Notes"),
        "readiness evidence should include release notes section"
    );
}
