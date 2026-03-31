use std::fs;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

#[test]
fn gate_c_release_ux_design_lock_is_published() {
    let root = repo_root();
    let path = root
        .join("docs")
        .join("design")
        .join("phase-25.2.2-release-ux-design-lock.md");
    let doc = fs::read_to_string(&path).expect("read gate c ux lock");
    assert!(doc.contains("Phase 25.2.2"));
    assert!(doc.contains("clg check"));
    assert!(doc.contains("clg test"));
    assert!(doc.contains("clg release"));
    assert!(doc.contains("--advisory-as-of"));
    assert!(doc.contains("--key"));
    assert!(doc.contains("--pubkey"));
    assert!(doc.contains("verify(require-assurance=proved_all)"));
}

#[test]
fn todo_marks_gate_c_ux_design_lock_complete() {
    let root = repo_root();
    let todo = fs::read_to_string(root.join("docs").join("TODO.md")).expect("read todo");
    assert!(
        todo.lines()
            .any(|line| line.trim_start().starts_with("- [x] 25.2.2 ")),
        "TODO must mark 25.2.2 complete once design lock ships"
    );
    assert!(
        todo.lines()
            .any(|line| line.trim_start().starts_with("- [x] 25.2.3 ")),
        "TODO must mark 25.2.3 complete once orchestration ships"
    );
}

#[test]
fn gate_c_release_orchestration_design_note_is_published() {
    let root = repo_root();
    let path = root
        .join("docs")
        .join("design")
        .join("phase-25.2.3-release-command-orchestration.md");
    let doc = fs::read_to_string(&path).expect("read gate c release orchestration note");
    assert!(doc.contains("25.2.3"));
    assert!(doc.contains("lock -> build/prove -> sign -> verify -> bundle"));
    assert!(doc.contains("clg.trust-policy.json"));
    assert!(doc.contains("trust-policy.json"));
}
