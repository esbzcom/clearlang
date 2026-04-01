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

fn as_string_sequence<'a>(value: &'a Value, ctx: &str) -> Vec<&'a str> {
    as_sequence(value, ctx)
        .iter()
        .map(|item| {
            item.as_str()
                .unwrap_or_else(|| panic!("{ctx} should contain strings"))
        })
        .collect()
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

    let (_, precheck_step) = find_step(checks_steps, "Release precheck gate (fmt + lint + tests)");
    let precheck_run = mapping_get_str(
        precheck_step,
        "run",
        "Release precheck gate (fmt + lint + tests) step",
    );
    assert!(
        precheck_run.contains("cargo run -p xtask -- release-precheck"),
        "release precheck step should execute fail-closed xtask precheck gate"
    );

    let (_, proof_step) = find_step(checks_steps, "Proof regression gates");
    let proof_run = mapping_get_str(proof_step, "run", "Proof regression gates step");
    for cmd in [
        "cargo test -p clg-cli --test vc_snapshots",
        "cargo test -p clg-cli --test solver_outcomes",
        "cargo test -p clg-cli --test solver_replay_stability",
        "cargo test -p clg-cli --test phase25_solver_support_matrix",
        "cargo test -p clg-cli --test phase25_solver_supply_chain_gate",
        "cargo test -p clg-cli --test phase25_crypto_release_closure",
        "cargo test -p clg-cli --test phase25_production_cutover_policy",
        "cargo test -p clg-cli --test phase25_gate_b_closure_metric",
        "cargo test -p clg-cli --test proof_artifact_replay",
        "cargo test -p clg-cli --test proof_coverage_matrix",
        "cargo test -p clg-cli --test verified_std_core_subset",
        "cargo test -p clg-cli --test profile_regression_gate",
        "cargo test -p clg-cli --test milestone3_release_gate",
        "cargo test -p clg-cli --test milestone3_proof_parity",
        "cargo test -p clg-cli --test signing verify_require_assurance_rejects_manifest_with_assumption_boundaries_when_proved_all",
        "cargo test -p clg-cli --test signing verify_require_assurance_rejects_symbols_outside_proved_allowlist_when_proved_all",
        "cargo test -p clg-cli --test cli_it diagnostics::production_release_profile_requires_proved_all_with_c121",
        "cargo test -p clg-cli --test solver_outcomes missing_solver_binary_keeps_generated_status",
        "cargo test -p clg-cli --test cli_it diagnostics::production_release_profile_rejects_non_proved_std_surface_with_c122",
        "cargo test -p clg-cli --test cli_it diagnostics::production_release_profile_rejects_crypto_assumption_boundary_with_c123",
        "cargo test -p clg-cli --test cli_it parse_rejects_theorem_keyword_with_explicit_code",
        "cargo test -p clg-cli --test cli_it vc_outputs::build_emits_assumption_boundaries_in_vc_json_and_proof_section",
        "cargo test -p clg-typer --test vc declares_bitwise_and_builtin_helpers",
        "cargo test -p clg-typer --test vc vc_smt_declares_parameters_with_sorts_and_closes_let_locals",
    ] {
        assert!(
            proof_run.contains(cmd),
            "Proof regression gates step should include `{cmd}`"
        );
    }

    let (_, perf_step) = find_step(checks_steps, "Milestone 2 performance budgets");
    let perf_run = mapping_get_str(perf_step, "run", "Milestone 2 performance budgets step");
    assert!(
        perf_run.contains("cargo run -p xtask -- milestone2-perf-gate"),
        "Performance gate step should execute xtask milestone2 perf gate"
    );

    let (_, supply_step) = find_step(checks_steps, "Milestone 2 supply-chain compliance gate");
    let supply_run = mapping_get_str(
        supply_step,
        "run",
        "Milestone 2 supply-chain compliance gate step",
    );
    assert!(
        supply_run.contains("cargo run -p xtask -- milestone2-supply-chain-gate"),
        "Supply-chain gate step should execute xtask supply-chain gate"
    );

    let (_, script_tests_step) = find_step(checks_steps, "Gate self-tests");
    let script_tests_run = mapping_get_str(script_tests_step, "run", "Gate self-tests step");
    assert!(
        script_tests_run.contains("cargo run -p xtask -- milestone2-supply-chain-gate --self-test"),
        "gate self-tests step should execute supply-chain gate self-test mode"
    );
    assert!(
        script_tests_run.contains("cargo run -p xtask -- milestone2-perf-gate --self-test"),
        "gate self-tests step should execute perf-gate self-test mode"
    );

    let (perf_idx, _) = find_step(checks_steps, "Milestone 2 performance budgets");
    let (std_core_idx, _) = find_step(checks_steps, "Std-core artifact reproducibility");
    let (supply_idx, _) = find_step(checks_steps, "Milestone 2 supply-chain compliance gate");
    assert!(
        supply_idx > perf_idx,
        "supply-chain compliance gate should run after perf gate so runtime-link artifacts are available"
    );
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
        perf_smoke_run.contains("cargo run -p xtask -- milestone2-perf-gate --portability-smoke"),
        "perf portability smoke step should execute portability smoke mode"
    );
    let (_, perf_windows_build_step) = find_step(perf_smoke_steps, "Release build");
    let perf_windows_build_run = mapping_get_str(
        perf_windows_build_step,
        "run",
        "windows perf Release build step",
    );
    assert!(
        perf_windows_build_run.contains("cargo build --release -p clg-cli"),
        "windows perf job should build release clg before full gate execution"
    );
    let (_, perf_self_test_step) = find_step(perf_smoke_steps, "Perf gate self-test (Git Bash)");
    let perf_self_test_run = mapping_get_str(
        perf_self_test_step,
        "run",
        "Perf gate self-test (Git Bash) step",
    );
    assert!(
        perf_self_test_run.contains("cargo run -p xtask -- milestone2-perf-gate --self-test"),
        "windows perf job should run perf gate self-test mode"
    );
    let (_, perf_full_step) = find_step(perf_smoke_steps, "Perf gate full run (Git Bash)");
    let perf_full_run =
        mapping_get_str(perf_full_step, "run", "Perf gate full run (Git Bash) step");
    assert!(
        perf_full_run.contains("$env:PERF_SAMPLE_RUNS = \"1\""),
        "windows perf full run should pin PERF_SAMPLE_RUNS=1 for bounded runtime"
    );
    assert!(
        perf_full_run.contains("cargo run -p xtask -- milestone2-perf-gate"),
        "windows perf job should execute full perf gate path"
    );

    let release_train_job = as_mapping(
        mapping_get(jobs, "milestone2-release-train-gate", "workflow jobs"),
        "milestone2-release-train-gate job",
    );
    let release_needs = as_string_sequence(
        mapping_get(
            release_train_job,
            "needs",
            "milestone2-release-train-gate job",
        ),
        "milestone2-release-train-gate.needs",
    );
    for required in [
        "checks",
        "milestone2-perf-portability-smoke",
        "strict-determinism-replay",
        "resolver-determinism-replay",
        "runtime-loader-determinism-replay",
        "milestone2-regression-gate",
    ] {
        assert!(
            release_needs.contains(&required),
            "release-train gate should depend on `{required}` to enforce all milestone_2 gates before tagging"
        );
    }
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

    let parity_job = as_mapping(
        mapping_get(jobs, "milestone3-proof-parity", "workflow jobs"),
        "milestone3-proof-parity job",
    );
    assert_eq!(
        mapping_get_str(parity_job, "runs-on", "milestone3-proof-parity job"),
        "windows-latest",
        "milestone3 proof parity job should run on windows-latest"
    );
    let parity_steps = as_sequence(
        mapping_get(parity_job, "steps", "milestone3-proof-parity job"),
        "milestone3-proof-parity steps",
    );
    let (_, solver_vendor_smoke_step) = find_step(parity_steps, "Solver vendor-path smoke gate");
    let solver_vendor_smoke_run = mapping_get_str(
        solver_vendor_smoke_step,
        "run",
        "Solver vendor-path smoke gate step",
    );
    assert!(
        solver_vendor_smoke_run.contains(
            "cargo test -p clg-cli --test solver_outcomes bundled_solver_root_is_used_when_solver_env_not_set"
        ),
        "milestone3 proof parity job should validate bundled solver fallback behavior"
    );
    let (_, parity_snapshot_step) = find_step(parity_steps, "Milestone 3 proof parity snapshot");
    let parity_snapshot_run = mapping_get_str(
        parity_snapshot_step,
        "run",
        "Milestone 3 proof parity snapshot step",
    );
    assert!(
        parity_snapshot_run.contains("cargo test -p clg-cli --test milestone3_proof_parity"),
        "milestone3 proof parity snapshot should run milestone3_proof_parity test"
    );
    let parity_snapshot_env = as_mapping(
        mapping_get(
            parity_snapshot_step,
            "env",
            "Milestone 3 proof parity snapshot step",
        ),
        "Milestone 3 proof parity snapshot step.env",
    );
    assert!(
        mapping_get_str(
            parity_snapshot_env,
            "CLG_PROOF_PARITY_OUT",
            "Milestone 3 proof parity snapshot step.env"
        )
        .contains("tmp/proof-parity/windows.json"),
        "milestone3 proof parity snapshot should emit deterministic windows artifact path"
    );

    let milestone3_release_train_job = as_mapping(
        mapping_get(jobs, "milestone3-release-train-gate", "workflow jobs"),
        "milestone3-release-train-gate job",
    );
    let milestone3_needs = as_string_sequence(
        mapping_get(
            milestone3_release_train_job,
            "needs",
            "milestone3-release-train-gate job",
        ),
        "milestone3-release-train-gate.needs",
    );
    assert!(
        milestone3_needs.contains(&"checks")
            && milestone3_needs.contains(&"milestone3-proof-parity"),
        "milestone3 release-train gate should require checks + windows parity job"
    );
}
