use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::ErrorKind;
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
const TEST_EXPECTATION_FAILURE_CODE: &str = "C141";
const DEFAULT_TEST_TIMEOUT_MS: u64 = 120_000;
const DEFAULT_TEST_WORKER_FUEL_LIMIT: u64 = 50_000_000;
const DEFAULT_TEST_WORKER_MEMORY_LIMIT_BYTES: usize = 64 * 1024 * 1024;

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
    expected_outcome: Option<TestExpectedOutcomeSpec>,
}

#[derive(Debug)]
struct TestStoreState {
    limits: wt::StoreLimits,
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
    #[serde(default)]
    expected_outcome: Option<TestExpectedOutcomeSpec>,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum TestExpectedOutcomeKind {
    Pass,
    AssertionFalse,
    Runtime,
    Timeout,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct TestExpectedOutcomeSpec {
    kind: TestExpectedOutcomeKind,
    #[serde(default)]
    failure_code: Option<String>,
    #[serde(default)]
    reason_contains: Option<String>,
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
    failure_id: Option<String>,
    reason: Option<String>,
    expected_outcome: Option<TestExpectedOutcomeSpec>,
    assertion_diff: Option<AssertionDiff>,
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
    failure_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    expected_outcome: Option<TestExpectedOutcomeSpec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    assertion_diff: Option<AssertionDiff>,
    captured_stdout: String,
    captured_stderr: String,
    replay: ReplayContract,
}

#[derive(Debug, Clone, Serialize)]
struct AssertionDiff {
    schema_version: u64,
    kind: &'static str,
    expected: String,
    actual: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    expected_failure_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    actual_failure_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reason_contains: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    actual_reason: Option<String>,
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

