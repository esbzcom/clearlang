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
fn solver_support_matrix_lock_pins_release_targets() {
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
        vec!["windows-x64", "linux-x64-glibc-2.39", "macos-x64-15.7.3"],
        "phase 25.1.15 matrix should pin the current release-target bundle set"
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
fn solver_support_matrix_release_targets_have_staged_bundles() {
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

    let release_targets = lock["release_target_platforms"]
        .as_array()
        .expect("release_target_platforms[]");
    for target in release_targets {
        let target = target.as_str().expect("release target should be a string");
        let bundle_path = match target {
            "windows-x64" => root
                .join("tools")
                .join("proof")
                .join("z3")
                .join("windows")
                .join("z3.exe"),
            "linux-x64-glibc-2.39" => root
                .join("tools")
                .join("proof")
                .join("z3")
                .join("linux")
                .join("z3"),
            "macos-x64-15.7.3" => root
                .join("tools")
                .join("proof")
                .join("z3")
                .join("macos")
                .join("z3"),
            other => panic!("unexpected release target `{other}`"),
        };
        assert!(
            bundle_path.is_file(),
            "release-target solver bundle should exist for `{target}`: `{}`",
            bundle_path.display()
        );
        let checksum_path = bundle_path.parent().expect("bundle parent").join(format!(
            "{}.sha256",
            bundle_path
                .file_name()
                .expect("bundle filename")
                .to_string_lossy()
        ));
        let signature_path = bundle_path.parent().expect("bundle parent").join(format!(
            "{}.sig",
            bundle_path
                .file_name()
                .expect("bundle filename")
                .to_string_lossy()
        ));
        assert!(
            checksum_path.is_file(),
            "release-target solver checksum should exist for `{target}`: `{}`",
            checksum_path.display()
        );
        assert!(
            signature_path.is_file(),
            "release-target solver signature should exist for `{target}`: `{}`",
            signature_path.display()
        );
    }
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
        "${{ matrix.os }}",
        "solver support-matrix CI job should run on matrix.os"
    );
    let strategy = as_mapping(
        mapping_get(parity_job, "strategy", expected_job),
        "solver support-matrix strategy",
    );
    let matrix = as_mapping(
        mapping_get(strategy, "matrix", "solver support-matrix strategy"),
        "solver support-matrix matrix",
    );
    let include = as_sequence(
        mapping_get(matrix, "include", "solver support-matrix matrix"),
        "solver support-matrix matrix include",
    );
    let mut has_windows = false;
    let mut has_linux = false;
    let mut has_macos = false;
    for entry in include {
        let item = as_mapping(entry, "solver support-matrix include item");
        let os = mapping_get_str(item, "os", "solver support-matrix include item");
        let platform = mapping_get_str(item, "platform", "solver support-matrix include item");
        if os == "windows-latest" && platform == "windows" {
            has_windows = true;
        }
        if os == "ubuntu-latest" && platform == "linux" {
            has_linux = true;
        }
        if os == "macos-latest" && platform == "macos" {
            has_macos = true;
        }
    }
    assert!(
        has_windows && has_linux && has_macos,
        "solver support-matrix CI job should cover windows + linux + macos release targets"
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
