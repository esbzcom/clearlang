use std::env;
use std::fs;
use std::path::Path;

fn enforce_gate() -> bool {
    env::var("CLG_ENFORCE_MILESTONE2_RELEASE_GATE")
        .ok()
        .as_deref()
        == Some("1")
}

#[test]
fn milestone2_release_train_gate_requires_24_2_checklist_and_release_notes_when_enforced() {
    if !enforce_gate() {
        return;
    }

    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let todo_path = root.join("docs").join("TODO.md");
    let todo = fs::read_to_string(&todo_path).expect("read docs/TODO.md");
    for idx in 1..=10 {
        let token = format!("  - [x] 24.2.{idx}");
        assert!(
            todo.contains(&token),
            "release-train gate requires checklist item `{token}` to be complete before milestone_2 tag"
        );
    }

    let release_notes_path = root.join("release_notes").join("milestone_2.md");
    assert!(
        release_notes_path.exists(),
        "release-train gate requires `release_notes/milestone_2.md`"
    );
}
