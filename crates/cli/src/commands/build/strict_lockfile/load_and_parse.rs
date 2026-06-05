pub(super) fn load_required_strict_lockfile_v0(
    root: &Path,
) -> Result<StrictLockfileV0, StrictLockfileError> {
    let path = root.join(STRICT_LOCKFILE_FILE);
    if !path.exists() {
        return Err(StrictLockfileError::new(
            "C101",
            format!(
                "strict mode requires `{}` at `{}`",
                STRICT_LOCKFILE_FILE,
                path.display()
            ),
        ));
    }
    if !path.is_file() {
        return Err(StrictLockfileError::new(
            "C101",
            format!(
                "strict lockfile path `{}` exists but is not a file",
                path.display()
            ),
        ));
    }

    let content = fs::read_to_string(&path).map_err(|err| {
        StrictLockfileError::new(
            "C101",
            format!("failed to read strict lockfile `{}`: {err}", path.display()),
        )
    })?;

    parse_strict_lockfile_v0(&content, &path)
}

fn parse_strict_lockfile_v0(
    content: &str,
    path: &Path,
) -> Result<StrictLockfileV0, StrictLockfileError> {
    let value: serde_json::Value = serde_json::from_str(content).map_err(|_| {
        StrictLockfileError::new(
            "C104",
            format!(
                "strict lockfile `{}` is not valid JSON (expected schema v0/v1/v2 object)",
                path.display()
            ),
        )
    })?;
    let schema_version = value
        .get("schema_version")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| {
            StrictLockfileError::new(
                "C104",
                format!(
                    "strict lockfile `{}` does not match schema v0/v1/v2 (`schema_version` required)",
                    path.display()
                ),
            )
        })?;

    match schema_version {
        0 => {
            let raw: RawStrictLockfileV0 = serde_json::from_value(value).map_err(|_| {
                StrictLockfileError::new(
                    "C104",
                    format!(
                        "strict lockfile `{}` does not match schema v0 (`schema_version`, `dependencies[]` with `name`, `version`, `digest`)",
                        path.display()
                    ),
                )
            })?;
            debug_assert_eq!(raw.schema_version, 0);
            let dependencies: Vec<StrictLockDependency> = raw
                .dependencies
                .into_iter()
                .map(|dep| StrictLockDependency {
                    name: dep.name,
                    version: dep.version,
                    digest: dep.digest,
                })
                .collect();
            validate_lock_dependencies(path, dependencies, false)
        }
        1 => {
            let raw: RawStrictLockfileV1 = serde_json::from_value(value).map_err(|_| {
                StrictLockfileError::new(
                    "C104",
                    format!(
                        "strict lockfile `{}` does not match schema v1 (`schema_version`, `resolver_version`, `roots[]`, `packages[]`)",
                        path.display()
                    ),
                )
            })?;
            debug_assert_eq!(raw.schema_version, 1);
            if raw.resolver_version != 1 {
                return Err(StrictLockfileError::new(
                    "C104",
                    format!(
                        "strict lockfile `{}` has unsupported resolver_version {}; expected 1",
                        path.display(),
                        raw.resolver_version
                    ),
                ));
            }

            validate_v1_roots(path, &raw.roots)?;

            let mut package_ids = HashSet::with_capacity(raw.packages.len());
            let mut package_id_dupes = BTreeSet::new();
            let mut dependencies = Vec::with_capacity(raw.packages.len());

            for pkg in &raw.packages {
                if !package_ids.insert(pkg.id.clone()) {
                    package_id_dupes.insert(pkg.id.clone());
                }

                validate_package_id(pkg.name.as_str()).map_err(|msg| {
                    StrictLockfileError::new(
                        "C104",
                        format!(
                            "strict lockfile `{}` has invalid package name `{}`: {msg}",
                            path.display(),
                            pkg.name
                        ),
                    )
                })?;
                validate_exact_semver(pkg.version.as_str()).map_err(|msg| {
                    StrictLockfileError::new(
                        "C104",
                        format!(
                            "strict lockfile `{}` has invalid package version `{}` for `{}`: {msg}",
                            path.display(),
                            pkg.version,
                            pkg.name
                        ),
                    )
                })?;
                validate_sha256_digest(pkg.digest.as_str()).map_err(|msg| {
                    StrictLockfileError::new(
                        "C102",
                        format!(
                            "strict lockfile `{}` has invalid package digest `{}` for `{}`: {msg}",
                            path.display(),
                            pkg.digest,
                            pkg.name
                        ),
                    )
                })?;
                if pkg.abi_id.trim().is_empty() {
                    return Err(StrictLockfileError::new(
                        "C104",
                        format!(
                            "strict lockfile `{}` package `{}` has empty abi_id",
                            path.display(),
                            pkg.name
                        ),
                    ));
                }
                let expected_id = format!("{}@{}", pkg.name, pkg.version);
                if pkg.id != expected_id {
                    return Err(StrictLockfileError::new(
                        "C104",
                        format!(
                            "strict lockfile `{}` package id `{}` does not match `name@version` (`{}`)",
                            path.display(),
                            pkg.id,
                            expected_id
                        ),
                    ));
                }
                dependencies.push(StrictLockDependency {
                    name: pkg.name.clone(),
                    version: pkg.version.clone(),
                    digest: pkg.digest.clone(),
                });
            }

            if let Some(dupe) = package_id_dupes.iter().next() {
                return Err(StrictLockfileError::new(
                    "C104",
                    format!(
                        "strict lockfile `{}` has duplicate package id `{}`",
                        path.display(),
                        dupe
                    ),
                ));
            }

            for pkg in &raw.packages {
                let mut seen_deps = HashSet::with_capacity(pkg.dependencies.len());
                let mut dep_dupes = BTreeSet::new();
                for dep in &pkg.dependencies {
                    if dep.trim().is_empty() {
                        return Err(StrictLockfileError::new(
                            "C104",
                            format!(
                                "strict lockfile `{}` package `{}` has empty dependency id",
                                path.display(),
                                pkg.id
                            ),
                        ));
                    }
                    if !seen_deps.insert(dep.clone()) {
                        dep_dupes.insert(dep.clone());
                    }
                    if !package_ids.contains(dep) {
                        return Err(StrictLockfileError::new(
                            "C104",
                            format!(
                                "strict lockfile `{}` package `{}` references dependency id `{}` that is missing from `packages[]`",
                                path.display(),
                                pkg.id,
                                dep
                            ),
                        ));
                    }
                }
                if let Some(dupe) = dep_dupes.iter().next() {
                    return Err(StrictLockfileError::new(
                        "C104",
                        format!(
                            "strict lockfile `{}` package `{}` has duplicate dependency id `{}`",
                            path.display(),
                            pkg.id,
                            dupe
                        ),
                    ));
                }
            }

            validate_lock_dependencies(path, dependencies, true)
        }
        2 => {
            let raw: RawStrictLockfileV2 = serde_json::from_value(value).map_err(|_| {
                StrictLockfileError::new(
                    "C104",
                    format!(
                        "strict lockfile `{}` does not match schema v2 (`schema_version`, `resolver_version`, `roots[]`, `packages[]`, `std`)",
                        path.display()
                    ),
                )
            })?;
            debug_assert_eq!(raw.schema_version, 2);
            if raw.resolver_version != 1 {
                return Err(StrictLockfileError::new(
                    "C104",
                    format!(
                        "strict lockfile `{}` has unsupported resolver_version {}; expected 1",
                        path.display(),
                        raw.resolver_version
                    ),
                ));
            }
            if raw.std.delivery.trim().is_empty() {
                return Err(StrictLockfileError::new(
                    "C104",
                    format!(
                        "strict lockfile `{}` schema v2 `std.delivery` must be non-empty",
                        path.display()
                    ),
                ));
            }
            validate_v1_roots(path, &raw.roots)?;

            let mut package_ids = HashSet::with_capacity(raw.packages.len());
            let mut package_id_dupes = BTreeSet::new();
            let mut dependencies = Vec::with_capacity(raw.packages.len() + raw.std.packages.len());

            for pkg in &raw.packages {
                if !package_ids.insert(pkg.id.clone()) {
                    package_id_dupes.insert(pkg.id.clone());
                }

                validate_package_id(pkg.name.as_str()).map_err(|msg| {
                    StrictLockfileError::new(
                        "C104",
                        format!(
                            "strict lockfile `{}` has invalid package name `{}`: {msg}",
                            path.display(),
                            pkg.name
                        ),
                    )
                })?;
                validate_exact_semver(pkg.version.as_str()).map_err(|msg| {
                    StrictLockfileError::new(
                        "C104",
                        format!(
                            "strict lockfile `{}` has invalid package version `{}` for `{}`: {msg}",
                            path.display(),
                            pkg.version,
                            pkg.name
                        ),
                    )
                })?;
                validate_sha256_digest(pkg.digest.as_str()).map_err(|msg| {
                    StrictLockfileError::new(
                        "C102",
                        format!(
                            "strict lockfile `{}` has invalid package digest `{}` for `{}`: {msg}",
                            path.display(),
                            pkg.digest,
                            pkg.name
                        ),
                    )
                })?;
                if pkg.abi_id.trim().is_empty() {
                    return Err(StrictLockfileError::new(
                        "C104",
                        format!(
                            "strict lockfile `{}` package `{}` has empty abi_id",
                            path.display(),
                            pkg.name
                        ),
                    ));
                }
                let expected_id = format!("{}@{}", pkg.name, pkg.version);
                if pkg.id != expected_id {
                    return Err(StrictLockfileError::new(
                        "C104",
                        format!(
                            "strict lockfile `{}` package id `{}` does not match `name@version` (`{}`)",
                            path.display(),
                            pkg.id,
                            expected_id
                        ),
                    ));
                }
                dependencies.push(StrictLockDependency {
                    name: pkg.name.clone(),
                    version: pkg.version.clone(),
                    digest: pkg.digest.clone(),
                });
            }

            for shared_pkg in &raw.std.packages {
                validate_package_id(shared_pkg.package_id.as_str()).map_err(|msg| {
                    StrictLockfileError::new(
                        "C104",
                        format!(
                            "strict lockfile `{}` has invalid shared std package_id `{}`: {msg}",
                            path.display(),
                            shared_pkg.package_id
                        ),
                    )
                })?;
                validate_exact_semver(shared_pkg.version.as_str()).map_err(|msg| {
                    StrictLockfileError::new(
                        "C104",
                        format!(
                            "strict lockfile `{}` has invalid shared std version `{}` for `{}`: {msg}",
                            path.display(),
                            shared_pkg.version,
                            shared_pkg.package_id
                        ),
                    )
                })?;
                validate_sha256_digest(shared_pkg.artifact.digest.as_str()).map_err(|msg| {
                    StrictLockfileError::new(
                        "C102",
                        format!(
                            "strict lockfile `{}` has invalid shared std digest `{}` for `{}`: {msg}",
                            path.display(),
                            shared_pkg.artifact.digest,
                            shared_pkg.package_id
                        ),
                    )
                })?;
                dependencies.push(StrictLockDependency {
                    name: shared_pkg.package_id.clone(),
                    version: shared_pkg.version.clone(),
                    digest: shared_pkg.artifact.digest.clone(),
                });
            }

            if let Some(dupe) = package_id_dupes.iter().next() {
                return Err(StrictLockfileError::new(
                    "C104",
                    format!(
                        "strict lockfile `{}` has duplicate package id `{}`",
                        path.display(),
                        dupe
                    ),
                ));
            }

            for pkg in &raw.packages {
                let mut seen_deps = HashSet::with_capacity(pkg.dependencies.len());
                let mut dep_dupes = BTreeSet::new();
                for dep in &pkg.dependencies {
                    if dep.trim().is_empty() {
                        return Err(StrictLockfileError::new(
                            "C104",
                            format!(
                                "strict lockfile `{}` package `{}` has empty dependency id",
                                path.display(),
                                pkg.id
                            ),
                        ));
                    }
                    if !seen_deps.insert(dep.clone()) {
                        dep_dupes.insert(dep.clone());
                    }
                    if !package_ids.contains(dep) {
                        return Err(StrictLockfileError::new(
                            "C104",
                            format!(
                                "strict lockfile `{}` package `{}` references dependency id `{}` that is missing from `packages[]`",
                                path.display(),
                                pkg.id,
                                dep
                            ),
                        ));
                    }
                }
                if let Some(dupe) = dep_dupes.iter().next() {
                    return Err(StrictLockfileError::new(
                        "C104",
                        format!(
                            "strict lockfile `{}` package `{}` has duplicate dependency id `{}`",
                            path.display(),
                            pkg.id,
                            dupe
                        ),
                    ));
                }
            }

            validate_lock_dependencies(path, dependencies, true)
        }
        other => Err(StrictLockfileError::new(
            "C104",
            format!(
                "strict lockfile `{}` has unsupported schema_version {}; expected 0, 1, or 2",
                path.display(),
                other
            ),
        )),
    }
}
