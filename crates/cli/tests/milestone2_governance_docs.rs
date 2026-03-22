use std::fs;
use std::path::Path;

#[test]
fn milestone2_governance_doc_covers_dri_dates_risks_and_release_train_gate() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let path = root
        .join("docs")
        .join("rollout")
        .join("milestone_2-governance.md");
    let content = fs::read_to_string(&path).expect("read milestone_2 governance doc");

    assert!(
        content.contains("24.3.1 DRI Ownership"),
        "governance doc should include DRI ownership section"
    );
    assert!(
        content.contains("24.3.2 Target Dates + Critical Path"),
        "governance doc should include target dates and critical path section"
    );
    assert!(
        content.contains("24.3.3 Risk Register"),
        "governance doc should include risk register section"
    );
    assert!(
        content.contains("24.3.4 Release-Train Gate Policy"),
        "governance doc should include release-train gate section"
    );
    assert!(
        content.contains("@felto"),
        "governance doc should include explicit DRI assignments"
    );
}
