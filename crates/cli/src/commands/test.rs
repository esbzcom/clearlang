use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;
use clap::ValueEnum;
use clg_ast::Func;
use clg_parser::parse_errors;
use serde::{Deserialize, Serialize};

use crate::commands::helpers::{make_single_json_error, CommandError};
use crate::logging::{Logger, StageTimings};

const TEST_DISCOVERY_ERROR_CODE: &str = "C134";
const TEST_PLAN_ERROR_CODE: &str = "C135";
const TEST_MOCK_ERROR_CODE: &str = "C136";

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

    logger.summary(&timings);
    emit_success_report(&roots.project_root, &selected, discovered.len(), report);
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

    if canonical_input.file_name().and_then(|s| s.to_str()) == Some("unit") {
        if canonical_input
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
    let mut files = Vec::new();
    collect_clear_files(root, &mut files, json_errors)?;
    files.sort_by(|a, b| a.as_os_str().cmp(b.as_os_str()));
    Ok(files)
}

fn collect_clear_files(dir: &Path, out: &mut Vec<PathBuf>, json_errors: bool) -> Result<()> {
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
        let ty = entry.file_type().map_err(|err| {
            test_error(
                TEST_DISCOVERY_ERROR_CODE,
                format!("reading `{}`: {err}", path.display()),
                path.as_path(),
                0,
                0,
                json_errors,
            )
        })?;
        if ty.is_dir() {
            collect_clear_files(path.as_path(), out, json_errors)?;
        } else if ty.is_file() && is_clear_source(path.as_path()) {
            out.push(path);
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
        if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                sets.insert(name.to_string());
            }
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

fn emit_success_report(
    project_root: &Path,
    selected: &[TestCase],
    discovered_count: usize,
    report: TestReportFormat,
) {
    let status = if selected.is_empty() {
        "no_tests"
    } else {
        "ok"
    };
    match report {
        TestReportFormat::Human => {
            if selected.is_empty() {
                println!(
                    "test ok: no_tests (discovered={}, selected=0, execution=deferred)",
                    discovered_count
                );
            } else {
                println!(
                    "test contract ok: discovered={}, selected={}, execution=deferred",
                    discovered_count,
                    selected.len()
                );
            }
        }
        TestReportFormat::Json => {
            let payload = JsonTestSummary {
                schema_version: 1,
                status,
                report: report.as_str(),
                discovered: discovered_count,
                selected: selected.len(),
                executed: 0,
                passed: 0,
                failed: 0,
                note: "Gate D foundation: discovery + contract validation only; execution lands in runner core slices",
                tests: selected
                    .iter()
                    .map(|case| JsonTestCase {
                        id: case.id.clone(),
                        file: normalize_relpath(project_root, case.file.as_path()),
                        function: case.function.clone(),
                    })
                    .collect(),
            };
            println!(
                "{}",
                serde_json::to_string_pretty(&payload).expect("serialize test summary")
            );
        }
        TestReportFormat::Junit => {
            let xml = render_junit_report(selected, status);
            println!("{xml}");
        }
    }
}

fn render_junit_report(selected: &[TestCase], status: &str) -> String {
    let mut out = String::new();
    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    out.push_str(&format!(
        "<testsuite name=\"clg.test\" tests=\"{}\" failures=\"0\" errors=\"0\" skipped=\"{}\" status=\"{}\">\n",
        selected.len(),
        selected.len(),
        escape_xml(status)
    ));
    for case in selected {
        out.push_str(&format!(
            "  <testcase classname=\"{}\" name=\"{}\">\n",
            escape_xml(case.file.display().to_string().as_str()),
            escape_xml(case.function.as_str())
        ));
        out.push_str("    <skipped message=\"execution deferred in Gate D foundation\" />\n");
        out.push_str("  </testcase>\n");
    }
    out.push_str("</testsuite>");
    out
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
    fs::canonicalize(path).map_err(|err| {
        {
            test_error(
                TEST_DISCOVERY_ERROR_CODE,
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
    use super::{escape_xml, normalize_relpath, TestPlan, TestPlanCase};
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
}
