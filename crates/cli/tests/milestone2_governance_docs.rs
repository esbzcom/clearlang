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
    assert!(
        content.contains(
            "| Phase 24 (host profiles + milestone exit) | `@felto` | `@felto` | complete |"
        ),
        "governance doc should mark phase 24 ownership status complete"
    );
    assert!(
        content.contains("Last reviewed:"),
        "governance doc should include a concrete risk-register review date"
    );
    assert!(
        content.contains("`24.2.8`)")
            && content.contains("`24.2.9`)")
            && content.contains("`24.2.10`)"),
        "governance doc should track milestone_2 evidence risks for 24.2.8/24.2.9/24.2.10"
    );
    assert!(
        content.contains("| mitigated |"),
        "governance doc should mark milestone_2 risk rows with explicit mitigated status"
    );
    assert!(
        !content.contains("| open |") && !content.contains("| in_progress |"),
        "governance doc should not leave milestone_2 control rows open or in_progress after completion"
    );
    assert!(
        content.contains("all `24.2.x` checklist items in `docs/TODO.md` are checked (`[x]`)")
            && content.contains("`release_notes/milestone_2.md` exists"),
        "governance doc should keep release-train policy aligned with enforced gate criteria"
    );
}
