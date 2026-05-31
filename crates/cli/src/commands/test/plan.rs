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
        if let Some(expected) = case.expected_outcome.as_ref() {
            validate_expected_outcome_spec(&plan_path, case.test_id.as_str(), expected, json_errors)?;
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
        let (timeout_ms, mock_sets, expected_outcome) = if let Some(plan) = plan {
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
            let expected_outcome = case_plan.and_then(|entry| entry.expected_outcome.clone());
            (timeout_ms, mock_sets, expected_outcome)
        } else {
            (DEFAULT_TEST_TIMEOUT_MS, Vec::new(), None)
        };

        out.push(TestExecutionConfig {
            timeout_ms,
            mock_sets,
            expected_outcome,
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

fn validate_expected_outcome_spec(
    plan_path: &Path,
    test_id: &str,
    expected: &TestExpectedOutcomeSpec,
    json_errors: bool,
) -> Result<()> {
    if expected.kind == TestExpectedOutcomeKind::Pass
        && (expected.failure_code.is_some() || expected.reason_contains.is_some())
    {
        return Err(test_error(
            TEST_PLAN_ERROR_CODE,
            format!(
                "`{}` case `{}` has invalid expected_outcome: `pass` must not set `failure_code` or `reason_contains`",
                plan_path.display(),
                test_id
            ),
            plan_path,
            0,
            0,
            json_errors,
        )
        .into());
    }
    if let Some(code) = expected.failure_code.as_deref() {
        if !is_valid_failure_code_token(code) {
            return Err(test_error(
                TEST_PLAN_ERROR_CODE,
                format!(
                    "`{}` case `{}` has invalid expected_outcome.failure_code `{}`",
                    plan_path.display(),
                    test_id,
                    code
                ),
                plan_path,
                0,
                0,
                json_errors,
            )
            .into());
        }
    }
    if let Some(needle) = expected.reason_contains.as_deref() {
        if needle.trim().is_empty() || needle.contains('\n') || needle.contains('\r') {
            return Err(test_error(
                TEST_PLAN_ERROR_CODE,
                format!(
                    "`{}` case `{}` has invalid expected_outcome.reason_contains; expected single-line non-empty substring",
                    plan_path.display(),
                    test_id
                ),
                plan_path,
                0,
                0,
                json_errors,
            )
            .into());
        }
    }
    Ok(())
}

fn is_valid_failure_code_token(code: &str) -> bool {
    let bytes = code.as_bytes();
    bytes.len() == 4
        && bytes[0] == b'C'
        && bytes[1].is_ascii_digit()
        && bytes[2].is_ascii_digit()
        && bytes[3].is_ascii_digit()
}

