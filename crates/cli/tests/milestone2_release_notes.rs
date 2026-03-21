use std::fs;
use std::path::Path;

#[test]
fn milestone2_release_notes_draft_exists_with_publish_blockers_section() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let path = root.join("release_notes").join("milestone_2.md");
    let content = fs::read_to_string(&path).expect("read milestone_2 release notes");

    assert!(
        content.contains("Milestone 2 Release Notes"),
        "milestone_2 release notes title should be present"
    );
    assert!(
        content.contains("Remaining Publish Blockers"),
        "release notes draft should include explicit publish blockers"
    );
    assert!(
        content.contains("24.2.8") && content.contains("24.2.9") && content.contains("24.2.10"),
        "release notes draft should track unresolved go-live gates"
    );
}
