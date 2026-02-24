use super::*;

#[derive(Clone, Copy)]
struct UsabilityCase {
    file: &'static str,
    command: &'static str,
    expected_code: &'static str,
    expected_stage: &'static str,
}

fn run_json_failure(command: &str, sample: &str) -> Value {
    let tmp = tempdir().expect("tempdir");
    let out = tmp.path().join("out.wasm");
    let mut cmd = Command::cargo_bin("clg").expect("bin");
    cmd.args(["--json-errors", command])
        .arg(repo_sample(sample));
    if command == "build" {
        cmd.args(["-o"]).arg(&out);
    }
    let output = cmd.assert().failure().get_output().stdout.clone();
    serde_json::from_slice(&output).expect("json")
}

#[test]
fn sdk_usability_import_ergonomics_local_module_flow() {
    let tmp = tempdir().expect("tempdir");
    let root = tmp.path();
    let math_dir = root.join("math");
    fs::create_dir_all(&math_dir).expect("mkdir math");

    fs::write(
        math_dir.join("util.clear"),
        r#"
            export pure function add1(x: Int) -> Int { x + 1 }
        "#,
    )
    .expect("write util");

    fs::write(
        root.join("main.clear"),
        r#"
            import math::util
            import math::util::{add1}

            function main() -> Int {
                util::add1(40) + add1(0)
            }
        "#,
    )
    .expect("write main");

    let wasm_path = root.join("out.wasm");
    Command::cargo_bin("clg")
        .expect("bin")
        .args(["build"])
        .arg(root.join("main.clear"))
        .args(["-o"])
        .arg(&wasm_path)
        .assert()
        .success();

    Command::cargo_bin("clg")
        .expect("bin")
        .args(["run"])
        .arg(&wasm_path)
        .assert()
        .success()
        .stdout(predicate::str::is_match(r"(?m)^\s*42\s*$").expect("regex"));
}

#[test]
fn sdk_usability_migration_error_quality_metrics() {
    let cases = [
        UsabilityCase {
            file: "migration/02_interface_type_params.clear",
            command: "build",
            expected_code: "T246",
            expected_stage: "type",
        },
        UsabilityCase {
            file: "migration/03_implementation_method_type_params.clear",
            command: "build",
            expected_code: "T245",
            expected_stage: "type",
        },
        UsabilityCase {
            file: "migration/05_set_resource.clear",
            command: "build",
            expected_code: "T806",
            expected_stage: "type",
        },
        UsabilityCase {
            file: "migration/06_array_resource.clear",
            command: "build",
            expected_code: "T806",
            expected_stage: "type",
        },
    ];

    let mut exact_code_matches = 0usize;
    let mut single_error_cases = 0usize;
    let mut stage_matches = 0usize;
    let mut spanful_cases = 0usize;
    let mut actionable_message_cases = 0usize;

    for case in cases {
        let v = run_json_failure(case.command, case.file);
        assert_eq!(v.get("ok").and_then(|b| b.as_bool()), Some(false));
        let errs = v
            .get("errors")
            .and_then(|e| e.as_array())
            .expect("errors array");
        if errs.len() == 1 {
            single_error_cases += 1;
        }
        let e0 = &errs[0];
        if e0.get("code").and_then(|s| s.as_str()) == Some(case.expected_code) {
            exact_code_matches += 1;
        }
        if e0.get("stage").and_then(|s| s.as_str()) == Some(case.expected_stage) {
            stage_matches += 1;
        }
        if e0.get("start").and_then(|n| n.as_u64()).is_some()
            && e0.get("end").and_then(|n| n.as_u64()).is_some()
        {
            spanful_cases += 1;
        }
        let message = e0
            .get("message")
            .and_then(|s| s.as_str())
            .unwrap_or_default();
        let actionable = message.len() >= 20
            && (message.contains("use a refined alias")
                || message.contains("not supported")
                || message.contains("unsupported")
                || message.contains("cannot declare"));
        if actionable {
            actionable_message_cases += 1;
        }
    }

    let total = 4usize;
    assert_eq!(
        exact_code_matches, total,
        "sdk usability code-match regression: {exact_code_matches}/{total}"
    );
    assert_eq!(
        single_error_cases, total,
        "sdk migration friction regression (multiple errors): {single_error_cases}/{total}"
    );
    assert_eq!(
        stage_matches, total,
        "sdk usability stage-match regression: {stage_matches}/{total}"
    );
    assert_eq!(
        spanful_cases, total,
        "sdk usability span-quality regression: {spanful_cases}/{total}"
    );
    assert_eq!(
        actionable_message_cases, total,
        "sdk usability actionable-message regression: {actionable_message_cases}/{total}"
    );
}

#[test]
fn sdk_usability_refinement_ergonomics_lifted_samples_build_and_parse() {
    Command::cargo_bin("clg")
        .expect("bin")
        .args(["parse"])
        .arg(repo_sample("migration/01_inline_refinement_param.clear"))
        .assert()
        .success();

    let tmp = tempdir().expect("tempdir");
    let out = tmp.path().join("out.wasm");
    Command::cargo_bin("clg")
        .expect("bin")
        .args(["build"])
        .arg(repo_sample("migration/04_generic_refinement_alias.clear"))
        .args(["-o"])
        .arg(&out)
        .assert()
        .success();
}
