use serde_yaml::{Mapping, Value};
use std::fs;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

fn as_mapping<'a>(value: &'a Value, ctx: &str) -> &'a Mapping {
    value
        .as_mapping()
        .unwrap_or_else(|| panic!("{ctx} should be a YAML mapping"))
}

fn as_sequence<'a>(value: &'a Value, ctx: &str) -> &'a [Value] {
    value
        .as_sequence()
        .map(Vec::as_slice)
        .unwrap_or_else(|| panic!("{ctx} should be a YAML sequence"))
}

fn mapping_get<'a>(map: &'a Mapping, key: &str, ctx: &str) -> &'a Value {
    map.iter()
        .find_map(|(k, v)| (k.as_str() == Some(key)).then_some(v))
        .unwrap_or_else(|| panic!("{ctx} should contain key `{key}`"))
}

fn mapping_get_str<'a>(map: &'a Mapping, key: &str, ctx: &str) -> &'a str {
    mapping_get(map, key, ctx)
        .as_str()
        .unwrap_or_else(|| panic!("{ctx}.{key} should be a YAML string"))
}

fn step_name(step: &Value) -> Option<&str> {
    step.as_mapping()
        .and_then(|m| {
            m.iter()
                .find_map(|(k, v)| (k.as_str() == Some("name")).then_some(v))
        })
        .and_then(Value::as_str)
}

fn find_step<'a>(steps: &'a [Value], name: &str) -> (usize, &'a Mapping) {
    for (idx, step) in steps.iter().enumerate() {
        if step_name(step) == Some(name) {
            return (idx, as_mapping(step, "workflow step"));
        }
    }
    panic!("workflow should include step `{name}`");
}

#[test]
fn ci_workflow_enforces_validation_and_tests() {
    let ci_path = repo_root().join(".github").join("workflows").join("ci.yml");
    let contents = fs::read_to_string(&ci_path).expect("read ci workflow");
    let workflow: Value = serde_yaml::from_str(&contents).expect("parse ci workflow yaml");

    let root = as_mapping(&workflow, "workflow root");
    let jobs = as_mapping(mapping_get(root, "jobs", "workflow root"), "workflow jobs");

    let checks_job = as_mapping(mapping_get(jobs, "checks", "workflow jobs"), "checks job");
    assert_eq!(
        mapping_get_str(checks_job, "runs-on", "checks job"),
        "ubuntu-latest",
        "`checks` job should run on ubuntu-latest"
    );
    let checks_steps = as_sequence(
        mapping_get(checks_job, "steps", "checks job"),
        "checks steps",
    );

    let (_, format_step) = find_step(checks_steps, "Format");
    let format_run = mapping_get_str(format_step, "run", "Format step");
    assert!(
        format_run.contains("cargo fmt --all -- --check"),
        "Format step should enforce cargo fmt --check"
    );

    let (_, clippy_step) = find_step(checks_steps, "Clippy");
    let clippy_run = mapping_get_str(clippy_step, "run", "Clippy step");
    assert!(
        clippy_run.contains("cargo clippy --workspace --all-targets -- -D warnings"),
        "Clippy step should enforce -D warnings"
    );

    let (_, test_step) = find_step(checks_steps, "Test");
    let test_run = mapping_get_str(test_step, "run", "Test step");
    assert!(
        test_run.contains("cargo test --workspace"),
        "Test step should execute workspace tests"
    );

    let (_, proof_step) = find_step(checks_steps, "Proof regression gates");
    let proof_run = mapping_get_str(proof_step, "run", "Proof regression gates step");
    for cmd in [
        "cargo test -p clg-cli --test vc_snapshots",
        "cargo test -p clg-cli --test proof_coverage_matrix",
        "cargo test -p clg-cli --test profile_regression_gate",
        "cargo test -p clg-cli --test cli_it vc_outputs::build_emits_assumption_boundaries_in_vc_json_and_proof_section",
        "cargo test -p clg-typer --test vc declares_bitwise_and_builtin_helpers",
    ] {
        assert!(
            proof_run.contains(cmd),
            "Proof regression gates step should include `{cmd}`"
        );
    }

    let (_, perf_step) = find_step(checks_steps, "Milestone 2 performance budgets");
    let perf_run = mapping_get_str(perf_step, "run", "Milestone 2 performance budgets step");
    assert!(
        perf_run.contains("bash scripts/ci/milestone2_perf_gate.sh"),
        "Performance gate step should execute milestone2 perf script"
    );

    let (_, supply_step) = find_step(checks_steps, "Milestone 2 supply-chain compliance gate");
    let supply_run = mapping_get_str(
        supply_step,
        "run",
        "Milestone 2 supply-chain compliance gate step",
    );
    assert!(
        supply_run.contains("python scripts/ci/milestone2_supply_chain_gate.py"),
        "Supply-chain gate step should execute python gate script"
    );

    let (_, script_tests_step) = find_step(checks_steps, "Script gate self-tests");
    let script_tests_run = mapping_get_str(script_tests_step, "run", "Script gate self-tests step");
    assert!(
        script_tests_run.contains("python scripts/ci/test_milestone2_supply_chain_gate.py"),
        "Script self-tests step should execute supply-chain gate unit tests"
    );
    assert!(
        script_tests_run.contains("bash scripts/ci/milestone2_perf_gate.sh --self-test"),
        "Script self-tests step should execute perf-gate self-test mode"
    );

    let (std_core_idx, _) = find_step(checks_steps, "Std-core artifact reproducibility");
    let (supply_idx, _) = find_step(checks_steps, "Milestone 2 supply-chain compliance gate");
    assert!(
        supply_idx > std_core_idx,
        "supply-chain compliance gate should run after std-core artifact generation"
    );

    let (_, upload_perf_step) = find_step(checks_steps, "Upload milestone_2 performance artifact");
    let upload_perf_with = as_mapping(
        mapping_get(
            upload_perf_step,
            "with",
            "Upload milestone_2 performance artifact",
        ),
        "Upload milestone_2 performance artifact.with",
    );
    assert_eq!(
        mapping_get_str(
            upload_perf_with,
            "name",
            "Upload milestone_2 performance artifact.with"
        ),
        "milestone2-performance",
        "performance artifact upload should use canonical artifact name"
    );

    let (_, upload_supply_step) =
        find_step(checks_steps, "Upload milestone_2 supply-chain artifact");
    let upload_supply_with = as_mapping(
        mapping_get(
            upload_supply_step,
            "with",
            "Upload milestone_2 supply-chain artifact",
        ),
        "Upload milestone_2 supply-chain artifact.with",
    );
    assert_eq!(
        mapping_get_str(
            upload_supply_with,
            "name",
            "Upload milestone_2 supply-chain artifact.with"
        ),
        "milestone2-supply-chain",
        "supply-chain artifact upload should use canonical artifact name"
    );

    let perf_smoke_job = as_mapping(
        mapping_get(jobs, "milestone2-perf-portability-smoke", "workflow jobs"),
        "milestone2-perf-portability-smoke job",
    );
    assert_eq!(
        mapping_get_str(
            perf_smoke_job,
            "runs-on",
            "milestone2-perf-portability-smoke job"
        ),
        "windows-latest",
        "perf portability smoke should run on windows-latest"
    );
    let perf_smoke_steps = as_sequence(
        mapping_get(
            perf_smoke_job,
            "steps",
            "milestone2-perf-portability-smoke job",
        ),
        "milestone2-perf-portability-smoke steps",
    );
    let (_, perf_smoke_step) =
        find_step(perf_smoke_steps, "Perf gate portability smoke (Git Bash)");
    let perf_smoke_run = mapping_get_str(
        perf_smoke_step,
        "run",
        "Perf gate portability smoke (Git Bash) step",
    );
    assert!(
        perf_smoke_run.contains("bash scripts/ci/milestone2_perf_gate.sh --portability-smoke"),
        "perf portability smoke step should execute portability smoke mode"
    );

    let release_train_job = as_mapping(
        mapping_get(jobs, "milestone2-release-train-gate", "workflow jobs"),
        "milestone2-release-train-gate job",
    );
    let release_if = mapping_get_str(release_train_job, "if", "milestone2-release-train-gate job");
    assert!(
        release_if.contains("startsWith(github.ref, 'refs/tags/milestone_2')"),
        "release-train gate job should target milestone_2 tags only"
    );
    let release_steps = as_sequence(
        mapping_get(
            release_train_job,
            "steps",
            "milestone2-release-train-gate job",
        ),
        "milestone2-release-train-gate steps",
    );
    let (_, release_gate_step) = find_step(
        release_steps,
        "Enforce milestone_2 release-train checklist gate",
    );
    let release_gate_run = mapping_get_str(
        release_gate_step,
        "run",
        "Enforce milestone_2 release-train checklist gate step",
    );
    assert!(
        release_gate_run.contains("cargo test -p clg-cli --test milestone2_release_train_gate"),
        "release-train gate step should execute checklist gate test"
    );
    let release_gate_env = as_mapping(
        mapping_get(
            release_gate_step,
            "env",
            "Enforce milestone_2 release-train checklist gate step",
        ),
        "Enforce milestone_2 release-train checklist gate step.env",
    );
    assert_eq!(
        mapping_get_str(
            release_gate_env,
            "CLG_ENFORCE_MILESTONE2_RELEASE_GATE",
            "release-train gate env",
        ),
        "1",
        "release-train gate should explicitly enable checklist enforcement"
    );
}
