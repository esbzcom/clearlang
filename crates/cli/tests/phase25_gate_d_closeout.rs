use assert_cmd::prelude::*;
use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

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
fn gate_d_cli_help_and_about_mark_test_as_shipped_primary_command() {
    let root = repo_root();
    let cli_main = fs::read_to_string(root.join("crates").join("cli").join("src").join("main.rs"))
        .expect("read cli main");
    assert!(
        cli_main.contains("primary: check, test, release;"),
        "cli about string should mark test as a shipped primary command"
    );

    let help = Command::cargo_bin("clg")
        .expect("clg binary")
        .arg("--help")
        .output()
        .expect("run clg --help");
    assert_eq!(help.status.code(), Some(0));
    let stdout = String::from_utf8(help.stdout).expect("help stdout utf8");
    assert!(
        stdout.contains("test") && stdout.contains("check") && stdout.contains("release"),
        "help output should include primary check/test/release commands"
    );

    let test_help = Command::cargo_bin("clg")
        .expect("clg binary")
        .args(["test", "--help"])
        .output()
        .expect("run clg test --help");
    assert_eq!(test_help.status.code(), Some(0));
    let test_stdout = String::from_utf8(test_help.stdout).expect("test help stdout utf8");
    for flag in [
        "--plan",
        "--mock-set",
        "--timeout-ms",
        "--fail-fast",
        "--list",
    ] {
        assert!(
            !test_stdout.contains(flag),
            "deferred non-essential test flag should not be exposed in shipped CLI surface: {flag}"
        );
    }
}

#[test]
fn gate_d_quality_matrix_and_migration_policy_docs_are_published() {
    let root = repo_root();
    let matrix = fs::read_to_string(root.join("docs").join("testing-quality-matrix.md"))
        .expect("read matrix");
    assert!(matrix.contains("No fixed numeric thresholds"));
    assert!(matrix.contains("Parser + discovery contract"));
    assert!(matrix.contains("Balanced mock confidence"));
    assert!(matrix.contains("Runtime worker safety limits"));

    let testing = fs::read_to_string(root.join("docs").join("testing.md")).expect("read testing");
    assert!(testing.contains("Migration Policy (25.3.8)"));
    assert!(testing.contains("clearlang-tests/"));
    assert!(testing.contains("Balanced Mock Confidence Gate (25.3.18)"));
    assert!(testing.contains("Runtime Worker Safety Policy (25.3.25)"));
    assert!(testing.contains("Proof-Mode Policy (25.3.5)"));
    assert!(testing.contains("Deferred Flags Policy (25.3.19.1)"));
    assert!(testing.contains("Single-process policy"));
}

#[test]
fn gate_d_design_and_policy_lock_docs_are_published() {
    let root = repo_root();

    let gate_d_lock = fs::read_to_string(
        root.join("docs")
            .join("design")
            .join("phase-25.3.0-gate-d-design-lock.md"),
    )
    .expect("read gate d design lock");
    assert!(gate_d_lock.contains("Phase 25.3.0"));
    assert!(gate_d_lock.contains("Design-Principle Alignment"));
    assert!(gate_d_lock.contains("Gate D Exit Criterion"));
    assert!(gate_d_lock.contains("clg test"));
    assert!(gate_d_lock.contains("release == proved"));
    assert!(gate_d_lock.contains("single active process per project root"));

    let proof_mode = fs::read_to_string(
        root.join("docs")
            .join("design")
            .join("phase-25.3.5-test-proof-mode-policy.md"),
    )
    .expect("read test proof-mode policy");
    assert!(proof_mode.contains("25.3.5"));
    assert!(proof_mode.contains("one deterministic default mode"));
    assert!(proof_mode.contains("release == proved"));
    assert!(proof_mode.contains("proved_all"));

    let deferred_flags = fs::read_to_string(
        root.join("docs")
            .join("design")
            .join("phase-25.3.19.1-test-cli-deferred-flags-policy.md"),
    )
    .expect("read deferred flags policy");
    assert!(deferred_flags.contains("25.3.19.1"));
    assert!(deferred_flags.contains("--plan"));
    assert!(deferred_flags.contains("--mock-set"));
    assert!(deferred_flags.contains("--timeout-ms"));
    assert!(deferred_flags.contains("--fail-fast"));
    assert!(deferred_flags.contains("--list"));
    assert!(deferred_flags.contains("tests/test-plan.json"));

    let evidence = fs::read_to_string(
        root.join("docs")
            .join("design")
            .join("phase-25.3-evidence-index.md"),
    )
    .expect("read gate d evidence index");
    assert!(evidence.contains("Phase 25.3 - Implementation Evidence Index"));
    assert!(evidence.contains("crates/cli/src/commands/test.rs"));
    assert!(evidence.contains("crates/cli/src/main.rs"));
    assert!(evidence.contains("xtask/src/main/core.rs"));
}

#[test]
fn gate_d_primary_docs_are_aligned_with_shipped_test_contracts() {
    let root = repo_root();
    let readme = fs::read_to_string(root.join("README.md")).expect("read README");
    assert!(readme.contains("primary UX contract is `check`/`test`/`release`"));
    assert!(readme.contains("docs/testing-quality-matrix.md"));
    assert!(readme.contains("phase-25.3.0-gate-d-design-lock.md"));

    let release_process = fs::read_to_string(root.join("docs").join("release-process.md"))
        .expect("read release docs");
    assert!(release_process.contains("Shipped primary commands:"));
    assert!(release_process.contains("- `clg test` (shipped in `25.3.2`)"));
    assert!(
        !release_process.contains("Planned primary command (reserved contract):"),
        "release process should no longer present clg test as planned"
    );

    let vscode = fs::read_to_string(root.join("docs").join("ide").join("vscode-cli-profile.md"))
        .expect("read vscode profile");
    assert!(vscode.contains("--json-events test"));
    assert!(vscode.contains("runtime safety reasons for `C138`"));

    let diagnostics =
        fs::read_to_string(root.join("docs").join("diagnostics.md")).expect("read diagnostics");
    assert!(diagnostics.contains("C138"));
    assert!(diagnostics.contains("fuel/memory worker-limit trap"));
}

#[test]
fn gate_d_balanced_mock_and_non_mocked_coverage_is_enforced_in_example() {
    let root = repo_root();
    let example_root = root.join("examples").join("projects").join("testing");
    let output = Command::cargo_bin("clg")
        .expect("clg binary")
        .args(["test"])
        .arg(&example_root)
        .args(["--report", "json"])
        .output()
        .expect("run clg test example");
    assert_eq!(output.status.code(), Some(0));

    let payload: Value = serde_json::from_slice(&output.stdout).expect("parse test json report");
    let tests = payload
        .get("tests")
        .and_then(Value::as_array)
        .expect("tests array");
    assert!(!tests.is_empty(), "expected at least one executed test");

    let non_mocked = tests
        .iter()
        .filter(|case| {
            case.get("mock_sets")
                .and_then(Value::as_array)
                .map(|sets| sets.is_empty())
                .unwrap_or(false)
        })
        .count();
    let mocked = tests
        .iter()
        .filter(|case| {
            case.get("mock_sets")
                .and_then(Value::as_array)
                .map(|sets| !sets.is_empty())
                .unwrap_or(false)
        })
        .count();
    assert!(
        non_mocked > 0 && mocked > 0,
        "example suite must include both non-mocked and mocked critical-path tests"
    );
}

#[test]
fn todo_marks_gate_d_phase_completion_items_complete() {
    let root = repo_root();
    let todo = fs::read_to_string(root.join("docs").join("TODO.md")).expect("read todo");
    for item in [
        "25.3",
        "25.3.0",
        "25.3.0.1",
        "25.3.5",
        "25.3.6",
        "25.3.8",
        "25.3.18",
        "25.3.19.1",
        "25.3.28",
        "25.3.25",
        "25.3.26",
        "25.3.29",
        "25.3.30",
        "25.3.31",
        "25.3.32",
        "25.3.33",
    ] {
        assert!(
            todo_has_checked_item(&todo, item),
            "TODO should mark `{item}` complete for Gate D final closeout"
        );
    }
}
