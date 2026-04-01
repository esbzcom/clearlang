fn resolve_solver_bin() -> Option<ResolvedSolverBin> {
    if let Some(configured) = configured_solver_bin_from_env() {
        return verify_solver_integrity(configured).map(|path| ResolvedSolverBin { path });
    }
    if let Some(from_bundle_env) = solver_bin_from_bundle_root_env() {
        return verify_solver_integrity(from_bundle_env).map(|path| ResolvedSolverBin { path });
    }
    resolver_default_bundle_candidates()
}

fn configured_solver_bin_from_env() -> Option<PathBuf> {
    let value = std::env::var_os(CLG_SOLVER_BIN_ENV)?;
    if value.is_empty() {
        return None;
    }
    Some(PathBuf::from(value))
}

fn solver_bin_from_bundle_root_env() -> Option<PathBuf> {
    let root = std::env::var_os(CLG_SOLVER_BUNDLE_ROOT_ENV)?;
    if root.is_empty() {
        return None;
    }
    let candidate = PathBuf::from(root).join(solver_bundle_relative_path());
    candidate.is_file().then_some(candidate)
}

fn resolver_default_bundle_candidates() -> Option<ResolvedSolverBin> {
    let relative = solver_bundle_relative_path();
    if let Ok(current_dir) = std::env::current_dir() {
        if let Some(path) = find_solver_in_ancestor_layouts(current_dir.as_path(), relative.as_path()) {
            if let Some(path) = verify_solver_integrity(path) {
                return Some(ResolvedSolverBin { path });
            }
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            if let Some(path) = find_solver_in_ancestor_layouts(exe_dir, relative.as_path()) {
                if let Some(path) = verify_solver_integrity(path) {
                    return Some(ResolvedSolverBin { path });
                }
            }
            let packaged = exe_dir.join("solver").join(relative.as_path());
            if let Some(path) = verify_solver_integrity(packaged) {
                return Some(ResolvedSolverBin { path });
            }
        }
    }
    None
}

fn find_solver_in_ancestor_layouts(
    start: &std::path::Path,
    relative: &std::path::Path,
) -> Option<PathBuf> {
    let solver_name = relative.file_name()?.to_str()?;
    for ancestor in start.ancestors().take(8) {
        let candidate = ancestor.join("tools").join("proof").join("z3").join(relative);
        if candidate.is_file() {
            return Some(candidate);
        }
        if let Some(extracted) = find_solver_in_extracted_bundle_layout(ancestor, solver_name) {
            return Some(extracted);
        }
    }
    None
}

fn find_solver_in_extracted_bundle_layout(
    ancestor: &std::path::Path,
    solver_name: &str,
) -> Option<PathBuf> {
    let extract_root = ancestor
        .join("tools")
        .join("proof")
        .join("z3")
        .join("z3-extract");
    let entries = std::fs::read_dir(extract_root).ok()?;
    let mut dirs = entries
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect::<Vec<_>>();
    // Prefer highest semantic-like version sequence when multiple extracted bundles exist.
    dirs.sort_by(|lhs, rhs| {
        extracted_bundle_version_key(rhs)
            .cmp(&extracted_bundle_version_key(lhs))
            .then_with(|| rhs.to_string_lossy().cmp(&lhs.to_string_lossy()))
    });
    for dir in dirs {
        let candidate = dir.join("bin").join(solver_name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

fn extracted_bundle_version_key(path: &std::path::Path) -> Vec<u32> {
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default();
    let mut nums = Vec::new();
    let mut cur = String::new();
    for ch in name.chars() {
        if ch.is_ascii_digit() {
            cur.push(ch);
        } else if !cur.is_empty() {
            nums.push(cur.parse::<u32>().unwrap_or(0));
            cur.clear();
        }
    }
    if !cur.is_empty() {
        nums.push(cur.parse::<u32>().unwrap_or(0));
    }
    nums
}

fn solver_bundle_relative_path() -> PathBuf {
    let platform = if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else {
        "linux"
    };
    if cfg!(target_os = "windows") {
        PathBuf::from(platform).join("z3.exe")
    } else {
        PathBuf::from(platform).join("z3")
    }
}
