fn validate_v1_roots(path: &Path, roots: &[RawStrictRootV1]) -> Result<(), StrictLockfileError> {
    let mut seen_root_names = HashSet::with_capacity(roots.len());
    let mut root_name_dupes = BTreeSet::new();
    for root in roots {
        if root.name.trim().is_empty() {
            return Err(StrictLockfileError::new(
                "C104",
                format!(
                    "strict lockfile `{}` has root with empty `name`",
                    path.display()
                ),
            ));
        }
        if !seen_root_names.insert(root.name.clone()) {
            root_name_dupes.insert(root.name.clone());
        }
        let mut seen_dep_names = HashSet::with_capacity(root.dependencies.len());
        let mut dep_name_dupes = BTreeSet::new();
        for dep in &root.dependencies {
            validate_package_id(dep.name.as_str()).map_err(|msg| {
                StrictLockfileError::new(
                    "C104",
                    format!(
                        "strict lockfile `{}` root `{}` has invalid dependency name `{}`: {msg}",
                        path.display(),
                        root.name,
                        dep.name
                    ),
                )
            })?;
            validate_semver_requirement(dep.requirement.as_str()).map_err(|msg| {
                StrictLockfileError::new(
                    "C104",
                    format!(
                        "strict lockfile `{}` root `{}` dependency `{}` has invalid requirement `{}`: {msg}",
                        path.display(),
                        root.name,
                        dep.name,
                        dep.requirement
                    ),
                )
            })?;
            if !seen_dep_names.insert(dep.name.clone()) {
                dep_name_dupes.insert(dep.name.clone());
            }
        }
        if let Some(dupe) = dep_name_dupes.iter().next() {
            return Err(StrictLockfileError::new(
                "C104",
                format!(
                    "strict lockfile `{}` root `{}` has duplicate dependency name `{}`",
                    path.display(),
                    root.name,
                    dupe
                ),
            ));
        }
    }
    if let Some(dupe) = root_name_dupes.iter().next() {
        return Err(StrictLockfileError::new(
            "C104",
            format!(
                "strict lockfile `{}` has duplicate root name `{}`",
                path.display(),
                dupe
            ),
        ));
    }
    Ok(())
}

fn validate_lock_dependencies(
    path: &Path,
    mut dependencies: Vec<StrictLockDependency>,
    allow_multi_version_per_name: bool,
) -> Result<StrictLockfileV0, StrictLockfileError> {
    let mut seen = HashSet::with_capacity(dependencies.len());
    let mut duplicates = BTreeSet::new();
    for dep in &dependencies {
        let key = if allow_multi_version_per_name {
            format!("{}@{}", dep.name, dep.version)
        } else {
            dep.name.clone()
        };
        if !seen.insert(key.clone()) {
            duplicates.insert(key);
        }
    }
    if let Some(duplicate) = duplicates.iter().next() {
        let kind = if allow_multi_version_per_name {
            "dependency id"
        } else {
            "dependency name"
        };
        return Err(StrictLockfileError::new(
            "C104",
            format!(
                "strict lockfile `{}` has duplicate {kind} `{}`",
                path.display(),
                duplicate
            ),
        ));
    }

    if allow_multi_version_per_name {
        dependencies.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.version.cmp(&b.version)));
    } else {
        dependencies.sort_by(|a, b| a.name.cmp(&b.name));
    }
    for dep in &dependencies {
        validate_package_id(&dep.name).map_err(|msg| {
            StrictLockfileError::new(
                "C104",
                format!(
                    "strict lockfile `{}` has invalid dependency name `{}`: {msg}",
                    path.display(),
                    dep.name
                ),
            )
        })?;
        validate_exact_semver(&dep.version).map_err(|msg| {
            StrictLockfileError::new(
                "C104",
                format!(
                    "strict lockfile `{}` has invalid version `{}` for `{}`: {msg}",
                    path.display(),
                    dep.version,
                    dep.name
                ),
            )
        })?;
        validate_sha256_digest(&dep.digest).map_err(|msg| {
            StrictLockfileError::new(
                "C102",
                format!(
                    "strict lockfile `{}` has invalid digest `{}` for `{}`: {msg}",
                    path.display(),
                    dep.digest,
                    dep.name
                ),
            )
        })?;
    }
    Ok(StrictLockfileV0 { dependencies })
}
