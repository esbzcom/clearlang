use std::fs;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

fn todo_has_checked_item(todo: &str, item: &str) -> bool {
    todo.lines().any(|line| {
        let line = line.trim_start();
        let Some(rest) = line.strip_prefix("- [x] ") else {
            return false;
        };
        let Some(after) = rest.strip_prefix(item) else {
            return false;
        };
        after
            .chars()
            .next()
            .map(|ch| ch.is_ascii_whitespace())
            .unwrap_or(true)
    })
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

#[test]
fn todo_marks_gate_e_cutover_policy_complete() {
    let root = repo_root();
    let todo = fs::read_to_string(root.join("docs").join("TODO.md")).expect("read todo");
    for item in [
        "25.4", "25.4.1", "25.4.2", "25.4.3", "25.4.4", "25.4.5", "25.4.6", "25.4.7", "25.4.8",
        "25.4.9", "25.4.10", "25.4.11", "25.4.12",
    ] {
        assert!(
            todo_has_checked_item(&todo, item),
            "TODO should mark `{item}` complete for Gate E closeout"
        );
    }
}
