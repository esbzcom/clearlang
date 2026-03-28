use serde_json::Value as JsonValue;
use serde_yaml::{Mapping, Value as YamlValue};
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
}

fn validate_ci_wiring(root: &Path) {
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
    for cmd in [
        "cargo test -p clg-cli --test vc_snapshots",
        "cargo test -p clg-cli --test proof_coverage_matrix",
        "cargo test -p clg-cli --test verified_std_core_subset",
        "cargo test -p clg-cli --test profile_regression_gate",
        "cargo test -p clg-cli --test milestone3_release_gate",
    ] {
        assert!(
            proof_run.contains(cmd),
            "Proof regression gates step should include `{cmd}`"
        );
    }

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
        needs.contains(&"checks"),
        "milestone_3 release gate must depend on `checks`"
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
    assert!(
        todo_has_checked_item(&todo, "25.0.8"),
        "milestone_3 release gate requires TODO item `25.0.8` to be marked complete before tagging"
    );
}
