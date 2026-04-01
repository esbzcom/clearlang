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
            .any(|line| line.trim_start().starts_with("- [x] 25.2.1 ")),
        "TODO must mark 25.2.1 complete once Gate C policy lock ships"
    );
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
    assert!(
        todo.lines()
            .any(|line| line.trim_start().starts_with("- [x] 25.2.4 ")),
        "TODO must mark 25.2.4 complete once proved-only release defaults ship"
    );
    assert!(
        todo.lines()
            .any(|line| line.trim_start().starts_with("- [x] 25.2.5 ")),
        "TODO must mark 25.2.5 complete once strict init defaults ship"
    );
    assert!(
        todo.lines()
            .any(|line| line.trim_start().starts_with("- [x] 25.2.6 ")),
        "TODO must mark 25.2.6 complete once migration guidance ships"
    );
    assert!(
        todo.lines()
            .any(|line| line.trim_start().starts_with("- [x] 25.2.7 ")),
        "TODO must mark 25.2.7 complete once check command ships"
    );
    assert!(
        todo.lines()
            .any(|line| line.trim_start().starts_with("- [x] 25.2.8 ")),
        "TODO must mark 25.2.8 complete once IDE contract ships"
    );
    assert!(
        todo.lines()
            .any(|line| line.trim_start().starts_with("- [x] 25.2.9 ")),
        "TODO must mark 25.2.9 complete once non-interactive contract ships"
    );
}

#[test]
fn gate_c_policy_lock_note_is_published() {
    let root = repo_root();
    let path = root
        .join("docs")
        .join("design")
        .join("phase-25.2.1-gate-c-policy-lock.md");
    let doc = fs::read_to_string(&path).expect("read gate c policy lock");
    assert!(doc.contains("25.2.1"));
    assert!(doc.contains("Simplified command options"));
    assert!(doc.contains("One-command release path"));
    assert!(doc.contains("Built-in Z3 migration path"));
    assert!(doc.contains("IDE/VSCode-first CLI contracts"));
    assert!(doc.contains("transitional"));
    assert!(doc.contains("**not** accepted for production release artifacts"));
    assert!(doc.contains("25.2.6"));
    assert!(doc.contains("25.2.7"));
    assert!(doc.contains("backward-compatibility is **not** a goal"));
    assert!(doc.contains("remove legacy/compatibility surfaces"));
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

#[test]
fn gate_c_release_proved_only_default_note_is_published() {
    let root = repo_root();
    let path = root
        .join("docs")
        .join("design")
        .join("phase-25.2.4-release-proved-only-default.md");
    let doc = fs::read_to_string(&path).expect("read gate c proved-only note");
    assert!(doc.contains("25.2.4"));
    assert!(doc.contains("no optional downgrade path"));
    assert!(doc.contains("C121"));
}

#[test]
fn gate_c_strict_init_bootstrap_note_is_published() {
    let root = repo_root();
    let path = root
        .join("docs")
        .join("design")
        .join("phase-25.2.5-strict-init-bootstrap.md");
    let doc = fs::read_to_string(&path).expect("read gate c strict init note");
    assert!(doc.contains("25.2.5"));
    assert!(doc.contains("clg strict init <root>"));
    assert!(doc.contains("clg.project.json"));
    assert!(doc.contains("C130"));
}

#[test]
fn gate_c_release_migration_guidance_note_is_published() {
    let root = repo_root();
    let path = root
        .join("docs")
        .join("design")
        .join("phase-25.2.6-release-migration-guidance.md");
    let doc = fs::read_to_string(&path).expect("read gate c migration note");
    assert!(doc.contains("25.2.6"));
    assert!(doc.contains("clg release"));
    assert!(doc.contains("expert/debug-only"));
}

#[test]
fn gate_c_check_command_note_is_published() {
    let root = repo_root();
    let path = root
        .join("docs")
        .join("design")
        .join("phase-25.2.7-check-command.md");
    let doc = fs::read_to_string(&path).expect("read gate c check note");
    assert!(doc.contains("25.2.7"));
    assert!(doc.contains("clg check"));
    assert!(doc.contains("strict"));
}

#[test]
fn gate_c_ide_contract_note_is_published() {
    let root = repo_root();
    let path = root
        .join("docs")
        .join("design")
        .join("phase-25.2.8-ide-cli-contract.md");
    let doc = fs::read_to_string(&path).expect("read gate c ide contract note");
    assert!(doc.contains("25.2.8"));
    assert!(doc.contains("--json-errors"));
    assert!(doc.contains("--json-events"));
    assert!(doc.contains("Exit-Code Contract"));
}

#[test]
fn gate_c_non_interactive_contract_note_is_published() {
    let root = repo_root();
    let path = root
        .join("docs")
        .join("design")
        .join("phase-25.2.9-non-interactive-cli-contract.md");
    let doc = fs::read_to_string(&path).expect("read gate c non-interactive note");
    assert!(doc.contains("25.2.9"));
    assert!(doc.contains("--non-interactive"));
    assert!(doc.contains("stdout/stderr"));
}
