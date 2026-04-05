fn production_release_surface_violation(program: &Program) -> Result<Option<String>> {
    let bundle_symbols = bundle_symbols_for_program(program);
    if bundle_symbols.is_empty() {
        return Ok(None);
    }
    let matrix_path = proof_matrix_path_from_env();
    let proved_allowlist = load_proved_surface_allowlist(&matrix_path).map_err(|err| {
        anyhow!(
            "release profile `production` surface allowlist load failed for `{}`: {err:#}",
            matrix_path.display()
        )
    })?;
    let disallowed: Vec<String> = bundle_symbols
        .into_iter()
        .filter(|symbol| !proved_allowlist.contains(symbol))
        .collect();
    if disallowed.is_empty() {
        return Ok(None);
    }
    Ok(Some(format!(
        "release profile `production` only permits proved std surfaces from `{}`; disallowed [{}]",
        matrix_path.display(),
        disallowed.join(", ")
    )))
}

fn strict_import_map_source_files(module_root: &Path, source_files: &[PathBuf]) -> Vec<String> {
    let canonical_root = module_root
        .canonicalize()
        .unwrap_or_else(|_| module_root.to_path_buf());
    let mut out = source_files
        .iter()
        .map(|source| {
            let rel = source.strip_prefix(canonical_root.as_path()).unwrap_or(source);
            normalize_path_for_report(rel)
        })
        .collect::<Vec<_>>();
    out.sort();
    out.dedup();
    out
}

fn release_module_graph_test_path_violation(
    module_root: &Path,
    source_files: &[PathBuf],
) -> Option<String> {
    let canonical_root = module_root
        .canonicalize()
        .unwrap_or_else(|_| module_root.to_path_buf());
    let mut offending = source_files
        .iter()
        .filter_map(|source| {
            let rel = source.strip_prefix(canonical_root.as_path()).unwrap_or(source);
            has_tests_or_mocks_segment(rel).then(|| normalize_path_for_report(rel))
        })
        .collect::<Vec<_>>();
    offending.sort();
    offending.dedup();
    if offending.is_empty() {
        return None;
    }
    Some(format!(
        "release profile `production` forbids module-graph references to `tests/` or `tests/mocks/`; found [{}]",
        offending.join(", ")
    ))
}

fn has_tests_or_mocks_segment(path: &Path) -> bool {
    let mut segments = path
        .components()
        .map(|component| component.as_os_str().to_string_lossy().to_string())
        .collect::<Vec<_>>();
    if segments.is_empty() {
        return false;
    }
    segments.iter_mut().for_each(|segment| {
        *segment = segment.to_ascii_lowercase();
    });
    for window in segments.windows(2) {
        if window[0] == "tests" {
            return true;
        }
    }
    segments.iter().any(|segment| segment == "tests")
}

fn normalize_path_for_report(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}
