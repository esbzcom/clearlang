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
            route: binding.route,
        })
        .collect();
    let external_codegen_imports: Vec<ExternalImport> = loaded
        .external_imports
        .iter()
        .filter(|binding| binding.route == clg_typer::BuiltinRoute::PackageImport)
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
                (
                    "expected_outcome",
                    expected_outcome_label(config.expected_outcome.as_ref()).to_string(),
                ),
            ],
        );
        let actual = execute_single_test_safely(engine, module, case, config.timeout_ms);
        let outcome = enforce_expected_outcome(actual, config.expected_outcome.as_ref());
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
            failure_id: outcome.failure_id,
            reason: outcome.reason,
            expected_outcome: config.expected_outcome.clone(),
            assertion_diff: outcome.assertion_diff,
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
    failure_id: Option<String>,
    reason: Option<String>,
    assertion_diff: Option<AssertionDiff>,
    captured_stdout: String,
    captured_stderr: String,
}

fn execute_single_test_safely(
    engine: &wt::Engine,
    module: &wt::Module,
    case: &TestCase,
    timeout_ms: u64,
) -> TestExecutionOutcome {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        execute_single_test(engine, module, case, timeout_ms)
    })) {
        Ok(outcome) => outcome,
        Err(payload) => fail_runtime_outcome(format!(
            "worker_crash: {}",
            panic_payload_message(payload.as_ref())
        )),
    }
}

fn execute_single_test(
    engine: &wt::Engine,
    module: &wt::Module,
    case: &TestCase,
    timeout_ms: u64,
) -> TestExecutionOutcome {
    if timeout_ms <= 1 {
        // Contract-level deterministic floor: 1ms is treated as immediate timeout.
        return timeout_failure_outcome(timeout_ms);
    }

    let limits = wt::StoreLimitsBuilder::new()
        .memory_size(DEFAULT_TEST_WORKER_MEMORY_LIMIT_BYTES)
        .trap_on_grow_failure(true)
        .build();
    let mut store = wt::Store::new(engine, TestStoreState { limits });
    store.limiter(|state| &mut state.limits);
    store.set_epoch_deadline(1);
    if let Err(err) = store.set_fuel(DEFAULT_TEST_WORKER_FUEL_LIMIT) {
        return fail_runtime_outcome(format!("fuel_config_error: {}", single_line_error(&err)));
    }

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
                    failure_id: None,
                    reason: None,
                    assertion_diff: None,
                    captured_stdout: String::new(),
                    captured_stderr: String::new(),
                }
            } else {
                TestExecutionOutcome {
                    status: "failed",
                    failure_kind: Some("assertion_false"),
                    failure_code: Some(TEST_ASSERTION_FAILURE_CODE),
                    failure_id: Some("assertion.bool_false".to_string()),
                    reason: Some(format!(
                        "assertion returned false (expected 1/true, got {value})"
                    )),
                    assertion_diff: Some(AssertionDiff {
                        schema_version: 1,
                        kind: "bool_return",
                        expected: "1".to_string(),
                        actual: value.to_string(),
                        expected_failure_code: Some(TEST_ASSERTION_FAILURE_CODE.to_string()),
                        actual_failure_code: Some(TEST_ASSERTION_FAILURE_CODE.to_string()),
                        reason_contains: None,
                        actual_reason: Some(format!("bool return value was {value}")),
                    }),
                    captured_stdout: String::new(),
                    captured_stderr: String::new(),
                }
            }
        }
        Err(err) => {
            if timed_out.load(Ordering::Relaxed) {
                return timeout_failure_outcome(timeout_ms);
            }
            fail_runtime_outcome(classify_runtime_failure(single_line_error(&err).as_str()))
        }
    }
}

fn timeout_failure_outcome(timeout_ms: u64) -> TestExecutionOutcome {
    TestExecutionOutcome {
        status: "failed",
        failure_kind: Some("timeout"),
        failure_code: Some(TEST_TIMEOUT_FAILURE_CODE),
        failure_id: Some("timeout.elapsed".to_string()),
        reason: Some(format!("timeout after {}ms", timeout_ms)),
        assertion_diff: None,
        captured_stdout: String::new(),
        captured_stderr: String::new(),
    }
}

fn fail_runtime_outcome(reason: String) -> TestExecutionOutcome {
    let failure_id = runtime_failure_id(reason.as_str());
    TestExecutionOutcome {
        status: "failed",
        failure_kind: Some("runtime"),
        failure_code: Some(TEST_RUNTIME_FAILURE_CODE),
        failure_id: Some(failure_id.to_string()),
        reason: Some(reason),
        assertion_diff: None,
        captured_stdout: String::new(),
        captured_stderr: String::new(),
    }
}

fn runtime_failure_id(reason: &str) -> &'static str {
    if reason.starts_with("fuel_exhausted:") {
        return "runtime.fuel_exhausted";
    }
    if reason.starts_with("memory_limit:") {
        return "runtime.memory_limit";
    }
    if reason.starts_with("worker_crash:") {
        return "runtime.worker_crash";
    }
    "runtime.trap"
}

fn expected_outcome_label(expected: Option<&TestExpectedOutcomeSpec>) -> &'static str {
    match expected
        .map(|spec| spec.kind)
        .unwrap_or(TestExpectedOutcomeKind::Pass)
    {
        TestExpectedOutcomeKind::Pass => "pass",
        TestExpectedOutcomeKind::AssertionFalse => "assertion_false",
        TestExpectedOutcomeKind::Runtime => "runtime",
        TestExpectedOutcomeKind::Timeout => "timeout",
    }
}

fn enforce_expected_outcome(
    actual: TestExecutionOutcome,
    expected: Option<&TestExpectedOutcomeSpec>,
) -> TestExecutionOutcome {
    let Some(expected) = expected else {
        return actual;
    };
    let expected_kind = expected_outcome_label(Some(expected));
    let actual_kind = if actual.status == "passed" {
        "pass"
    } else {
        actual.failure_kind.unwrap_or("runtime")
    };

    if expected_kind != actual_kind {
        return expectation_mismatch_outcome(
            actual,
            expected,
            "expectation.kind_mismatch",
            format!("expected outcome `{expected_kind}`, observed `{actual_kind}`"),
        );
    }

    if let Some(expected_code) = expected.failure_code.as_deref() {
        let actual_code = actual.failure_code.unwrap_or("<none>");
        if expected_code != actual_code {
            return expectation_mismatch_outcome(
                actual,
                expected,
                "expectation.code_mismatch",
                format!("expected failure_code `{expected_code}`, observed `{actual_code}`"),
            );
        }
    }

    if let Some(needle) = expected.reason_contains.as_deref() {
        let reason = actual.reason.clone().unwrap_or_default();
        if !reason.contains(needle) {
            return expectation_mismatch_outcome(
                actual,
                expected,
                "expectation.reason_mismatch",
                format!("expected reason to contain `{needle}`, observed `{reason}`"),
            );
        }
    }

    TestExecutionOutcome {
        status: "passed",
        failure_kind: None,
        failure_code: None,
        failure_id: None,
        reason: None,
        assertion_diff: None,
        captured_stdout: actual.captured_stdout,
        captured_stderr: actual.captured_stderr,
    }
}

fn expectation_mismatch_outcome(
    actual: TestExecutionOutcome,
    expected: &TestExpectedOutcomeSpec,
    failure_id: &'static str,
    reason: String,
) -> TestExecutionOutcome {
    let expected_kind = expected_outcome_label(Some(expected)).to_string();
    let actual_kind = if actual.status == "passed" {
        "pass".to_string()
    } else {
        actual.failure_kind.unwrap_or("runtime").to_string()
    };

    TestExecutionOutcome {
        status: "failed",
        failure_kind: Some("assertion_mismatch"),
        failure_code: Some(TEST_EXPECTATION_FAILURE_CODE),
        failure_id: Some(failure_id.to_string()),
        reason: Some(reason),
        assertion_diff: Some(AssertionDiff {
            schema_version: 1,
            kind: "expected_outcome",
            expected: expected_kind,
            actual: actual_kind,
            expected_failure_code: expected.failure_code.clone(),
            actual_failure_code: actual.failure_code.map(|code| code.to_string()),
            reason_contains: expected.reason_contains.clone(),
            actual_reason: actual.reason,
        }),
        captured_stdout: actual.captured_stdout,
        captured_stderr: actual.captured_stderr,
    }
}

fn classify_runtime_failure(message: &str) -> String {
    let lowered = message.to_ascii_lowercase();
    if lowered.contains("out of fuel") || lowered.contains("fuel") {
        return format!("fuel_exhausted: {message}");
    }
    if lowered.contains("growing memory")
        || lowered.contains("memory.grow")
        || lowered.contains("memory growth")
        || (lowered.contains("memory") && lowered.contains("grow"))
    {
        return format!("memory_limit: {message}");
    }
    format!("runtime_trap: {message}")
}

fn panic_payload_message(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(msg) = payload.downcast_ref::<&'static str>() {
        return (*msg).to_string();
    }
    if let Some(msg) = payload.downcast_ref::<String>() {
        return msg.clone();
    }
    "panic payload is not a string".to_string()
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
