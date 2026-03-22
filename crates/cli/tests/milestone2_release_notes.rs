use std::fs;
use std::path::Path;

#[test]
fn milestone2_release_notes_published_with_release_evidence_sections() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let path = root.join("release_notes").join("milestone_2.md");
    let content = fs::read_to_string(&path).expect("read milestone_2 release notes");

    assert!(
        content.contains("Milestone 2 Release Notes"),
        "milestone_2 release notes title should be present"
    );
    assert!(
        content.contains("Published on"),
        "release notes should include published status"
    );
    assert!(
        !content.to_ascii_lowercase().contains("draft"),
        "release notes should not be marked as draft once milestone_2 is published"
    );
    assert!(
        content.contains("Release Evidence"),
        "release notes should include release evidence section"
    );
    assert!(
        content.contains("24.2.8") && content.contains("milestone_2-readiness.md"),
        "release notes should reference readiness evidence for 24.2.8"
    );
    assert!(
        content.contains("24.2.9") && content.contains("milestone_2-performance.md"),
        "release notes should reference performance evidence for 24.2.9"
    );
    assert!(
        content.contains("24.2.10") && content.contains("milestone_2-supply-chain.md"),
        "release notes should reference supply-chain evidence for 24.2.10"
    );
    assert!(
        content.contains("Post-Review Hardening (Option 1)"),
        "release notes should document approved option 1 hardening decisions"
    );
    assert!(
        content.contains("Publish Criteria Status"),
        "release notes should include explicit publish criteria status section"
    );
}
