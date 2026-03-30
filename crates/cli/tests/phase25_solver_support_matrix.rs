use serde_json::Value as JsonValue;
use serde_yaml::{Mapping, Value as YamlValue};
use std::fs;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
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

fn find_step<'a>(steps: &'a [YamlValue], name: &str) -> &'a Mapping {
    steps
        .iter()
        .find_map(|step| {
            (step_name(step) == Some(name)).then_some(as_mapping(step, "workflow step"))
        })
        .unwrap_or_else(|| panic!("workflow should include step `{name}`"))
}

#[test]
fn solver_support_matrix_lock_pins_windows_release_target() {
    let root = repo_root();
    let lock_path = root
        .join("docs")
        .join("design")
        .join("phase-25.1.15-solver-support-matrix.lock.json");
    let lock: JsonValue = serde_json::from_slice(
        fs::read(lock_path)
            .expect("read solver support matrix lock")
            .as_slice(),
    )
    .expect("parse solver support matrix lock");

    assert_eq!(lock["schema_version"], JsonValue::from(1));
    assert_eq!(
        lock["policy_version"],
        JsonValue::String("25.1.15".to_string())
    );

    let release_targets = lock["release_target_platforms"]
        .as_array()
        .expect("release_target_platforms[]");
    let release_targets: Vec<&str> = release_targets
        .iter()
        .map(|entry| entry.as_str().expect("release target should be a string"))
        .collect();
    assert_eq!(
        release_targets,
        vec!["windows-x64"],
        "phase 25.1.15 matrix should pin windows-x64 as the current release target"
    );

    assert_eq!(
        lock["unsupported_target_policy"]["release_mode"],
        JsonValue::String("fail_closed".to_string())
    );
    assert_eq!(
        lock["unsupported_target_policy"]["diagnostic_code"],
        JsonValue::String("C124".to_string())
    );
}

#[test]
fn solver_support_matrix_ci_strategy_matches_workflow() {
    let root = repo_root();
    let lock_path = root
        .join("docs")
        .join("design")
        .join("phase-25.1.15-solver-support-matrix.lock.json");
    let lock: JsonValue = serde_json::from_slice(
        fs::read(lock_path)
            .expect("read solver support matrix lock")
            .as_slice(),
    )
    .expect("parse solver support matrix lock");

    let expected_workflow = lock["ci_validation_strategy"]["workflow"]
        .as_str()
        .expect("ci_validation_strategy.workflow");
    let expected_job = lock["ci_validation_strategy"]["required_job"]
        .as_str()
        .expect("ci_validation_strategy.required_job");
    let expected_step = lock["ci_validation_strategy"]["required_step"]
        .as_str()
        .expect("ci_validation_strategy.required_step");
    let expected_command = lock["ci_validation_strategy"]["required_command"]
        .as_str()
        .expect("ci_validation_strategy.required_command");

    let workflow_path = root.join(Path::new(expected_workflow));
    let workflow_raw = fs::read_to_string(workflow_path).expect("read ci workflow");
    let workflow: YamlValue = serde_yaml::from_str(&workflow_raw).expect("parse ci workflow");
    let workflow_root = as_mapping(&workflow, "workflow root");
    let jobs = as_mapping(
        mapping_get(workflow_root, "jobs", "workflow root"),
        "workflow jobs",
    );
    let parity_job = as_mapping(
        mapping_get(jobs, expected_job, "workflow jobs"),
        expected_job,
    );
    assert_eq!(
        mapping_get_str(parity_job, "runs-on", expected_job),
        "windows-latest",
        "solver support-matrix CI job should run on windows-latest"
    );
    let steps = as_sequence(
        mapping_get(parity_job, "steps", expected_job),
        "parity steps",
    );
    let step = find_step(steps, expected_step);
    let run = mapping_get_str(step, "run", expected_step);
    assert!(
        run.contains(expected_command),
        "solver support-matrix CI step should run required command from lock"
    );
}
