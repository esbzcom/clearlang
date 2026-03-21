use std::fs;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

#[test]
fn ci_workflow_enforces_validation_and_tests() {
    let ci_path = repo_root().join(".github").join("workflows").join("ci.yml");
    let contents = fs::read_to_string(&ci_path).expect("read ci workflow");
    assert!(
        contents.contains("wasm-tools validate"),
        "ci should validate emitted wasm with wasm-tools"
    );
    assert!(
        contents.contains("cargo run -p clg-cli -- run clearlang-tests/16_namespaced_call.clear"),
        "ci should run the namespaced runnable baseline fixture directly"
    );
    assert!(
        contents.contains("cargo test --workspace"),
        "ci should run workspace tests"
    );
    assert!(
        contents.contains("cargo test -p clg-cli --test cli_it sdk_usability_"),
        "ci should enforce sdk usability gate tests"
    );
    assert!(
        contents.contains("cargo test -p clg-cli --test cli_it migration_"),
        "ci should enforce migration-friction regression tests"
    );
    assert!(
        contents.contains("cargo test -p clg-cli --test cli_it imports::"),
        "ci should enforce import ergonomics gate tests"
    );
    assert!(
        contents.contains("cargo test -p clg-cli --test cli_it strict_acceptance_"),
        "ci should enforce strict gate acceptance suite tests"
    );
    assert!(
        contents.contains(
            "cargo test -p clg-cli --test cli_it pkg_lock_replay_with_identical_inputs_is_byte_identical"
        ),
        "ci should enforce resolver/solver lockfile+graph replay determinism gates"
    );
    assert!(
        contents.contains(
            "cargo test -p clg-cli --test cli_it pkg_lock_diagnostics_output_is_deterministic_across_identical_runs"
        ),
        "ci should enforce resolver/solver diagnostics ordering replay determinism gates"
    );
    assert!(
        contents.contains("cargo test -p clg-cli --test run_smoke runtime_loader_tamper_"),
        "ci should enforce runtime-loader tamper failure matrix gates"
    );
    assert!(
        contents.contains("cargo test -p clg-cli --test run_smoke runtime_loader_replay_"),
        "ci should enforce runtime-loader replay determinism gates"
    );
    assert!(
        contents.contains("milestone2-regression-gate"),
        "ci should include an explicit milestone_2 cross-phase regression gate job"
    );
    assert!(
        contents.contains("cargo test -p clg-cli --test phase19_design_principles"),
        "milestone_2 gate should re-check phase 19 design-principle guarantees"
    );
    assert!(
        contents.contains("cargo test -p clg-cli --test host_conformance_certification"),
        "milestone_2 gate should include host-profile conformance certification coverage"
    );
    assert!(
        contents.contains("milestone2-release-train-gate"),
        "ci should include a milestone_2 release-train gate job for tag events"
    );
    assert!(
        contents.contains("CLG_ENFORCE_MILESTONE2_RELEASE_GATE: \"1\""),
        "release-train gate should explicitly enable enforced checklist validation"
    );
    assert!(
        contents.contains("cargo test -p clg-cli --test milestone2_release_train_gate"),
        "release-train gate should run milestone2 checklist enforcement test"
    );
    assert!(
        contents.contains("cargo test -p clg-cli --test vc_snapshots"),
        "ci should enforce proof fixture snapshot regression tests"
    );
    assert!(
        contents.contains("cargo test -p clg-cli --test proof_coverage_matrix"),
        "ci should enforce proof coverage matrix regression tests"
    );
    assert!(
        contents.contains("cargo test -p clg-cli --test profile_regression_gate"),
        "ci should enforce verified-profile assurance regression tests"
    );
    assert!(
        contents.contains(
            "cargo test -p clg-cli --test cli_it vc_outputs::build_emits_assumption_boundaries_in_vc_json_and_proof_section"
        ),
        "ci should enforce assumption-boundary proof artifact regression tests"
    );
    assert!(
        contents.contains("cargo test -p clg-typer --test vc declares_bitwise_and_builtin_helpers"),
        "ci should enforce typer-level bitwise/builtin proof-model regression tests"
    );
    assert!(
        contents.contains("cargo fmt --all -- --check"),
        "ci should enforce formatting"
    );
    assert!(
        contents.contains("cargo clippy --workspace --all-targets -- -D warnings"),
        "ci should enforce clippy warnings"
    );
}
