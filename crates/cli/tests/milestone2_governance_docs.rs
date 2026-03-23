use std::fs;
use std::path::Path;

fn parse_last_reviewed_ymd(content: &str) -> (i32, u32, u32) {
    let line = content
        .lines()
        .find(|line| line.trim_start().starts_with("Last reviewed:"))
        .expect("governance doc should include `Last reviewed:` line");
    let raw = line
        .split_once(':')
        .map(|(_, rhs)| rhs.trim().trim_end_matches('.'))
        .expect("`Last reviewed:` line should include date");
    let mut parts = raw.split('-');
    let year = parts
        .next()
        .and_then(|part| part.parse::<i32>().ok())
        .expect("last reviewed year should parse");
    let month = parts
        .next()
        .and_then(|part| part.parse::<u32>().ok())
        .expect("last reviewed month should parse");
    let day = parts
        .next()
        .and_then(|part| part.parse::<u32>().ok())
        .expect("last reviewed day should parse");
    assert!(
        parts.next().is_none(),
        "last reviewed date should be in YYYY-MM-DD format"
    );
    assert!(
        (1..=12).contains(&month),
        "last reviewed month should be in range 1..=12"
    );
    assert!(
        (1..=31).contains(&day),
        "last reviewed day should be in range 1..=31"
    );
    (year, month, day)
}

fn weekday_sunday_zero(year: i32, month: u32, day: u32) -> i32 {
    let month_offsets = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    let mut y = year;
    if month < 3 {
        y -= 1;
    }
    (y + (y / 4) - (y / 100) + (y / 400) + month_offsets[(month - 1) as usize] + day as i32) % 7
}

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
        content.contains("Review cadence: weekly (every Friday)."),
        "governance doc should keep explicit weekly Friday review cadence"
    );
    let (year, month, day) = parse_last_reviewed_ymd(&content);
    let weekday = weekday_sunday_zero(year, month, day);
    assert_eq!(
        weekday, 5,
        "governance `Last reviewed` date should be a Friday when cadence is locked to Friday reviews"
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
