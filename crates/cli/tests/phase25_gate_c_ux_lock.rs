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
    assert!(
        todo.lines()
            .any(|line| line.trim_start().starts_with("- [x] 25.2.10 ")),
        "TODO must mark 25.2.10 complete once VSCode command profile docs ship"
    );
    assert!(
        todo.lines()
            .any(|line| line.trim_start().starts_with("- [x] 25.2.11 ")),
        "TODO must mark 25.2.11 complete once IDE CI contract tests ship"
    );
    assert!(
        todo.lines()
            .any(|line| line.trim_start().starts_with("- [x] 25.2.12 ")),
        "TODO must mark 25.2.12 complete once release-precheck gate ships"
    );
    assert!(
        todo.lines()
            .any(|line| line.trim_start().starts_with("- [x] 25.2.13 ")),
        "TODO must mark 25.2.13 complete once clg fmt ships"
    );
    assert!(
        todo.lines()
            .any(|line| line.trim_start().starts_with("- [x] 25.2.14 ")),
        "TODO must mark 25.2.14 complete once clg lint ships"
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

#[test]
fn gate_c_vscode_command_profile_design_note_is_published() {
    let root = repo_root();
    let path = root
        .join("docs")
        .join("design")
        .join("phase-25.2.10-vscode-command-profile.md");
    let doc = fs::read_to_string(&path).expect("read gate c vscode profile note");
    assert!(doc.contains("25.2.10"));
    assert!(doc.contains("--non-interactive"));
    assert!(doc.contains("--json-errors"));
    assert!(doc.contains("--json-events"));
    assert!(doc.contains("schema_version: 1"));
    assert!(doc.contains("Cancellation and Timeout Contract"));
}

#[test]
fn gate_c_vscode_plugin_profile_doc_is_published() {
    let root = repo_root();
    let path = root.join("docs").join("ide").join("vscode-cli-profile.md");
    let doc = fs::read_to_string(&path).expect("read vscode plugin profile doc");
    assert!(doc.contains("VSCode CLI Profile"));
    assert!(doc.contains("clg --non-interactive --json-errors --json-events check"));
    assert!(doc.contains("clg --non-interactive --json-errors --json-events release"));
    assert!(doc.contains("\"schema_version\":1"));
    assert!(doc.contains("Cancellation and Timeout Behavior"));
}

#[test]
fn gate_c_ide_ci_contract_note_and_ci_step_are_published() {
    let root = repo_root();
    let path = root
        .join("docs")
        .join("design")
        .join("phase-25.2.11-ide-cli-ci-contract-tests.md");
    let doc = fs::read_to_string(&path).expect("read gate c ide ci contract note");
    assert!(doc.contains("25.2.11"));
    assert!(doc.contains("ide_cli_contract.rs"));
    assert!(doc.contains("IDE CLI contract gates"));
    assert!(doc.contains("schema_version: 1"));

    let ci = fs::read_to_string(root.join(".github").join("workflows").join("ci.yml"))
        .expect("read ci workflow");
    assert!(ci.contains("name: IDE CLI contract gates"));
    assert!(ci.contains("cargo test -p clg-cli --test ide_cli_contract"));
}

#[test]
fn gate_c_release_precheck_gate_is_published_and_wired() {
    let root = repo_root();
    let path = root
        .join("docs")
        .join("design")
        .join("phase-25.2.12-release-precheck-gate.md");
    let doc = fs::read_to_string(&path).expect("read gate c release precheck note");
    assert!(doc.contains("25.2.12"));
    assert!(doc.contains("release-precheck"));
    assert!(doc.contains("fmt --all -- --check"));
    assert!(doc.contains("clippy --workspace --all-targets -- -D warnings"));
    assert!(doc.contains("test --workspace"));

    let xtask_core =
        fs::read_to_string(root.join("xtask").join("src").join("main").join("core.rs"))
            .expect("read xtask core");
    assert!(xtask_core.contains("\"release-precheck\""));
    assert!(xtask_core.contains("run_release_precheck"));

    let ci = fs::read_to_string(root.join(".github").join("workflows").join("ci.yml"))
        .expect("read ci workflow");
    assert!(ci.contains("name: Release precheck gate (fmt + lint + tests)"));
    assert!(ci.contains("cargo run -p xtask -- release-precheck"));
}

#[test]
fn gate_c_clg_fmt_command_is_published() {
    let root = repo_root();
    let path = root
        .join("docs")
        .join("design")
        .join("phase-25.2.13-clg-fmt-command.md");
    let doc = fs::read_to_string(&path).expect("read gate c clg fmt note");
    assert!(doc.contains("25.2.13"));
    assert!(doc.contains("clg fmt <PATH>"));
    assert!(doc.contains("--check"));
    assert!(doc.contains("C132"));

    let cli_main = fs::read_to_string(root.join("crates").join("cli").join("src").join("main.rs"))
        .expect("read cli main");
    assert!(cli_main.contains("Fmt {"));
    assert!(cli_main.contains("fmt as cmd_fmt"));
}

#[test]
fn gate_c_clg_lint_command_is_published() {
    let root = repo_root();
    let path = root
        .join("docs")
        .join("design")
        .join("phase-25.2.14-clg-lint-command.md");
    let doc = fs::read_to_string(&path).expect("read gate c clg lint note");
    assert!(doc.contains("25.2.14"));
    assert!(doc.contains("clg lint <PATH>"));
    assert!(doc.contains("--deny-warnings"));
    assert!(doc.contains("C133"));

    let cli_main = fs::read_to_string(root.join("crates").join("cli").join("src").join("main.rs"))
        .expect("read cli main");
    assert!(cli_main.contains("Lint {"));
    assert!(cli_main.contains("lint as cmd_lint"));
}
