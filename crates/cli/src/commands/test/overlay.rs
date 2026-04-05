fn build_execution_engine() -> Result<wt::Engine> {
    let mut config = wt::Config::new();
    config.epoch_interruption(true);
    config.consume_fuel(true);
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
        let compile_result = (|| {
            let harness = write_case_harness(
                overlay_root.as_path(),
                project_root.as_path(),
                case,
                json_errors,
            )?;
            compile_test_module(
                engine,
                harness.as_path(),
                std::slice::from_ref(&case.function),
                json_errors,
            )
        })();
        let cleanup_result = cleanup_overlay_root(overlay_root.as_path(), json_errors);

        match (compile_result, cleanup_result) {
            (Ok(module), Ok(())) => modules.push(module),
            (Ok(_), Err(cleanup_err)) => return Err(cleanup_err),
            (Err(compile_err), Ok(())) => return Err(compile_err),
            (Err(compile_err), Err(_cleanup_err)) => {
                // Preserve deterministic compile diagnostics when both compile and cleanup fail.
                return Err(compile_err);
            }
        }
    }
    Ok(modules)
}

fn cleanup_overlay_root(overlay_root: &Path, json_errors: bool) -> Result<()> {
    match fs::remove_dir_all(overlay_root) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == ErrorKind::NotFound => Ok(()),
        Err(err) => Err(test_error(
            TEST_DISCOVERY_ERROR_CODE,
            format!("removing `{}`: {err}", overlay_root.display()),
            overlay_root,
            0,
            0,
            json_errors,
        )
        .into()),
    }
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

