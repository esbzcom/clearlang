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

