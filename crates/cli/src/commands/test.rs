use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use clap::ValueEnum;
use clg_ast::{Func, Type};
use clg_codegen_wasm::{
    emit_from_ir_with_opts, CodegenOpts, ExportAlias, ExternalImport,
    StdCoreLinkMode as WasmStdCoreLinkMode,
};
use clg_parser::parse_errors;
use clg_typer::{check_with_vcs_with_std_and_external, ExternalBuiltinSig, TyperError};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use wasmtime as wt;

use crate::commands::helpers::{extract_function_name, make_single_json_error, CommandError};
use crate::commands::modules::load_program;
use crate::logging::{LogLevel, Logger, StageTimings};

const TEST_DISCOVERY_ERROR_CODE: &str = "C134";
const TEST_PLAN_ERROR_CODE: &str = "C135";
const TEST_MOCK_ERROR_CODE: &str = "C136";
const TEST_TIMEOUT_FAILURE_CODE: &str = "C137";
const TEST_RUNTIME_FAILURE_CODE: &str = "C138";
const TEST_ASSERTION_FAILURE_CODE: &str = "C139";
const DEFAULT_TEST_TIMEOUT_MS: u64 = 120_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum TestReportFormat {
    Human,
    Json,
    Junit,
}

impl TestReportFormat {
    fn as_str(self) -> &'static str {
        match self {
            TestReportFormat::Human => "human",
            TestReportFormat::Json => "json",
            TestReportFormat::Junit => "junit",
        }
    }
}

#[derive(Debug, Clone)]
struct Roots {
    project_root: PathBuf,
    tests_root: PathBuf,
    unit_root: PathBuf,
}

#[derive(Debug, Clone)]
struct TestCase {
    id: String,
    file: PathBuf,
    function: String,
}

#[derive(Debug, Clone)]
struct TestExecutionConfig {
    timeout_ms: u64,
    mock_sets: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct TestPlan {
    schema_version: u64,
    #[serde(default)]
    default_mock_sets: Vec<String>,
    #[serde(default)]
    cases: Vec<TestPlanCase>,
}

#[derive(Debug, Deserialize)]
struct TestPlanCase {
    test_id: String,
    #[serde(default)]
    mock_sets: Vec<String>,
    timeout_ms: Option<u64>,
}

#[derive(Debug)]
struct TestPlanLookup {
    default_mock_sets: Vec<String>,
    cases_by_id: HashMap<String, TestPlanCase>,
}

#[derive(Debug, Clone)]
struct ExecutedTestCase {
    id: String,
    file: String,
    function: String,
    timeout_ms: u64,
    mock_sets: Vec<String>,
    status: &'static str,
    failure_kind: Option<&'static str>,
    failure_code: Option<&'static str>,
    reason: Option<String>,
    captured_stdout: String,
    captured_stderr: String,
    replay: ReplayContract,
}

#[derive(Debug, Clone)]
struct TestRunSummary {
    status: &'static str,
    discovered: usize,
    selected: usize,
    executed: usize,
    passed: usize,
    failed: usize,
}

#[derive(Debug, Serialize)]
struct JsonTestSummary {
    schema_version: u64,
    status: &'static str,
    report: &'static str,
    discovered: usize,
    selected: usize,
    executed: usize,
    passed: usize,
    failed: usize,
    note: &'static str,
    tests: Vec<JsonTestCase>,
}

#[derive(Debug, Serialize)]
struct JsonTestCase {
    id: String,
    file: String,
    function: String,
    timeout_ms: u64,
    mock_sets: Vec<String>,
    status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    failure_kind: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    failure_code: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<String>,
    captured_stdout: String,
    captured_stderr: String,
    replay: ReplayContract,
}

#[derive(Debug, Clone, Serialize)]
struct ReplayContract {
    argv: Vec<String>,
}

pub fn run(
    path: Option<PathBuf>,
    filter: Option<String>,
    report: TestReportFormat,
    json_errors: bool,
    logger: Logger,
) -> Result<()> {
    let input_path = path.unwrap_or_else(|| PathBuf::from("."));

    let mut timings = StageTimings::new();
    let roots = {
        let _stage = timings.start(logger, "test_contract");
        resolve_roots(input_path.as_path(), json_errors)?
    };

    let discovered = {
        let _stage = timings.start(logger, "test_discover");
        discover_tests(&roots, json_errors)?
    };

    {
        let _stage = timings.start(logger, "test_plan");
        validate_test_plan(&roots, &discovered, json_errors)?
    }

    let selected = select_tests(&discovered, filter.as_deref());
    let plan_lookup = load_test_plan_lookup(&roots, json_errors)?;
    let execution_config =
        build_execution_config(&selected, plan_lookup.as_ref(), &roots, json_errors)?;

    let results = if selected.is_empty() {
        Vec::new()
    } else {
        let engine = build_execution_engine().context("building test execution engine")?;
        let modules = {
            let _stage = timings.start(logger, "test_compile");
            compile_modules_for_selected(
                &engine,
                &roots,
                &selected,
                &execution_config,
                json_errors,
            )?
        };
        {
            let _stage = timings.start(logger, "test_execute");
            execute_selected_tests(
                &engine,
                &roots.project_root,
                &selected,
                &execution_config,
                modules.as_slice(),
                json_errors,
                logger,
            )?
        }
    };

    let summary = summarize_results(discovered.len(), selected.len(), &results);

    logger.summary(&timings);
    emit_report(report, &summary, &results);
    if summary.failed > 0 {
        return Err(CommandError::stderr(format!(
            "test failures: {} failed ({} passed)",
            summary.failed, summary.passed
        ))
        .into());
    }
    Ok(())
}

fn resolve_roots(input: &Path, json_errors: bool) -> Result<Roots> {
    if !input.exists() {
        return Err(test_error(
            TEST_DISCOVERY_ERROR_CODE,
            format!("path does not exist: `{}`", input.display()),
            input,
            0,
            0,
            json_errors,
        )
        .into());
    }
    if input.is_file() {
        return Err(test_error(
            TEST_DISCOVERY_ERROR_CODE,
            format!(
                "expected a project/tests directory for `clg test`, found file `{}`",
                input.display()
            ),
            input,
            0,
            0,
            json_errors,
        )
        .into());
    }

    let canonical_input = canonicalize_path(input, json_errors)?;
    let tests_from_project = canonical_input.join("tests");
    let unit_from_project = tests_from_project.join("unit");
    if unit_from_project.is_dir() {
        return Ok(Roots {
            project_root: canonical_input,
            tests_root: tests_from_project,
            unit_root: unit_from_project,
        });
    }

    if canonical_input.file_name().and_then(|s| s.to_str()) == Some("tests") {
        let unit_root = canonical_input.join("unit");
        if unit_root.is_dir() {
            let project_root = canonical_input
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_else(|| canonical_input.clone());
            return Ok(Roots {
                project_root,
                tests_root: canonical_input,
                unit_root,
            });
        }
    }

    if canonical_input.file_name().and_then(|s| s.to_str()) == Some("unit")
        && canonical_input
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|s| s.to_str())
            == Some("tests")
    {
        let tests_root = canonical_input
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| canonical_input.clone());
        let project_root = tests_root
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| tests_root.clone());
        return Ok(Roots {
            project_root,
            tests_root,
            unit_root: canonical_input,
        });
    }

    Err(test_error(
        TEST_DISCOVERY_ERROR_CODE,
        format!(
            "expected a project/tests root containing `tests/unit`, found `{}`",
            input.display()
        ),
        input,
        0,
        0,
        json_errors,
    )
    .into())
}

fn discover_tests(roots: &Roots, json_errors: bool) -> Result<Vec<TestCase>> {
    let files = discover_clear_files(roots.unit_root.as_path(), json_errors)?;
    if files.is_empty() {
        return Ok(Vec::new());
    }

    let mut discovered = Vec::new();
    let mut seen_ids = HashSet::new();

    for file in files {
        let source = fs::read_to_string(&file).map_err(|err| {
            test_error(
                TEST_DISCOVERY_ERROR_CODE,
                format!("reading `{}`: {err}", file.display()),
                file.as_path(),
                0,
                0,
                json_errors,
            )
        })?;
        let program = parse_errors(source.as_str()).map_err(|errs| {
            let first = errs.first().expect("parser returned at least one error");
            test_error(
                TEST_DISCOVERY_ERROR_CODE,
                format!(
                    "parsing `{}` for test discovery failed: {}",
                    file.display(),
                    first.message
                ),
                file.as_path(),
                first.start,
                first.end,
                json_errors,
            )
        })?;

        collect_test_functions(
            &mut discovered,
            &mut seen_ids,
            roots.project_root.as_path(),
            file.as_path(),
            &program.funcs,
            json_errors,
        )?;
    }

    discovered.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(discovered)
}

fn collect_test_functions(
    discovered: &mut Vec<TestCase>,
    seen_ids: &mut HashSet<String>,
    project_root: &Path,
    file: &Path,
    funcs: &[Func],
    json_errors: bool,
) -> Result<()> {
    for func in funcs {
        if !func.name.starts_with("test_") {
            continue;
        }
        if !func.type_params.is_empty() {
            return Err(test_error(
                TEST_DISCOVERY_ERROR_CODE,
                format!(
                    "test function `{}` in `{}` must not be generic",
                    func.name,
                    file.display()
                ),
                file,
                0,
                0,
                json_errors,
            )
            .into());
        }
        if !func.params.is_empty() {
            return Err(test_error(
                TEST_DISCOVERY_ERROR_CODE,
                format!(
                    "test function `{}` in `{}` must have signature `() -> Bool`",
                    func.name,
                    file.display()
                ),
                file,
                0,
                0,
                json_errors,
            )
            .into());
        }
        if func.ret != Type::Bool {
            return Err(test_error(
                TEST_DISCOVERY_ERROR_CODE,
                format!(
                    "test function `{}` in `{}` must return `Bool`",
                    func.name,
                    file.display()
                ),
                file,
                0,
                0,
                json_errors,
            )
            .into());
        }
        let relative = normalize_relpath(project_root, file);
        let test_id = format!("{relative}::{}", func.name);
        if !seen_ids.insert(test_id.clone()) {
            return Err(test_error(
                TEST_DISCOVERY_ERROR_CODE,
                format!("duplicate discovered test id: `{test_id}`"),
                file,
                0,
                0,
                json_errors,
            )
            .into());
        }
        discovered.push(TestCase {
            id: test_id,
            file: file.to_path_buf(),
            function: func.name.clone(),
        });
    }
    Ok(())
}

fn discover_clear_files(root: &Path, json_errors: bool) -> Result<Vec<PathBuf>> {
    let canonical_root = canonicalize_path(root, json_errors)?;
    let mut files = Vec::new();
    collect_clear_files(
        canonical_root.as_path(),
        canonical_root.as_path(),
        &mut files,
        TEST_DISCOVERY_ERROR_CODE,
        json_errors,
    )?;
    files.sort_by(|a, b| a.as_os_str().cmp(b.as_os_str()));
    Ok(files)
}

fn collect_clear_files(
    dir: &Path,
    canonical_root: &Path,
    out: &mut Vec<PathBuf>,
    error_code: &'static str,
    json_errors: bool,
) -> Result<()> {
    let mut entries = fs::read_dir(dir)
        .map_err(|err| {
            test_error(
                error_code,
                format!("reading `{}`: {err}", dir.display()),
                dir,
                0,
                0,
                json_errors,
            )
        })?
        .collect::<std::io::Result<Vec<_>>>()
        .map_err(|err| {
            test_error(
                error_code,
                format!("reading `{}`: {err}", dir.display()),
                dir,
                0,
                0,
                json_errors,
            )
        })?;
    entries.sort_by(|a, b| a.path().as_os_str().cmp(b.path().as_os_str()));

    for entry in entries {
        let path = entry.path();
        let ty = entry.file_type().map_err(|err| {
            test_error(
                error_code,
                format!("reading `{}`: {err}", path.display()),
                path.as_path(),
                0,
                0,
                json_errors,
            )
        })?;
        if ty.is_symlink() {
            return Err(test_error(
                error_code,
                format!(
                    "path safety violation: symlink entries are not allowed in `{}` (`{}`)",
                    canonical_root.display(),
                    path.display()
                ),
                path.as_path(),
                0,
                0,
                json_errors,
            )
            .into());
        }
        if ty.is_dir() {
            let canonical_dir =
                canonicalize_path_with_code(path.as_path(), error_code, json_errors)?;
            if !canonical_dir.starts_with(canonical_root) {
                return Err(test_error(
                    error_code,
                    format!(
                        "path safety violation: directory `{}` resolves outside `{}`",
                        path.display(),
                        canonical_root.display()
                    ),
                    path.as_path(),
                    0,
                    0,
                    json_errors,
                )
                .into());
            }
            collect_clear_files(path.as_path(), canonical_root, out, error_code, json_errors)?;
        } else if ty.is_file() && is_clear_source(path.as_path()) {
            let canonical_file =
                canonicalize_path_with_code(path.as_path(), error_code, json_errors)?;
            if !canonical_file.starts_with(canonical_root) {
                return Err(test_error(
                    error_code,
                    format!(
                        "path safety violation: file `{}` resolves outside `{}`",
                        path.display(),
                        canonical_root.display()
                    ),
                    path.as_path(),
                    0,
                    0,
                    json_errors,
                )
                .into());
            }
            out.push(canonical_file);
        }
    }
    Ok(())
}

fn is_clear_source(path: &Path) -> bool {
    path.extension().and_then(|ext| ext.to_str()) == Some("clear")
}

fn select_tests(discovered: &[TestCase], filter: Option<&str>) -> Vec<TestCase> {
    match filter {
        Some(pattern) => discovered
            .iter()
            .filter(|case| case.id.contains(pattern))
            .cloned()
            .collect(),
        None => discovered.to_vec(),
    }
}

fn validate_test_plan(roots: &Roots, discovered: &[TestCase], json_errors: bool) -> Result<()> {
    let plan_path = roots.tests_root.join("test-plan.json");
    if !plan_path.exists() {
        return Ok(());
    }

    let raw = fs::read_to_string(&plan_path).map_err(|err| {
        test_error(
            TEST_PLAN_ERROR_CODE,
            format!("reading `{}`: {err}", plan_path.display()),
            plan_path.as_path(),
            0,
            0,
            json_errors,
        )
    })?;
    let plan: TestPlan = serde_json::from_str(raw.as_str()).map_err(|err| {
        test_error(
            TEST_PLAN_ERROR_CODE,
            format!("parsing `{}`: {err}", plan_path.display()),
            plan_path.as_path(),
            0,
            0,
            json_errors,
        )
    })?;

    if plan.schema_version != 1 {
        return Err(test_error(
            TEST_PLAN_ERROR_CODE,
            format!(
                "`{}` has unsupported schema_version {}; expected 1",
                plan_path.display(),
                plan.schema_version
            ),
            plan_path.as_path(),
            0,
            0,
            json_errors,
        )
        .into());
    }

    let mut sorted_ids: Vec<&str> = plan.cases.iter().map(|c| c.test_id.as_str()).collect();
    let original_ids = sorted_ids.clone();
    sorted_ids.sort_unstable();
    if sorted_ids != original_ids {
        return Err(test_error(
            TEST_PLAN_ERROR_CODE,
            format!(
                "`{}` cases must be sorted by test_id for deterministic diffs",
                plan_path.display()
            ),
            plan_path.as_path(),
            0,
            0,
            json_errors,
        )
        .into());
    }

    let mut case_ids = HashSet::new();
    for case in &plan.cases {
        if !case_ids.insert(case.test_id.as_str()) {
            return Err(test_error(
                TEST_PLAN_ERROR_CODE,
                format!(
                    "`{}` has duplicate test_id `{}`",
                    plan_path.display(),
                    case.test_id
                ),
                plan_path.as_path(),
                0,
                0,
                json_errors,
            )
            .into());
        }
        if let Some(timeout_ms) = case.timeout_ms {
            if timeout_ms == 0 {
                return Err(test_error(
                    TEST_PLAN_ERROR_CODE,
                    format!(
                        "`{}` case `{}` has invalid timeout_ms=0",
                        plan_path.display(),
                        case.test_id
                    ),
                    plan_path.as_path(),
                    0,
                    0,
                    json_errors,
                )
                .into());
            }
        }
    }

    let discovered_ids: HashSet<&str> = discovered.iter().map(|case| case.id.as_str()).collect();
    for case in &plan.cases {
        if !discovered_ids.contains(case.test_id.as_str()) {
            return Err(test_error(
                TEST_PLAN_ERROR_CODE,
                format!(
                    "`{}` references unknown test_id `{}`",
                    plan_path.display(),
                    case.test_id
                ),
                plan_path.as_path(),
                0,
                0,
                json_errors,
            )
            .into());
        }
    }

    let available_sets = discover_mock_sets(roots.tests_root.as_path(), json_errors)?;
    validate_mock_sets(
        &plan.default_mock_sets,
        "default_mock_sets",
        roots.tests_root.as_path(),
        &available_sets,
        json_errors,
    )?;
    for case in &plan.cases {
        validate_mock_sets(
            &case.mock_sets,
            case.test_id.as_str(),
            roots.tests_root.as_path(),
            &available_sets,
            json_errors,
        )?;
    }

    Ok(())
}

fn load_test_plan_lookup(roots: &Roots, json_errors: bool) -> Result<Option<TestPlanLookup>> {
    let plan_path = roots.tests_root.join("test-plan.json");
    if !plan_path.exists() {
        return Ok(None);
    }
    let raw = fs::read_to_string(&plan_path).map_err(|err| {
        test_error(
            TEST_PLAN_ERROR_CODE,
            format!("reading `{}`: {err}", plan_path.display()),
            plan_path.as_path(),
            0,
            0,
            json_errors,
        )
    })?;
    let plan: TestPlan = serde_json::from_str(raw.as_str()).map_err(|err| {
        test_error(
            TEST_PLAN_ERROR_CODE,
            format!("parsing `{}`: {err}", plan_path.display()),
            plan_path.as_path(),
            0,
            0,
            json_errors,
        )
    })?;
    let cases_by_id = plan
        .cases
        .into_iter()
        .map(|case| (case.test_id.clone(), case))
        .collect();
    Ok(Some(TestPlanLookup {
        default_mock_sets: plan.default_mock_sets,
        cases_by_id,
    }))
}

fn build_execution_config(
    selected: &[TestCase],
    plan: Option<&TestPlanLookup>,
    roots: &Roots,
    json_errors: bool,
) -> Result<Vec<TestExecutionConfig>> {
    let mut out = Vec::with_capacity(selected.len());
    for case in selected {
        let (timeout_ms, mock_sets) = if let Some(plan) = plan {
            let case_plan = plan.cases_by_id.get(case.id.as_str());
            if !plan.default_mock_sets.is_empty() && case_plan.is_none() {
                return Err(test_error(
                    TEST_MOCK_ERROR_CODE,
                    format!(
                        "missing explicit mock binding for test `{}`; add a `cases[]` entry in `tests/test-plan.json`",
                        case.id
                    ),
                    roots.tests_root.as_path(),
                    0,
                    0,
                    json_errors,
                )
                .into());
            }
            let timeout_ms = case_plan
                .and_then(|entry| entry.timeout_ms)
                .unwrap_or(DEFAULT_TEST_TIMEOUT_MS);
            let mock_sets = merge_mock_sets(
                plan.default_mock_sets.as_slice(),
                case_plan
                    .map(|entry| entry.mock_sets.as_slice())
                    .unwrap_or(&[]),
            );
            (timeout_ms, mock_sets)
        } else {
            (DEFAULT_TEST_TIMEOUT_MS, Vec::new())
        };

        out.push(TestExecutionConfig {
            timeout_ms,
            mock_sets,
        });
    }
    Ok(out)
}

fn merge_mock_sets(default_sets: &[String], case_sets: &[String]) -> Vec<String> {
    let combined = default_sets
        .iter()
        .chain(case_sets.iter())
        .cloned()
        .collect::<Vec<_>>();
    let mut deduped_rev = Vec::new();
    let mut seen = HashSet::new();
    for name in combined.into_iter().rev() {
        if seen.insert(name.clone()) {
            deduped_rev.push(name);
        }
    }
    deduped_rev.reverse();
    deduped_rev
}

fn discover_mock_sets(tests_root: &Path, json_errors: bool) -> Result<HashSet<String>> {
    let mocks_root = tests_root.join("mocks");
    if !mocks_root.exists() {
        return Ok(HashSet::new());
    }
    if !mocks_root.is_dir() {
        return Err(test_error(
            TEST_MOCK_ERROR_CODE,
            format!(
                "expected `{}` to be a directory containing mock sets",
                mocks_root.display()
            ),
            mocks_root.as_path(),
            0,
            0,
            json_errors,
        )
        .into());
    }
    let canonical_mocks_root =
        canonicalize_path_with_code(mocks_root.as_path(), TEST_MOCK_ERROR_CODE, json_errors)?;

    let mut entries = fs::read_dir(mocks_root.as_path())
        .map_err(|err| {
            test_error(
                TEST_MOCK_ERROR_CODE,
                format!("reading `{}`: {err}", mocks_root.display()),
                mocks_root.as_path(),
                0,
                0,
                json_errors,
            )
        })?
        .collect::<std::io::Result<Vec<_>>>()
        .map_err(|err| {
            test_error(
                TEST_MOCK_ERROR_CODE,
                format!("reading `{}`: {err}", mocks_root.display()),
                mocks_root.as_path(),
                0,
                0,
                json_errors,
            )
        })?;
    entries.sort_by(|a, b| a.path().as_os_str().cmp(b.path().as_os_str()));

    let mut sets = HashSet::new();
    for entry in entries {
        let path = entry.path();
        let file_type = entry.file_type().map_err(|err| {
            test_error(
                TEST_MOCK_ERROR_CODE,
                format!("reading `{}`: {err}", path.display()),
                path.as_path(),
                0,
                0,
                json_errors,
            )
        })?;
        if file_type.is_symlink() {
            return Err(test_error(
                TEST_MOCK_ERROR_CODE,
                format!(
                    "path safety violation: symlink mock-set entries are not allowed in `{}` (`{}`)",
                    mocks_root.display(),
                    path.display()
                ),
                path.as_path(),
                0,
                0,
                json_errors,
            )
            .into());
        }
        if file_type.is_dir() {
            let canonical_set_root =
                canonicalize_path_with_code(path.as_path(), TEST_MOCK_ERROR_CODE, json_errors)?;
            if !canonical_set_root.starts_with(&canonical_mocks_root) {
                return Err(test_error(
                    TEST_MOCK_ERROR_CODE,
                    format!(
                        "path safety violation: mock set `{}` resolves outside `{}`",
                        path.display(),
                        mocks_root.display()
                    ),
                    path.as_path(),
                    0,
                    0,
                    json_errors,
                )
                .into());
            }
            let name = path.file_name().and_then(|n| n.to_str()).ok_or_else(|| {
                test_error(
                    TEST_MOCK_ERROR_CODE,
                    format!("mock set path is not valid utf-8: `{}`", path.display()),
                    path.as_path(),
                    0,
                    0,
                    json_errors,
                )
            })?;
            sets.insert(name.to_string());
        }
    }
    Ok(sets)
}

fn validate_mock_sets(
    mock_sets: &[String],
    context: &str,
    tests_root: &Path,
    available_sets: &HashSet<String>,
    json_errors: bool,
) -> Result<()> {
    for set_name in mock_sets {
        if !available_sets.contains(set_name) {
            return Err(test_error(
                TEST_MOCK_ERROR_CODE,
                format!(
                    "unknown mock set `{set_name}` in `{context}`; expected a directory at `{}`",
                    tests_root.join("mocks").join(set_name).display()
                ),
                tests_root,
                0,
                0,
                json_errors,
            )
            .into());
        }
    }
    Ok(())
}

fn build_execution_engine() -> Result<wt::Engine> {
    let mut config = wt::Config::new();
    config.epoch_interruption(true);
    wt::Engine::new(&config).context("creating wasmtime engine")
}

fn compile_modules_for_selected(
    engine: &wt::Engine,
    roots: &Roots,
    selected: &[TestCase],
    execution_config: &[TestExecutionConfig],
    json_errors: bool,
) -> Result<Vec<wt::Module>> {
    if selected.len() != execution_config.len() {
        return Err(test_error(
            TEST_DISCOVERY_ERROR_CODE,
            "internal error: selected/config length mismatch".to_string(),
            roots.project_root.as_path(),
            0,
            0,
            json_errors,
        )
        .into());
    }

    let project_root = canonicalize_path_with_code(
        roots.project_root.as_path(),
        TEST_DISCOVERY_ERROR_CODE,
        json_errors,
    )?;
    let mut modules = Vec::with_capacity(selected.len());
    for (case, config) in selected.iter().zip(execution_config.iter()) {
        let overlay_root = prepare_case_overlay_project(
            roots,
            project_root.as_path(),
            case,
            config.mock_sets.as_slice(),
            json_errors,
        )?;
        let harness = write_case_harness(
            overlay_root.as_path(),
            project_root.as_path(),
            case,
            json_errors,
        )?;
        let module = compile_test_module(
            engine,
            harness.as_path(),
            std::slice::from_ref(&case.function),
            json_errors,
        )?;
        let _ = fs::remove_dir_all(overlay_root.as_path());
        modules.push(module);
    }
    Ok(modules)
}

fn prepare_case_overlay_project(
    roots: &Roots,
    project_root: &Path,
    case: &TestCase,
    mock_sets: &[String],
    json_errors: bool,
) -> Result<PathBuf> {
    if !case.file.starts_with(project_root) {
        return Err(test_error(
            TEST_DISCOVERY_ERROR_CODE,
            format!(
                "internal path error: discovered test `{}` is outside project root `{}`",
                case.file.display(),
                project_root.display()
            ),
            case.file.as_path(),
            0,
            0,
            json_errors,
        )
        .into());
    }

    let overlay_base = std::env::temp_dir().join("clearlang-test-overlays-v1");
    fs::create_dir_all(overlay_base.as_path()).map_err(|err| {
        test_error(
            TEST_DISCOVERY_ERROR_CODE,
            format!("creating `{}`: {err}", overlay_base.display()),
            overlay_base.as_path(),
            0,
            0,
            json_errors,
        )
    })?;

    let overlay_key = stable_overlay_key(project_root, case.id.as_str(), mock_sets);
    let overlay_root = overlay_base.join(overlay_key);
    if overlay_root.exists() {
        fs::remove_dir_all(overlay_root.as_path()).map_err(|err| {
            test_error(
                TEST_DISCOVERY_ERROR_CODE,
                format!("removing `{}`: {err}", overlay_root.display()),
                overlay_root.as_path(),
                0,
                0,
                json_errors,
            )
        })?;
    }
    fs::create_dir_all(overlay_root.as_path()).map_err(|err| {
        test_error(
            TEST_DISCOVERY_ERROR_CODE,
            format!("creating `{}`: {err}", overlay_root.display()),
            overlay_root.as_path(),
            0,
            0,
            json_errors,
        )
    })?;

    let source_files = collect_project_source_files(project_root, json_errors)?;
    for source in source_files {
        let rel = source.strip_prefix(project_root).map_err(|_| {
            test_error(
                TEST_DISCOVERY_ERROR_CODE,
                format!(
                    "internal path error: cannot strip `{}` from `{}`",
                    project_root.display(),
                    source.display()
                ),
                source.as_path(),
                0,
                0,
                json_errors,
            )
        })?;
        copy_overlay_file(
            source.as_path(),
            overlay_root.join(rel).as_path(),
            TEST_DISCOVERY_ERROR_CODE,
            json_errors,
        )?;
    }

    for metadata_file in ["clg.package-metadata.json", "clg-packages.json"] {
        let source = project_root.join(metadata_file);
        if source.is_file() {
            copy_overlay_file(
                source.as_path(),
                overlay_root.join(metadata_file).as_path(),
                TEST_DISCOVERY_ERROR_CODE,
                json_errors,
            )?;
        }
    }

    apply_mock_sets_to_overlay(
        roots.tests_root.as_path(),
        overlay_root.as_path(),
        mock_sets,
        json_errors,
    )?;

    Ok(overlay_root)
}

fn collect_project_source_files(project_root: &Path, json_errors: bool) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    collect_project_source_files_rec(project_root, &mut files, json_errors)?;
    files.sort_by(|a, b| a.as_os_str().cmp(b.as_os_str()));
    Ok(files)
}

fn collect_project_source_files_rec(
    dir: &Path,
    out: &mut Vec<PathBuf>,
    json_errors: bool,
) -> Result<()> {
    let mut entries = fs::read_dir(dir)
        .map_err(|err| {
            test_error(
                TEST_DISCOVERY_ERROR_CODE,
                format!("reading `{}`: {err}", dir.display()),
                dir,
                0,
                0,
                json_errors,
            )
        })?
        .collect::<std::io::Result<Vec<_>>>()
        .map_err(|err| {
            test_error(
                TEST_DISCOVERY_ERROR_CODE,
                format!("reading `{}`: {err}", dir.display()),
                dir,
                0,
                0,
                json_errors,
            )
        })?;
    entries.sort_by(|a, b| a.path().as_os_str().cmp(b.path().as_os_str()));

    for entry in entries {
        let path = entry.path();
        let file_type = entry.file_type().map_err(|err| {
            test_error(
                TEST_DISCOVERY_ERROR_CODE,
                format!("reading `{}`: {err}", path.display()),
                path.as_path(),
                0,
                0,
                json_errors,
            )
        })?;
        if file_type.is_symlink() {
            continue;
        }
        if file_type.is_dir() {
            collect_project_source_files_rec(path.as_path(), out, json_errors)?;
        } else if file_type.is_file() && is_clear_source(path.as_path()) {
            out.push(path);
        }
    }
    Ok(())
}

fn apply_mock_sets_to_overlay(
    tests_root: &Path,
    overlay_root: &Path,
    mock_sets: &[String],
    json_errors: bool,
) -> Result<()> {
    if mock_sets.is_empty() {
        return Ok(());
    }

    let mocks_root = tests_root.join("mocks");
    let canonical_mocks_root =
        canonicalize_path_with_code(mocks_root.as_path(), TEST_MOCK_ERROR_CODE, json_errors)?;
    for set_name in mock_sets {
        let set_root = mocks_root.join(set_name);
        let canonical_set_root =
            canonicalize_path_with_code(set_root.as_path(), TEST_MOCK_ERROR_CODE, json_errors)?;
        if !canonical_set_root.starts_with(canonical_mocks_root.as_path()) {
            return Err(test_error(
                TEST_MOCK_ERROR_CODE,
                format!(
                    "path safety violation: mock set `{}` resolves outside `{}`",
                    set_root.display(),
                    mocks_root.display()
                ),
                set_root.as_path(),
                0,
                0,
                json_errors,
            )
            .into());
        }
        let mut mock_files = Vec::new();
        collect_clear_files(
            canonical_set_root.as_path(),
            canonical_set_root.as_path(),
            &mut mock_files,
            TEST_MOCK_ERROR_CODE,
            json_errors,
        )?;
        for source in mock_files {
            let rel = source
                .strip_prefix(canonical_set_root.as_path())
                .map_err(|_| {
                    test_error(
                        TEST_MOCK_ERROR_CODE,
                        format!(
                            "internal path error: cannot strip `{}` from `{}`",
                            canonical_set_root.display(),
                            source.display()
                        ),
                        source.as_path(),
                        0,
                        0,
                        json_errors,
                    )
                })?;
            copy_overlay_file(
                source.as_path(),
                overlay_root.join(rel).as_path(),
                TEST_MOCK_ERROR_CODE,
                json_errors,
            )?;
        }
    }

    Ok(())
}

fn copy_overlay_file(
    source: &Path,
    destination: &Path,
    error_code: &'static str,
    json_errors: bool,
) -> Result<()> {
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent).map_err(|err| {
            test_error(
                error_code,
                format!("creating `{}`: {err}", parent.display()),
                parent,
                0,
                0,
                json_errors,
            )
        })?;
    }
    fs::copy(source, destination).map_err(|err| {
        test_error(
            error_code,
            format!(
                "copying `{}` to `{}`: {err}",
                source.display(),
                destination.display()
            ),
            source,
            0,
            0,
            json_errors,
        )
    })?;
    Ok(())
}

fn write_case_harness(
    overlay_root: &Path,
    project_root: &Path,
    case: &TestCase,
    json_errors: bool,
) -> Result<PathBuf> {
    let rel = case.file.strip_prefix(project_root).map_err(|_| {
        test_error(
            TEST_DISCOVERY_ERROR_CODE,
            format!(
                "internal path error: discovered test `{}` is outside project root `{}`",
                case.file.display(),
                project_root.display()
            ),
            case.file.as_path(),
            0,
            0,
            json_errors,
        )
    })?;
    let module_path = file_to_module_path(rel, case.file.as_path(), json_errors)?;
    let module_alias = module_path.rsplit("::").next().ok_or_else(|| {
        test_error(
            TEST_DISCOVERY_ERROR_CODE,
            format!("invalid module path derived from `{}`", case.file.display()),
            case.file.as_path(),
            0,
            0,
            json_errors,
        )
    })?;
    let harness_source = format!(
        "import {module_path}\n\nfunction {}() -> Bool {{\n    {module_alias}::{}()\n}}\n",
        case.function, case.function
    );
    let digest = stable_overlay_key(project_root, case.id.as_str(), &[]);
    let harness_path = overlay_root.join(format!("__clg_test_harness_{digest}.clear"));
    fs::write(harness_path.as_path(), harness_source).map_err(|err| {
        test_error(
            TEST_DISCOVERY_ERROR_CODE,
            format!("writing `{}`: {err}", harness_path.display()),
            harness_path.as_path(),
            0,
            0,
            json_errors,
        )
    })?;
    Ok(harness_path)
}

fn file_to_module_path(rel_file: &Path, file: &Path, json_errors: bool) -> Result<String> {
    if rel_file.extension().and_then(|ext| ext.to_str()) != Some("clear") {
        return Err(test_error(
            TEST_DISCOVERY_ERROR_CODE,
            format!("expected `.clear` test file, found `{}`", file.display()),
            file,
            0,
            0,
            json_errors,
        )
        .into());
    }
    let mut segments = rel_file
        .components()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    if segments.is_empty() {
        return Err(test_error(
            TEST_DISCOVERY_ERROR_CODE,
            format!("invalid empty module path for `{}`", file.display()),
            file,
            0,
            0,
            json_errors,
        )
        .into());
    }
    let last = segments
        .last_mut()
        .expect("at least one module-path segment exists");
    if let Some(stem) = Path::new(last).file_stem().and_then(|s| s.to_str()) {
        *last = stem.to_string();
    } else {
        return Err(test_error(
            TEST_DISCOVERY_ERROR_CODE,
            format!("invalid utf-8 file name in `{}`", file.display()),
            file,
            0,
            0,
            json_errors,
        )
        .into());
    }
    Ok(segments.join("::"))
}

fn stable_overlay_key(project_root: &Path, test_id: &str, mock_sets: &[String]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(project_root.to_string_lossy().as_bytes());
    hasher.update(b"\n");
    hasher.update(test_id.as_bytes());
    hasher.update(b"\n");
    for set_name in mock_sets {
        hasher.update(set_name.as_bytes());
        hasher.update(b"\n");
    }
    format!("{:x}", hasher.finalize())
}

fn compile_test_module(
    engine: &wt::Engine,
    file: &Path,
    functions: &[String],
    json_errors: bool,
) -> Result<wt::Module> {
    let loaded = load_program(file, json_errors)?;
    let ast = &loaded.program;

    let external_typer_sigs: Vec<ExternalBuiltinSig> = loaded
        .external_imports
        .iter()
        .map(|binding| ExternalBuiltinSig {
            name: binding.function.clone(),
            params: binding.params.clone(),
            ret: binding.ret.clone(),
            effect: binding.effect,
        })
        .collect();
    let external_codegen_imports: Vec<ExternalImport> = loaded
        .external_imports
        .iter()
        .map(|binding| ExternalImport {
            function: binding.function.clone(),
            import_module: binding.import_module.clone(),
            import_name: binding.import_name.clone(),
        })
        .collect();

    let type_output =
        match check_with_vcs_with_std_and_external(ast, &loaded.std_types, &external_typer_sigs) {
            Ok(result) => result,
            Err(err) => {
                if json_errors {
                    if let Some((typer, function)) = find_typer_error(&err) {
                        let json = make_single_json_error(
                            typer.code,
                            "type",
                            typer.message.clone(),
                            file,
                            typer.start,
                            typer.end,
                            function,
                        );
                        return Err(CommandError::json(json).into());
                    }
                    let json = make_single_json_error(
                        "T000",
                        "type",
                        format!("{err:#}"),
                        file,
                        0,
                        0,
                        None,
                    );
                    return Err(CommandError::json(json).into());
                }
                return Err(err.context(format!("type-check failed for `{}`", file.display())));
            }
        };

    let mut export_aliases = Vec::with_capacity(functions.len());
    for function in functions {
        let internal_name =
            resolve_ir_function_name(&type_output.ir, function.as_str(), file, json_errors)?;
        export_aliases.push(ExportAlias {
            target: internal_name,
            export: function.clone(),
        });
    }

    let wasm_bytes = emit_from_ir_with_opts(
        &type_output.ir,
        CodegenOpts {
            debug_names: false,
            proof_section: None,
            export_aliases,
            external_imports: external_codegen_imports,
            std_core_link_mode: WasmStdCoreLinkMode::Intrinsic,
        },
    )
    .with_context(|| format!("codegen failed for `{}`", file.display()))?;

    wt::Module::from_binary(engine, wasm_bytes.as_slice())
        .with_context(|| format!("loading test module `{}`", file.display()))
}

fn resolve_ir_function_name(
    ir: &clg_ir::Module,
    test_function: &str,
    file: &Path,
    json_errors: bool,
) -> Result<String> {
    if let Some(found) = ir.funcs.iter().find(|func| func.name == test_function) {
        return Ok(found.name.clone());
    }

    let suffix = format!("::{test_function}");
    let suffix_matches: Vec<&str> = ir
        .funcs
        .iter()
        .filter_map(|func| {
            if func.name.ends_with(suffix.as_str()) {
                Some(func.name.as_str())
            } else {
                None
            }
        })
        .collect();

    match suffix_matches.as_slice() {
        [single] => Ok((*single).to_string()),
        [] => Err(test_error(
            TEST_DISCOVERY_ERROR_CODE,
            format!(
                "test function `{}` was not lowered in `{}`",
                test_function,
                file.display()
            ),
            file,
            0,
            0,
            json_errors,
        )
        .into()),
        _ => Err(test_error(
            TEST_DISCOVERY_ERROR_CODE,
            format!(
                "test function `{}` in `{}` is ambiguous after lowering: {}",
                test_function,
                file.display(),
                suffix_matches.join(", ")
            ),
            file,
            0,
            0,
            json_errors,
        )
        .into()),
    }
}

fn find_typer_error(err: &anyhow::Error) -> Option<(&TyperError, Option<String>)> {
    let mut function: Option<String> = None;
    for cause in err.chain() {
        if function.is_none() {
            let msg = cause.to_string();
            if let Some(name) = extract_function_name(&msg) {
                function = Some(name);
            }
        }
        if let Some(typer) = cause.downcast_ref::<TyperError>() {
            return Some((typer, function));
        }
    }
    None
}

fn execute_selected_tests(
    engine: &wt::Engine,
    project_root: &Path,
    selected: &[TestCase],
    execution_config: &[TestExecutionConfig],
    modules: &[wt::Module],
    json_errors: bool,
    logger: Logger,
) -> Result<Vec<ExecutedTestCase>> {
    if selected.len() != execution_config.len() {
        return Err(test_error(
            TEST_DISCOVERY_ERROR_CODE,
            "internal error: selected/config length mismatch".to_string(),
            project_root,
            0,
            0,
            json_errors,
        )
        .into());
    }
    if selected.len() != modules.len() {
        return Err(test_error(
            TEST_DISCOVERY_ERROR_CODE,
            "internal error: selected/module length mismatch".to_string(),
            project_root,
            0,
            0,
            json_errors,
        )
        .into());
    }

    let mut results = Vec::with_capacity(selected.len());
    for ((case, config), module) in selected
        .iter()
        .zip(execution_config.iter())
        .zip(modules.iter())
    {
        logger.event(
            LogLevel::Info,
            "start",
            "test_case",
            &[
                ("test_id", case.id.clone()),
                ("timeout_ms", config.timeout_ms.to_string()),
                ("mock_sets", format_mock_sets(config.mock_sets.as_slice())),
            ],
        );
        let outcome = execute_single_test(engine, module, case, config.timeout_ms);
        logger.event(
            LogLevel::Info,
            "finish",
            "test_case",
            &[
                ("test_id", case.id.clone()),
                ("status", outcome.status.to_string()),
                (
                    "failure_code",
                    outcome.failure_code.unwrap_or("<none>").to_string(),
                ),
                ("timeout_ms", config.timeout_ms.to_string()),
            ],
        );
        results.push(ExecutedTestCase {
            id: case.id.clone(),
            file: normalize_relpath(project_root, case.file.as_path()),
            function: case.function.clone(),
            timeout_ms: config.timeout_ms,
            mock_sets: config.mock_sets.clone(),
            status: outcome.status,
            failure_kind: outcome.failure_kind,
            failure_code: outcome.failure_code,
            reason: outcome.reason,
            captured_stdout: outcome.captured_stdout,
            captured_stderr: outcome.captured_stderr,
            replay: replay_contract(case.id.as_str()),
        });
    }

    Ok(results)
}

#[derive(Debug)]
struct TestExecutionOutcome {
    status: &'static str,
    failure_kind: Option<&'static str>,
    failure_code: Option<&'static str>,
    reason: Option<String>,
    captured_stdout: String,
    captured_stderr: String,
}

fn execute_single_test(
    engine: &wt::Engine,
    module: &wt::Module,
    case: &TestCase,
    timeout_ms: u64,
) -> TestExecutionOutcome {
    let mut store = wt::Store::new(engine, ());
    store.set_epoch_deadline(1);

    let instance = match wt::Instance::new(&mut store, module, &[]) {
        Ok(instance) => instance,
        Err(err) => {
            return fail_runtime_outcome(format!(
                "instantiate failed: {}",
                single_line_error(&err)
            ));
        }
    };
    let func = match instance.get_typed_func::<(), i32>(&mut store, case.function.as_str()) {
        Ok(func) => func,
        Err(err) => {
            return fail_runtime_outcome(format!(
                "invalid test signature (expected `() -> Bool`): {}",
                single_line_error(&err)
            ));
        }
    };

    let timeout = Duration::from_millis(timeout_ms.max(1));
    let timed_out = Arc::new(AtomicBool::new(false));
    let stop_watchdog = Arc::new(AtomicBool::new(false));
    let watchdog = spawn_timeout_watchdog(
        engine.clone(),
        timeout,
        Arc::clone(&timed_out),
        Arc::clone(&stop_watchdog),
    );

    let call_result = func.call(&mut store, ());
    stop_watchdog.store(true, Ordering::Relaxed);
    let _ = watchdog.join();

    match call_result {
        Ok(value) => {
            if value == 1 {
                TestExecutionOutcome {
                    status: "passed",
                    failure_kind: None,
                    failure_code: None,
                    reason: None,
                    captured_stdout: String::new(),
                    captured_stderr: String::new(),
                }
            } else {
                TestExecutionOutcome {
                    status: "failed",
                    failure_kind: Some("assertion_false"),
                    failure_code: Some(TEST_ASSERTION_FAILURE_CODE),
                    reason: Some(format!(
                        "assertion returned false (expected 1/true, got {value})"
                    )),
                    captured_stdout: String::new(),
                    captured_stderr: String::new(),
                }
            }
        }
        Err(err) => {
            if timed_out.load(Ordering::Relaxed) {
                return TestExecutionOutcome {
                    status: "failed",
                    failure_kind: Some("timeout"),
                    failure_code: Some(TEST_TIMEOUT_FAILURE_CODE),
                    reason: Some(format!("timeout after {}ms", timeout_ms)),
                    captured_stdout: String::new(),
                    captured_stderr: String::new(),
                };
            }
            fail_runtime_outcome(format!("runtime trap: {}", single_line_error(&err)))
        }
    }
}

fn fail_runtime_outcome(reason: String) -> TestExecutionOutcome {
    TestExecutionOutcome {
        status: "failed",
        failure_kind: Some("runtime"),
        failure_code: Some(TEST_RUNTIME_FAILURE_CODE),
        reason: Some(reason),
        captured_stdout: String::new(),
        captured_stderr: String::new(),
    }
}

fn spawn_timeout_watchdog(
    engine: wt::Engine,
    timeout: Duration,
    timed_out: Arc<AtomicBool>,
    stop_watchdog: Arc<AtomicBool>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let start = Instant::now();
        loop {
            if stop_watchdog.load(Ordering::Relaxed) {
                return;
            }
            let elapsed = start.elapsed();
            if elapsed >= timeout {
                timed_out.store(true, Ordering::Relaxed);
                engine.increment_epoch();
                return;
            }
            let remaining = timeout.saturating_sub(elapsed);
            thread::sleep(remaining.min(Duration::from_millis(5)));
        }
    })
}

fn summarize_results(
    discovered_count: usize,
    selected_count: usize,
    results: &[ExecutedTestCase],
) -> TestRunSummary {
    let passed = results
        .iter()
        .filter(|case| case.status == "passed")
        .count();
    let failed = results
        .iter()
        .filter(|case| case.status == "failed")
        .count();
    let status = if selected_count == 0 {
        "no_tests"
    } else if failed == 0 {
        "ok"
    } else {
        "failed"
    };
    TestRunSummary {
        status,
        discovered: discovered_count,
        selected: selected_count,
        executed: results.len(),
        passed,
        failed,
    }
}

fn emit_report(report: TestReportFormat, summary: &TestRunSummary, results: &[ExecutedTestCase]) {
    match report {
        TestReportFormat::Human => emit_human_report(summary, results),
        TestReportFormat::Json => emit_json_report(report, summary, results),
        TestReportFormat::Junit => {
            let xml = render_junit_report(summary, results);
            println!("{xml}");
        }
    }
}

fn emit_human_report(summary: &TestRunSummary, results: &[ExecutedTestCase]) {
    if summary.status == "no_tests" {
        println!(
            "test ok: no_tests (discovered={}, selected=0, executed=0)",
            summary.discovered
        );
        return;
    }
    if summary.failed == 0 {
        println!(
            "test ok: discovered={}, selected={}, executed={}, passed={}, failed=0",
            summary.discovered, summary.selected, summary.executed, summary.passed
        );
        return;
    }

    println!(
        "test failed: discovered={}, selected={}, executed={}, passed={}, failed={}",
        summary.discovered, summary.selected, summary.executed, summary.passed, summary.failed
    );
    for case in results.iter().filter(|case| case.status == "failed") {
        let failure_code = case.failure_code.unwrap_or(TEST_RUNTIME_FAILURE_CODE);
        let reason = case.reason.as_deref().unwrap_or("unknown failure");
        println!(" - [{}] {}: {}", failure_code, case.id, reason);
        println!(
            "   replay: {}",
            render_replay_command(case.replay.argv.as_slice())
        );
    }
}

fn emit_json_report(
    report: TestReportFormat,
    summary: &TestRunSummary,
    results: &[ExecutedTestCase],
) {
    let payload = JsonTestSummary {
        schema_version: 1,
        status: summary.status,
        report: report.as_str(),
        discovered: summary.discovered,
        selected: summary.selected,
        executed: summary.executed,
        passed: summary.passed,
        failed: summary.failed,
        note: "Gate D machine-readable contract v1: serial execution, deterministic failure codes, per-test capture fields, replay argv, and deterministic mock-set execution",
        tests: results
            .iter()
            .map(|case| JsonTestCase {
                id: case.id.clone(),
                file: case.file.clone(),
                function: case.function.clone(),
                timeout_ms: case.timeout_ms,
                mock_sets: case.mock_sets.clone(),
                status: case.status,
                failure_kind: case.failure_kind,
                failure_code: case.failure_code,
                reason: case.reason.clone(),
                captured_stdout: case.captured_stdout.clone(),
                captured_stderr: case.captured_stderr.clone(),
                replay: case.replay.clone(),
            })
            .collect(),
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&payload).expect("serialize test summary")
    );
}

fn render_junit_report(summary: &TestRunSummary, results: &[ExecutedTestCase]) -> String {
    let mut out = String::new();
    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    out.push_str(&format!(
        "<testsuite name=\"clg.test\" tests=\"{}\" failures=\"{}\" errors=\"0\" skipped=\"0\" status=\"{}\">\n",
        summary.executed,
        summary.failed,
        escape_xml(summary.status)
    ));
    for case in results {
        out.push_str(&format!(
            "  <testcase classname=\"{}\" name=\"{}\">\n",
            escape_xml(case.file.as_str()),
            escape_xml(case.function.as_str())
        ));
        if case.status == "failed" {
            let failure_code = case.failure_code.unwrap_or(TEST_RUNTIME_FAILURE_CODE);
            out.push_str(&format!(
                "    <failure type=\"{}\" message=\"{}\" />\n",
                escape_xml(failure_code),
                escape_xml(case.reason.as_deref().unwrap_or("test failed"))
            ));
        }
        out.push_str(&format!(
            "    <system-out>{}</system-out>\n",
            escape_xml(case.captured_stdout.as_str())
        ));
        out.push_str(&format!(
            "    <system-err>{}</system-err>\n",
            escape_xml(case.captured_stderr.as_str())
        ));
        out.push_str("  </testcase>\n");
    }
    out.push_str("</testsuite>");
    out
}

fn single_line_error(err: &impl std::fmt::Display) -> String {
    err.to_string()
        .lines()
        .next()
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty())
        .unwrap_or_else(|| "unknown error".to_string())
}

fn format_mock_sets(mock_sets: &[String]) -> String {
    if mock_sets.is_empty() {
        "<none>".to_string()
    } else {
        mock_sets.join(",")
    }
}

fn replay_contract(test_id: &str) -> ReplayContract {
    ReplayContract {
        argv: vec![
            "clg".to_string(),
            "test".to_string(),
            "<project-root>".to_string(),
            "--filter".to_string(),
            test_id.to_string(),
            "--report".to_string(),
            "json".to_string(),
        ],
    }
}

fn render_replay_command(argv: &[String]) -> String {
    argv.iter()
        .map(|arg| {
            if arg.bytes().all(|b| {
                b.is_ascii_alphanumeric()
                    || b == b'-'
                    || b == b'_'
                    || b == b':'
                    || b == b'.'
                    || b == b'/'
            }) {
                arg.clone()
            } else {
                format!("\"{}\"", arg.replace('"', "\\\""))
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn escape_xml(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn normalize_relpath(root: &Path, path: &Path) -> String {
    let rel = path.strip_prefix(root).unwrap_or(path);
    rel.components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

fn canonicalize_path(path: &Path, json_errors: bool) -> Result<PathBuf> {
    canonicalize_path_with_code(path, TEST_DISCOVERY_ERROR_CODE, json_errors)
}

fn canonicalize_path_with_code(
    path: &Path,
    error_code: &'static str,
    json_errors: bool,
) -> Result<PathBuf> {
    fs::canonicalize(path).map_err(|err| {
        {
            test_error(
                error_code,
                format!("canonicalizing `{}`: {err}", path.display()),
                path,
                0,
                0,
                json_errors,
            )
        }
        .into()
    })
}

fn test_error(
    code: &'static str,
    message: String,
    file: &Path,
    start: usize,
    end: usize,
    json_errors: bool,
) -> CommandError {
    if json_errors {
        CommandError::json(make_single_json_error(
            code, "test", message, file, start, end, None,
        ))
    } else {
        CommandError::stderr(message)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        escape_xml, merge_mock_sets, normalize_relpath, render_replay_command, replay_contract,
        TestPlan, TestPlanCase, DEFAULT_TEST_TIMEOUT_MS, TEST_ASSERTION_FAILURE_CODE,
        TEST_RUNTIME_FAILURE_CODE, TEST_TIMEOUT_FAILURE_CODE,
    };
    use std::path::Path;

    #[test]
    fn normalize_relpath_uses_forward_slashes() {
        let root = Path::new("C:/repo");
        let path = Path::new("C:/repo/tests/unit/a.clear");
        assert_eq!(normalize_relpath(root, path), "tests/unit/a.clear");
    }

    #[test]
    fn xml_escape_covers_reserved_characters() {
        assert_eq!(escape_xml("<a&b>\"'"), "&lt;a&amp;b&gt;&quot;&apos;");
    }

    #[test]
    fn test_plan_case_order_can_be_checked_deterministically() {
        let plan = TestPlan {
            schema_version: 1,
            default_mock_sets: vec!["common".to_string()],
            cases: vec![
                TestPlanCase {
                    test_id: "tests/unit/b.clear::test_b".to_string(),
                    mock_sets: vec!["common".to_string()],
                    timeout_ms: None,
                },
                TestPlanCase {
                    test_id: "tests/unit/a.clear::test_a".to_string(),
                    mock_sets: vec!["common".to_string()],
                    timeout_ms: Some(120_000),
                },
            ],
        };

        let mut ids: Vec<&str> = plan.cases.iter().map(|c| c.test_id.as_str()).collect();
        let original = ids.clone();
        ids.sort_unstable();
        assert_ne!(
            original, ids,
            "unsorted test plan fixtures should differ from sorted order"
        );
    }

    #[test]
    fn merge_mock_sets_preserves_last_override_deterministically() {
        let merged = merge_mock_sets(
            &["common".to_string(), "base".to_string()],
            &["promo".to_string(), "base".to_string()],
        );
        assert_eq!(merged, vec!["common", "promo", "base"]);
    }

    #[test]
    fn default_timeout_contract_is_two_minutes() {
        assert_eq!(DEFAULT_TEST_TIMEOUT_MS, 120_000);
    }

    #[test]
    fn replay_contract_contains_single_test_filter_flow() {
        let replay = replay_contract("tests/unit/a.clear::test_a");
        assert_eq!(
            replay.argv,
            vec![
                "clg".to_string(),
                "test".to_string(),
                "<project-root>".to_string(),
                "--filter".to_string(),
                "tests/unit/a.clear::test_a".to_string(),
                "--report".to_string(),
                "json".to_string()
            ]
        );
    }

    #[test]
    fn render_replay_command_quotes_non_identifier_arguments() {
        let rendered = render_replay_command(&[
            "clg".to_string(),
            "test".to_string(),
            "<project-root>".to_string(),
            "--filter".to_string(),
            "tests/unit/a.clear::test a".to_string(),
        ]);
        assert!(rendered.contains("\"<project-root>\""));
        assert!(rendered.contains("\"tests/unit/a.clear::test a\""));
    }

    #[test]
    fn test_failure_codes_are_stable() {
        assert_eq!(TEST_TIMEOUT_FAILURE_CODE, "C137");
        assert_eq!(TEST_RUNTIME_FAILURE_CODE, "C138");
        assert_eq!(TEST_ASSERTION_FAILURE_CODE, "C139");
    }
}
