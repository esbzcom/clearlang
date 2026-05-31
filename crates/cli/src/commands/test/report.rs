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
        let failure_id = case.failure_id.as_deref().unwrap_or("runtime.unknown");
        let reason = case.reason.as_deref().unwrap_or("unknown failure");
        println!(" - [{}:{}] {}: {}", failure_code, failure_id, case.id, reason);
        if let Some(diff) = case.assertion_diff.as_ref() {
            println!(
                "   diff({}): expected={}, actual={}",
                diff.kind, diff.expected, diff.actual
            );
        }
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
        note: "Gate D/F machine-readable contract v1: serial execution, deterministic failure codes and failure_id taxonomy, optional expected_outcome/assertion_diff fields, per-test capture fields, replay argv, deterministic mock-set execution, and runtime safety limits (fuel+memory)",
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
                failure_id: case.failure_id.clone(),
                reason: case.reason.clone(),
                expected_outcome: case.expected_outcome.clone(),
                assertion_diff: case.assertion_diff.clone(),
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

