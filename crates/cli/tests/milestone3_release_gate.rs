use serde_json::Value as JsonValue;
use serde_yaml::{Mapping, Value as YamlValue};
use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

fn enforce_gate() -> bool {
    env::var("CLG_ENFORCE_MILESTONE3_RELEASE_GATE")
        .ok()
        .as_deref()
        == Some("1")
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

fn as_mapping<'a>(value: &'a YamlValue, ctx: &str) -> &'a Mapping {
    value
        .as_mapping()
        .unwrap_or_else(|| panic!("{ctx} should be a YAML mapping"))
}

fn as_sequence<'a>(value: &'a YamlValue, ctx: &str) -> &'a [YamlValue] {
    value
        .as_sequence()
        .map(Vec::as_slice)
        .unwrap_or_else(|| panic!("{ctx} should be a YAML sequence"))
}

fn as_string_sequence<'a>(value: &'a YamlValue, ctx: &str) -> Vec<&'a str> {
    as_sequence(value, ctx)
        .iter()
        .map(|item| {
            item.as_str()
                .unwrap_or_else(|| panic!("{ctx} should contain strings"))
        })
        .collect()
}

fn mapping_get<'a>(map: &'a Mapping, key: &str, ctx: &str) -> &'a YamlValue {
    map.iter()
        .find_map(|(k, v)| (k.as_str() == Some(key)).then_some(v))
        .unwrap_or_else(|| panic!("{ctx} should contain key `{key}`"))
}

fn mapping_get_str<'a>(map: &'a Mapping, key: &str, ctx: &str) -> &'a str {
    mapping_get(map, key, ctx)
        .as_str()
        .unwrap_or_else(|| panic!("{ctx}.{key} should be a YAML string"))
}

fn step_name(step: &YamlValue) -> Option<&str> {
    step.as_mapping()
        .and_then(|m| {
            m.iter()
                .find_map(|(k, v)| (k.as_str() == Some("name")).then_some(v))
        })
        .and_then(YamlValue::as_str)
}

fn find_step<'a>(steps: &'a [YamlValue], name: &str) -> (usize, &'a Mapping) {
    for (idx, step) in steps.iter().enumerate() {
        if step_name(step) == Some(name) {
            return (idx, as_mapping(step, "workflow step"));
        }
    }
    panic!("workflow should include step `{name}`");
}

fn validate_lock_artifacts_and_docs(root: &Path) {
    let lock_path = root
        .join("docs")
        .join("evidence")
        .join("milestone_3-proof-gate.lock.json");
    let lock: JsonValue =
        serde_json::from_slice(&fs::read(&lock_path).expect("read milestone_3 gate lock"))
            .expect("parse milestone_3 gate lock");
    assert_eq!(
        lock.get("schema_version").and_then(|v| v.as_u64()),
        Some(1),
        "milestone_3 proof gate lock must use schema_version=1"
    );

    let required_artifacts = lock
        .get("required_artifacts")
        .and_then(|v| v.as_array())
        .expect("milestone_3 lock required_artifacts[]");
    assert!(
        !required_artifacts.is_empty(),
        "milestone_3 proof gate lock must include required_artifacts"
    );
    for artifact in required_artifacts {
        let path = artifact
            .as_str()
            .expect("required artifact path should be a string");
        assert!(
            root.join(path).exists(),
            "milestone_3 proof gate requires artifact path `{path}` to exist"
        );
    }

    let required_ci_tests = lock
        .get("required_ci_tests")
        .and_then(|v| v.as_array())
        .expect("milestone_3 lock required_ci_tests[]");
    assert!(
        !required_ci_tests.is_empty(),
        "milestone_3 proof gate lock must include required_ci_tests"
    );

    let prohibited_boundaries = lock
        .get("prohibited_release_assumption_boundaries")
        .and_then(|v| v.as_array())
        .expect("milestone_3 lock prohibited_release_assumption_boundaries[]");
    let actual_boundaries: Vec<&str> = prohibited_boundaries
        .iter()
        .map(|entry| {
            entry
                .as_str()
                .expect("prohibited assumption boundary should be a string")
        })
        .collect();
    assert_eq!(
        actual_boundaries,
        vec![
            "unsigned.int_model",
            "bitwise.uninterpreted",
            "crypto.uninterpreted"
        ],
        "milestone_3 lock must pin prohibited assumption boundaries in deterministic order"
    );
    let parity_targets = lock
        .get("cross_platform_proof_parity_targets")
        .and_then(|v| v.as_array())
        .expect("milestone_3 lock cross_platform_proof_parity_targets[]");
    let parity_target_ids: Vec<&str> = parity_targets
        .iter()
        .map(|entry| {
            entry
                .as_str()
                .expect("cross-platform parity target should be a string")
        })
        .collect();
    assert_eq!(
        parity_target_ids,
        vec!["linux", "windows", "macos"],
        "milestone_3 lock must pin cross-platform parity targets in deterministic order"
    );
    let release_symbol_allowlist = lock
        .get("release_symbol_allowlist")
        .and_then(|v| v.as_object())
        .expect("milestone_3 lock release_symbol_allowlist{}");
    assert_eq!(
        release_symbol_allowlist
            .get("source")
            .and_then(|v| v.as_str()),
        Some("docs/proofs/proof-coverage-matrix.json"),
        "milestone_3 lock must pin release symbol allowlist source"
    );
    assert_eq!(
        release_symbol_allowlist
            .get("rule")
            .and_then(|v| v.as_str()),
        Some("bundle_symbols must be subset of entries[].surface where status==proved"),
        "milestone_3 lock must pin release symbol allowlist rule"
    );
    let production_release_surface_policy = lock
        .get("production_release_surface_policy")
        .and_then(|v| v.as_object())
        .expect("milestone_3 lock production_release_surface_policy{}");
    assert_eq!(
        production_release_surface_policy
            .get("enforced_by")
            .and_then(|v| v.as_str()),
        Some("C122"),
        "milestone_3 lock must pin production release surface policy diagnostic"
    );
    assert_eq!(
        production_release_surface_policy
            .get("rule")
            .and_then(|v| v.as_str()),
        Some(
            "release-profile production may use only std surfaces marked proved in proof-coverage-matrix"
        ),
        "milestone_3 lock must pin production release surface policy rule"
    );
    let production_crypto_boundary_policy = lock
        .get("production_crypto_boundary_policy")
        .and_then(|v| v.as_object())
        .expect("milestone_3 lock production_crypto_boundary_policy{}");
    assert_eq!(
        production_crypto_boundary_policy
            .get("enforced_by")
            .and_then(|v| v.as_str()),
        Some("C123"),
        "milestone_3 lock must pin crypto release boundary diagnostic"
    );
    assert_eq!(
        production_crypto_boundary_policy
            .get("blocked_boundary")
            .and_then(|v| v.as_str()),
        Some("crypto.uninterpreted"),
        "milestone_3 lock must pin blocked crypto boundary id"
    );
    assert_eq!(
        production_crypto_boundary_policy
            .get("rule")
            .and_then(|v| v.as_str()),
        Some(
            "release-profile production fails closed when unresolved crypto proof boundary remains"
        ),
        "milestone_3 lock must pin crypto release boundary rule"
    );

    let required_release_commands = lock
        .get("required_release_commands")
        .and_then(|v| v.as_array())
        .expect("milestone_3 lock required_release_commands[]");
    let release_process = fs::read_to_string(root.join("docs").join("release-process.md"))
        .expect("read docs/release-process.md");
    for fragment in required_release_commands {
        let fragment = fragment
            .as_str()
            .expect("required release command fragment should be a string");
        assert!(
            release_process.contains(fragment),
            "release process should include required command fragment `{fragment}`"
        );
    }

    let evidence_doc_path = root
        .join("docs")
        .join("evidence")
        .join("milestone_3-proof-gate.md");
    let evidence_doc =
        fs::read_to_string(&evidence_doc_path).expect("read milestone_3 proof gate evidence doc");
    for section in [
        "## Required Proof Matrix Artifacts",
        "## CI and Release-Train Gate",
        "## Release Workflow Proof Requirements",
        "## Sign-off Snapshot",
    ] {
        assert!(
            evidence_doc.contains(section),
            "milestone_3 proof gate evidence doc should include section `{section}`"
        );
    }
    let root_readme = fs::read_to_string(root.join("README.md")).expect("read root README.md");
    assert!(
        root_readme.contains(
            "`--compiler-mode permissive` and `--compiler-mode standard` are dev/evidence workflows"
        ),
        "root README must clarify permissive/standard profiles are dev/evidence workflows"
    );
    assert!(
        root_readme.contains("do **not** imply theorem-grade status (`proved_all`)"),
        "root README must avoid implying standard compile is theorem-grade"
    );
    assert!(
        root_readme.contains("no `theorem` language keyword"),
        "root README must state theorem-grade is certification status, not milestone_3 syntax"
    );
    let examples_readme =
        fs::read_to_string(root.join("examples").join("projects").join("README.md"))
            .expect("read examples/projects/README.md");
    assert!(
        examples_readme.contains("--release-profile production"),
        "examples quick-release docs must include production release profile command"
    );
    assert!(
        examples_readme.contains("--require-assurance proved_all"),
        "examples quick-release docs must include theorem-grade verify gate command"
    );
}

fn validate_ci_wiring(root: &Path) {
    let lock_path = root
        .join("docs")
        .join("evidence")
        .join("milestone_3-proof-gate.lock.json");
    let lock: JsonValue =
        serde_json::from_slice(&fs::read(&lock_path).expect("read milestone_3 gate lock"))
            .expect("parse milestone_3 gate lock");
    let required_ci_tests = lock
        .get("required_ci_tests")
        .and_then(|v| v.as_array())
        .expect("milestone_3 lock required_ci_tests[]");

    let ci_path = root.join(".github").join("workflows").join("ci.yml");
    let contents = fs::read_to_string(&ci_path).expect("read ci workflow");
    let workflow: YamlValue = serde_yaml::from_str(&contents).expect("parse ci workflow yaml");
    let root_map = as_mapping(&workflow, "workflow root");
    let jobs = as_mapping(
        mapping_get(root_map, "jobs", "workflow root"),
        "workflow jobs",
    );

    let checks = as_mapping(mapping_get(jobs, "checks", "workflow jobs"), "checks job");
    let check_steps = as_sequence(mapping_get(checks, "steps", "checks job"), "checks steps");
    let (_, proof_step) = find_step(check_steps, "Proof regression gates");
    let proof_run = mapping_get_str(proof_step, "run", "Proof regression gates step");
    for cmd in required_ci_tests {
        let cmd = cmd.as_str().expect("required_ci_tests[] should be strings");
        assert!(
            proof_run.contains(cmd),
            "Proof regression gates step should include `{cmd}`"
        );
    }

    let parity_job = as_mapping(
        mapping_get(jobs, "milestone3-proof-parity", "workflow jobs"),
        "milestone3-proof-parity job",
    );
    assert_eq!(
        mapping_get_str(parity_job, "runs-on", "milestone3-proof-parity job"),
        "${{ matrix.os }}",
        "milestone3 proof parity job should run on matrix targets"
    );
    let parity_strategy = as_mapping(
        mapping_get(parity_job, "strategy", "milestone3-proof-parity job"),
        "milestone3-proof-parity strategy",
    );
    let parity_matrix = as_mapping(
        mapping_get(
            parity_strategy,
            "matrix",
            "milestone3-proof-parity strategy",
        ),
        "milestone3-proof-parity matrix",
    );
    let parity_include = as_sequence(
        mapping_get(parity_matrix, "include", "milestone3-proof-parity matrix"),
        "milestone3-proof-parity include[]",
    );
    let mut parity_ids = BTreeSet::new();
    for entry in parity_include {
        let entry_map = as_mapping(entry, "milestone3-proof-parity include entry");
        parity_ids.insert(
            mapping_get_str(entry_map, "id", "milestone3-proof-parity include entry").to_string(),
        );
    }
    assert_eq!(
        parity_ids,
        BTreeSet::from([
            "linux".to_string(),
            "windows".to_string(),
            "macos".to_string()
        ]),
        "milestone3 proof parity matrix should pin linux/windows/macos ids"
    );

    let parity_compare_job = as_mapping(
        mapping_get(jobs, "milestone3-proof-parity-compare", "workflow jobs"),
        "milestone3-proof-parity-compare job",
    );
    let parity_compare_needs = as_string_sequence(
        mapping_get(
            parity_compare_job,
            "needs",
            "milestone3-proof-parity-compare job",
        ),
        "milestone3-proof-parity-compare.needs",
    );
    assert!(
        parity_compare_needs.contains(&"milestone3-proof-parity"),
        "milestone3 proof parity compare job should depend on milestone3-proof-parity matrix job"
    );
    let parity_compare_steps = as_sequence(
        mapping_get(
            parity_compare_job,
            "steps",
            "milestone3-proof-parity-compare job",
        ),
        "milestone3-proof-parity-compare steps",
    );
    let (_, parity_compare_step) = find_step(
        parity_compare_steps,
        "Compare cross-platform proof parity artifacts",
    );
    let parity_compare_run = mapping_get_str(
        parity_compare_step,
        "run",
        "Compare cross-platform proof parity artifacts step",
    );
    assert!(
        parity_compare_run.contains(
            "cmp --silent tmp/proof-parity/linux/linux.json tmp/proof-parity/windows/windows.json"
        ),
        "milestone3 proof parity compare should compare linux and windows artifacts"
    );
    assert!(
        parity_compare_run.contains(
            "cmp --silent tmp/proof-parity/linux/linux.json tmp/proof-parity/macos/macos.json"
        ),
        "milestone3 proof parity compare should compare linux and macos artifacts"
    );

    let release_job = as_mapping(
        mapping_get(jobs, "milestone3-release-train-gate", "workflow jobs"),
        "milestone3-release-train-gate job",
    );
    let release_if = mapping_get_str(release_job, "if", "milestone3-release-train-gate job");
    assert!(
        release_if.contains("startsWith(github.ref, 'refs/tags/milestone_3')"),
        "milestone_3 release gate must only trigger on milestone_3 tags"
    );
    let needs = as_string_sequence(
        mapping_get(release_job, "needs", "milestone3-release-train-gate job"),
        "milestone3-release-train-gate.needs",
    );
    assert!(
        needs.contains(&"checks") && needs.contains(&"milestone3-proof-parity-compare"),
        "milestone_3 release gate must depend on `checks` and `milestone3-proof-parity-compare`"
    );

    let release_steps = as_sequence(
        mapping_get(release_job, "steps", "milestone3-release-train-gate job"),
        "milestone3-release-train-gate steps",
    );
    let (_, gate_step) = find_step(
        release_steps,
        "Enforce milestone_3 proof/evidence release gate",
    );
    let gate_run = mapping_get_str(
        gate_step,
        "run",
        "Enforce milestone_3 proof/evidence release gate step",
    );
    assert!(
        gate_run.contains("cargo test -p clg-cli --test milestone3_release_gate"),
        "milestone_3 release-train step should run milestone3_release_gate test"
    );
    let gate_env = as_mapping(
        mapping_get(
            gate_step,
            "env",
            "Enforce milestone_3 proof/evidence release gate step",
        ),
        "Enforce milestone_3 proof/evidence release gate step.env",
    );
    assert_eq!(
        mapping_get_str(
            gate_env,
            "CLG_ENFORCE_MILESTONE3_RELEASE_GATE",
            "milestone3 release gate env",
        ),
        "1",
        "milestone_3 release gate must set CLG_ENFORCE_MILESTONE3_RELEASE_GATE=1"
    );
}

#[test]
fn milestone3_release_gate_lock_and_ci_wiring_are_valid_in_all_runs() {
    let root = repo_root();
    validate_lock_artifacts_and_docs(&root);
    validate_ci_wiring(&root);
}

#[test]
fn milestone3_release_gate_requires_todo_completion_when_enforced() {
    if !enforce_gate() {
        return;
    }
    let root = repo_root();
    let todo = fs::read_to_string(root.join("docs").join("TODO.md")).expect("read docs/TODO.md");
    for item in [
        "25.0.8", "25.0.9", "25.0.10", "25.0.11", "25.0.12", "25.0.13", "25.0.14", "25.0.15",
    ] {
        assert!(
            todo_has_checked_item(&todo, item),
            "milestone_3 release gate requires TODO item `{item}` to be marked complete before tagging"
        );
    }
}
