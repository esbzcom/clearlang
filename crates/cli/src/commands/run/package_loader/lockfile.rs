fn load_runtime_lockfile_evidence(
    root: &Path,
) -> Result<RuntimeLockfileEvidence, RuntimePackageLoaderError> {
    let path = root.join(STRICT_LOCKFILE_FILE);
    if !path.exists() {
        return Err(RuntimePackageLoaderError::new(
            "R013",
            format!(
                "runtime trust gate requires `{}` at `{}`",
                STRICT_LOCKFILE_FILE,
                path.display()
            ),
        ));
    }
    if !path.is_file() {
        return Err(RuntimePackageLoaderError::new(
            "R013",
            format!(
                "runtime lockfile path `{}` exists but is not a file",
                path.display()
            ),
        ));
    }
    let content = fs::read_to_string(&path).map_err(|err| {
        RuntimePackageLoaderError::new(
            "R013",
            format!(
                "reading runtime lockfile `{}` failed (io_kind={})",
                path.display(),
                io_error_kind_label(&err)
            ),
        )
    })?;
    let value: serde_json::Value = serde_json::from_str(content.as_str()).map_err(|err| {
        RuntimePackageLoaderError::new(
            "R013",
            format!(
                "runtime lockfile `{}` is not valid JSON (expected schema v0/v1; json_class={})",
                path.display(),
                json_error_class_label(&err)
            ),
        )
    })?;
    let schema_version = value
        .get("schema_version")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| {
            RuntimePackageLoaderError::new(
                "R013",
                format!(
                    "runtime lockfile `{}` is missing required `schema_version`",
                    path.display()
                ),
            )
        })?;

    match schema_version {
        0 => {
            let raw: RawStrictLockfileV0 = serde_json::from_value(value).map_err(|err| {
                let _ = err;
                RuntimePackageLoaderError::new(
                    "R013",
                    format!(
                        "runtime lockfile `{}` does not match schema v0 (`dependencies[]`)",
                        path.display()
                    ),
                )
            })?;
            debug_assert_eq!(raw.schema_version, 0);
            let mut digests_by_id = HashMap::with_capacity(raw.dependencies.len());
            for dep in raw.dependencies {
                validate_package_id(dep.name.as_str()).map_err(|msg| {
                    RuntimePackageLoaderError::new(
                        "R013",
                        format!(
                            "runtime lockfile `{}` has invalid dependency name `{}`: {msg}",
                            path.display(),
                            dep.name
                        ),
                    )
                })?;
                validate_exact_semver(dep.version.as_str()).map_err(|msg| {
                    RuntimePackageLoaderError::new(
                        "R013",
                        format!(
                            "runtime lockfile `{}` has invalid dependency version `{}` for `{}`: {msg}",
                            path.display(),
                            dep.version,
                            dep.name
                        ),
                    )
                })?;
                validate_sha256_digest(dep.digest.as_str()).map_err(|msg| {
                    RuntimePackageLoaderError::new(
                        "R013",
                        format!(
                            "runtime lockfile `{}` has invalid dependency digest `{}` for `{}`: {msg}",
                            path.display(),
                            dep.digest,
                            dep.name
                        ),
                    )
                })?;
                let id = format!("{}@{}", dep.name, dep.version);
                if let Some(existing) = digests_by_id.insert(id.clone(), dep.digest.clone()) {
                    return Err(RuntimePackageLoaderError::new(
                        "R013",
                        format!(
                            "runtime lockfile `{}` has duplicate package id `{}` (digests: `{}` vs `{}`)",
                            path.display(),
                            id,
                            existing,
                            dep.digest
                        ),
                    ));
                }
            }
            Ok(RuntimeLockfileEvidence {
                resolver_version: None,
                digests_by_id,
                shared_std_by_id: HashMap::new(),
            })
        }
        1 => {
            let raw: RawStrictLockfileV1 = serde_json::from_value(value).map_err(|err| {
                let _ = err;
                RuntimePackageLoaderError::new(
                    "R013",
                    format!(
                        "runtime lockfile `{}` does not match schema v1 (`resolver_version`, `roots[]`, `packages[]`)",
                        path.display()
                    ),
                )
            })?;
            debug_assert_eq!(raw.schema_version, 1);
            if raw.resolver_version != 1 {
                return Err(RuntimePackageLoaderError::new(
                    "R013",
                    format!(
                        "runtime lockfile `{}` has unsupported resolver_version {}; expected 1",
                        path.display(),
                        raw.resolver_version
                    ),
                ));
            }
            let mut package_versions_by_name: HashMap<String, BTreeSet<String>> =
                HashMap::with_capacity(raw.packages.len());
            let mut package_dependencies: Vec<(String, Vec<String>)> =
                Vec::with_capacity(raw.packages.len());

            for root in &raw.roots {
                if root.name.trim().is_empty() {
                    return Err(RuntimePackageLoaderError::new(
                        "R013",
                        format!(
                            "runtime lockfile `{}` has root with empty name",
                            path.display()
                        ),
                    ));
                }
                for dep in &root.dependencies {
                    validate_package_id(dep.name.as_str()).map_err(|msg| {
                        RuntimePackageLoaderError::new(
                            "R013",
                            format!(
                                "runtime lockfile `{}` root `{}` has invalid dependency name `{}`: {msg}",
                                path.display(),
                                root.name,
                                dep.name
                            ),
                        )
                    })?;
                    validate_semver_requirement(dep.requirement.as_str()).map_err(|msg| {
                        RuntimePackageLoaderError::new(
                            "R013",
                            format!(
                                "runtime lockfile `{}` root `{}` dependency `{}` has invalid requirement `{}`: {msg}",
                                path.display(),
                                root.name,
                                dep.name,
                                dep.requirement
                            ),
                        )
                    })?;
                }
            }

            let mut digests_by_id = HashMap::with_capacity(raw.packages.len());
            for pkg in raw.packages {
                validate_package_id(pkg.name.as_str()).map_err(|msg| {
                    RuntimePackageLoaderError::new(
                        "R013",
                        format!(
                            "runtime lockfile `{}` has invalid package name `{}`: {msg}",
                            path.display(),
                            pkg.name
                        ),
                    )
                })?;
                validate_exact_semver(pkg.version.as_str()).map_err(|msg| {
                    RuntimePackageLoaderError::new(
                        "R013",
                        format!(
                            "runtime lockfile `{}` has invalid package version `{}` for `{}`: {msg}",
                            path.display(),
                            pkg.version,
                            pkg.name
                        ),
                    )
                })?;
                validate_sha256_digest(pkg.digest.as_str()).map_err(|msg| {
                    RuntimePackageLoaderError::new(
                        "R013",
                        format!(
                            "runtime lockfile `{}` has invalid package digest `{}` for `{}`: {msg}",
                            path.display(),
                            pkg.digest,
                            pkg.name
                        ),
                    )
                })?;
                if pkg.abi_id.trim().is_empty() {
                    return Err(RuntimePackageLoaderError::new(
                        "R013",
                        format!(
                            "runtime lockfile `{}` package `{}` has empty abi_id",
                            path.display(),
                            pkg.id
                        ),
                    ));
                }
                let expected_id = format!("{}@{}", pkg.name, pkg.version);
                if pkg.id != expected_id {
                    return Err(RuntimePackageLoaderError::new(
                        "R013",
                        format!(
                            "runtime lockfile `{}` package id `{}` does not match `name@version` (`{}`)",
                            path.display(),
                            pkg.id,
                            expected_id
                        ),
                    ));
                }
                for dep in &pkg.dependencies {
                    validate_runtime_package_id(dep.as_str()).map_err(|msg| {
                        RuntimePackageLoaderError::new(
                            "R013",
                            format!(
                                "runtime lockfile `{}` package `{}` has invalid dependency id `{}`: {msg}",
                                path.display(),
                                pkg.id,
                                dep
                            ),
                        )
                    })?;
                }
                if let Some(existing) = digests_by_id.insert(pkg.id.clone(), pkg.digest.clone()) {
                    return Err(RuntimePackageLoaderError::new(
                        "R013",
                        format!(
                            "runtime lockfile `{}` has duplicate package id `{}` (digests: `{}` vs `{}`)",
                            path.display(),
                            pkg.id,
                            existing,
                            pkg.digest
                        ),
                    ));
                }
                package_versions_by_name
                    .entry(pkg.name.clone())
                    .or_default()
                    .insert(pkg.version.clone());
                package_dependencies.push((pkg.id.clone(), pkg.dependencies.clone()));
            }

            for root in &raw.roots {
                let mut seen_dependency_names = HashSet::with_capacity(root.dependencies.len());
                let mut duplicate_dependency_names = BTreeSet::new();
                for dep in &root.dependencies {
                    if !seen_dependency_names.insert(dep.name.clone()) {
                        duplicate_dependency_names.insert(dep.name.clone());
                    }
                    let matching_versions = package_versions_by_name
                        .get(dep.name.as_str())
                        .ok_or_else(|| {
                            RuntimePackageLoaderError::new(
                                "R013",
                                format!(
                                    "runtime lockfile `{}` root `{}` dependency `{}` is missing from `packages[]`",
                                    path.display(),
                                    root.name,
                                    dep.name
                                ),
                            )
                        })?;
                    if !matching_versions.iter().any(|version| {
                        requirement_matches_version(dep.requirement.as_str(), version.as_str())
                    }) {
                        return Err(RuntimePackageLoaderError::new(
                            "R013",
                            format!(
                                "runtime lockfile `{}` root `{}` dependency `{}` requirement `{}` is not satisfied by `packages[]`",
                                path.display(),
                                root.name,
                                dep.name,
                                dep.requirement
                            ),
                        ));
                    }
                }
                if let Some(dupe) = duplicate_dependency_names.iter().next() {
                    return Err(RuntimePackageLoaderError::new(
                        "R013",
                        format!(
                            "runtime lockfile `{}` root `{}` has duplicate dependency name `{}`",
                            path.display(),
                            root.name,
                            dupe
                        ),
                    ));
                }
            }

            for (package_id, dependencies) in package_dependencies {
                let mut seen_dependencies = HashSet::with_capacity(dependencies.len());
                let mut duplicate_dependencies = BTreeSet::new();
                for dep in dependencies {
                    if !seen_dependencies.insert(dep.clone()) {
                        duplicate_dependencies.insert(dep.clone());
                    }
                    if !digests_by_id.contains_key(dep.as_str()) {
                        return Err(RuntimePackageLoaderError::new(
                            "R013",
                            format!(
                                "runtime lockfile `{}` package `{}` references dependency id `{}` that is missing from `packages[]`",
                                path.display(),
                                package_id,
                                dep
                            ),
                        ));
                    }
                }
                if let Some(dupe) = duplicate_dependencies.iter().next() {
                    return Err(RuntimePackageLoaderError::new(
                        "R013",
                        format!(
                            "runtime lockfile `{}` package `{}` has duplicate dependency id `{}`",
                            path.display(),
                            package_id,
                            dupe
                        ),
                    ));
                }
            }
            Ok(RuntimeLockfileEvidence {
                resolver_version: Some(raw.resolver_version),
                digests_by_id,
                shared_std_by_id: HashMap::new(),
            })
        }
        2 => {
            let raw: RawStrictLockfileV2 = serde_json::from_value(value).map_err(|err| {
                let _ = err;
                RuntimePackageLoaderError::new(
                    "R013",
                    format!(
                        "runtime lockfile `{}` does not match schema v2 (`resolver_version`, `roots[]`, `packages[]`, `std`)",
                        path.display()
                    ),
                )
            })?;
            debug_assert_eq!(raw.schema_version, 2);
            if raw.resolver_version != 1 {
                return Err(RuntimePackageLoaderError::new(
                    "R013",
                    format!(
                        "runtime lockfile `{}` has unsupported resolver_version {}; expected 1",
                        path.display(),
                        raw.resolver_version
                    ),
                ));
            }

            let mut package_versions_by_name: HashMap<String, BTreeSet<String>> =
                HashMap::with_capacity(raw.packages.len());
            let mut package_dependencies: Vec<(String, Vec<String>)> =
                Vec::with_capacity(raw.packages.len());
            let mut digests_by_id = HashMap::with_capacity(raw.packages.len());

            for root in &raw.roots {
                if root.name.trim().is_empty() {
                    return Err(RuntimePackageLoaderError::new(
                        "R013",
                        format!(
                            "runtime lockfile `{}` has root with empty name",
                            path.display()
                        ),
                    ));
                }
                for dep in &root.dependencies {
                    validate_package_id(dep.name.as_str()).map_err(|msg| {
                        RuntimePackageLoaderError::new(
                            "R013",
                            format!(
                                "runtime lockfile `{}` root `{}` has invalid dependency name `{}`: {msg}",
                                path.display(),
                                root.name,
                                dep.name
                            ),
                        )
                    })?;
                    validate_semver_requirement(dep.requirement.as_str()).map_err(|msg| {
                        RuntimePackageLoaderError::new(
                            "R013",
                            format!(
                                "runtime lockfile `{}` root `{}` dependency `{}` has invalid requirement `{}`: {msg}",
                                path.display(),
                                root.name,
                                dep.name,
                                dep.requirement
                            ),
                        )
                    })?;
                }
            }

            for pkg in raw.packages {
                validate_package_id(pkg.name.as_str()).map_err(|msg| {
                    RuntimePackageLoaderError::new(
                        "R013",
                        format!(
                            "runtime lockfile `{}` has invalid package name `{}`: {msg}",
                            path.display(),
                            pkg.name
                        ),
                    )
                })?;
                validate_exact_semver(pkg.version.as_str()).map_err(|msg| {
                    RuntimePackageLoaderError::new(
                        "R013",
                        format!(
                            "runtime lockfile `{}` has invalid package version `{}` for `{}`: {msg}",
                            path.display(),
                            pkg.version,
                            pkg.name
                        ),
                    )
                })?;
                validate_sha256_digest(pkg.digest.as_str()).map_err(|msg| {
                    RuntimePackageLoaderError::new(
                        "R013",
                        format!(
                            "runtime lockfile `{}` has invalid package digest `{}` for `{}`: {msg}",
                            path.display(),
                            pkg.digest,
                            pkg.name
                        ),
                    )
                })?;
                if pkg.abi_id.trim().is_empty() {
                    return Err(RuntimePackageLoaderError::new(
                        "R013",
                        format!(
                            "runtime lockfile `{}` package `{}` has empty abi_id",
                            path.display(),
                            pkg.id
                        ),
                    ));
                }
                let expected_id = format!("{}@{}", pkg.name, pkg.version);
                if pkg.id != expected_id {
                    return Err(RuntimePackageLoaderError::new(
                        "R013",
                        format!(
                            "runtime lockfile `{}` package id `{}` does not match `name@version` (`{}`)",
                            path.display(),
                            pkg.id,
                            expected_id
                        ),
                    ));
                }
                for dep in &pkg.dependencies {
                    validate_runtime_package_id(dep.as_str()).map_err(|msg| {
                        RuntimePackageLoaderError::new(
                            "R013",
                            format!(
                                "runtime lockfile `{}` package `{}` has invalid dependency id `{}`: {msg}",
                                path.display(),
                                pkg.id,
                                dep
                            ),
                        )
                    })?;
                }
                if let Some(existing) = digests_by_id.insert(pkg.id.clone(), pkg.digest.clone()) {
                    return Err(RuntimePackageLoaderError::new(
                        "R013",
                        format!(
                            "runtime lockfile `{}` has duplicate package id `{}` (digests: `{}` vs `{}`)",
                            path.display(),
                            pkg.id,
                            existing,
                            pkg.digest
                        ),
                    ));
                }
                package_versions_by_name
                    .entry(pkg.name.clone())
                    .or_default()
                    .insert(pkg.version.clone());
                package_dependencies.push((pkg.id.clone(), pkg.dependencies.clone()));
            }

            for root in &raw.roots {
                let mut seen_dependency_names = HashSet::with_capacity(root.dependencies.len());
                let mut duplicate_dependency_names = BTreeSet::new();
                for dep in &root.dependencies {
                    if !seen_dependency_names.insert(dep.name.clone()) {
                        duplicate_dependency_names.insert(dep.name.clone());
                    }
                    let matching_versions = package_versions_by_name
                        .get(dep.name.as_str())
                        .ok_or_else(|| {
                            RuntimePackageLoaderError::new(
                                "R013",
                                format!(
                                    "runtime lockfile `{}` root `{}` dependency `{}` is missing from `packages[]`",
                                    path.display(),
                                    root.name,
                                    dep.name
                                ),
                            )
                        })?;
                    if !matching_versions.iter().any(|version| {
                        requirement_matches_version(dep.requirement.as_str(), version.as_str())
                    }) {
                        return Err(RuntimePackageLoaderError::new(
                            "R013",
                            format!(
                                "runtime lockfile `{}` root `{}` dependency `{}` requirement `{}` is not satisfied by `packages[]`",
                                path.display(),
                                root.name,
                                dep.name,
                                dep.requirement
                            ),
                        ));
                    }
                }
                if let Some(dupe) = duplicate_dependency_names.iter().next() {
                    return Err(RuntimePackageLoaderError::new(
                        "R013",
                        format!(
                            "runtime lockfile `{}` root `{}` has duplicate dependency name `{}`",
                            path.display(),
                            root.name,
                            dupe
                        ),
                    ));
                }
            }

            for (package_id, dependencies) in package_dependencies {
                let mut seen_dependencies = HashSet::with_capacity(dependencies.len());
                let mut duplicate_dependencies = BTreeSet::new();
                for dep in dependencies {
                    if !seen_dependencies.insert(dep.clone()) {
                        duplicate_dependencies.insert(dep.clone());
                    }
                    if !digests_by_id.contains_key(dep.as_str()) {
                        return Err(RuntimePackageLoaderError::new(
                            "R013",
                            format!(
                                "runtime lockfile `{}` package `{}` references dependency id `{}` that is missing from `packages[]`",
                                path.display(),
                                package_id,
                                dep
                            ),
                        ));
                    }
                }
                if let Some(dupe) = duplicate_dependencies.iter().next() {
                    return Err(RuntimePackageLoaderError::new(
                        "R013",
                        format!(
                            "runtime lockfile `{}` package `{}` has duplicate dependency id `{}`",
                            path.display(),
                            package_id,
                            dupe
                        ),
                    ));
                }
            }

            let delivery = raw.std.delivery.as_str();
            if delivery != "embedded" && delivery != "shared" {
                return Err(RuntimePackageLoaderError::new(
                    "R013",
                    format!(
                        "runtime lockfile `{}` has invalid `std.delivery` `{}`; expected `embedded` or `shared`",
                        path.display(),
                        raw.std.delivery
                    ),
                ));
            }
            if delivery == "embedded" && !raw.std.packages.is_empty() {
                return Err(RuntimePackageLoaderError::new(
                    "R013",
                    format!(
                        "runtime lockfile `{}` declares `std.delivery = embedded` but also contains `std.packages[]`",
                        path.display()
                    ),
                ));
            }
            if delivery == "shared" && raw.std.packages.is_empty() {
                return Err(RuntimePackageLoaderError::new(
                    "R013",
                    format!(
                        "runtime lockfile `{}` declares `std.delivery = shared` but `std.packages[]` is empty",
                        path.display()
                    ),
                ));
            }

            let mut shared_std_by_id = HashMap::with_capacity(raw.std.packages.len());
            let mut previous_id: Option<String> = None;
            for shared_pkg in raw.std.packages {
                validate_package_id(shared_pkg.package_id.as_str()).map_err(|msg| {
                    RuntimePackageLoaderError::new(
                        "R013",
                        format!(
                            "runtime lockfile `{}` has invalid shared std package_id `{}`: {msg}",
                            path.display(),
                            shared_pkg.package_id
                        ),
                    )
                })?;
                validate_exact_semver(shared_pkg.version.as_str()).map_err(|msg| {
                    RuntimePackageLoaderError::new(
                        "R013",
                        format!(
                            "runtime lockfile `{}` has invalid shared std version `{}` for `{}`: {msg}",
                            path.display(),
                            shared_pkg.version,
                            shared_pkg.package_id
                        ),
                    )
                })?;
                if shared_pkg.verified_std_abi.minor_max < shared_pkg.verified_std_abi.minor_min {
                    return Err(RuntimePackageLoaderError::new(
                        "R013",
                        format!(
                            "runtime lockfile `{}` shared std package `{}` has invalid verified_std_abi range {}.{}..{}",
                            path.display(),
                            shared_pkg.package_id,
                            shared_pkg.verified_std_abi.major,
                            shared_pkg.verified_std_abi.minor_min,
                            shared_pkg.verified_std_abi.minor_max
                        ),
                    ));
                }
                if shared_pkg.artifact.format.trim().is_empty() {
                    return Err(RuntimePackageLoaderError::new(
                        "R013",
                        format!(
                            "runtime lockfile `{}` shared std package `{}` has empty artifact format",
                            path.display(),
                            shared_pkg.package_id
                        ),
                    ));
                }
                validate_relative_artifact_path(shared_pkg.artifact.path.as_str()).map_err(
                    |msg| {
                        RuntimePackageLoaderError::new(
                            "R013",
                            format!(
                                "runtime lockfile `{}` shared std package `{}` has invalid artifact path `{}`: {msg}",
                                path.display(),
                                shared_pkg.package_id,
                                shared_pkg.artifact.path
                            ),
                        )
                    },
                )?;
                if shared_pkg.artifact.size_bytes == 0 {
                    return Err(RuntimePackageLoaderError::new(
                        "R013",
                        format!(
                            "runtime lockfile `{}` shared std package `{}` has zero artifact size_bytes",
                            path.display(),
                            shared_pkg.package_id
                        ),
                    ));
                }
                validate_sha256_digest(shared_pkg.artifact.digest.as_str()).map_err(|msg| {
                    RuntimePackageLoaderError::new(
                        "R013",
                        format!(
                            "runtime lockfile `{}` shared std package `{}` has invalid artifact digest `{}`: {msg}",
                            path.display(),
                            shared_pkg.package_id,
                            shared_pkg.artifact.digest
                        ),
                    )
                })?;
                if shared_pkg.signature.key_id.trim().is_empty()
                    || shared_pkg.signature.algorithm.trim().is_empty()
                    || shared_pkg.signature.signed_at.trim().is_empty()
                    || shared_pkg.signature.signature.trim().is_empty()
                {
                    return Err(RuntimePackageLoaderError::new(
                        "R013",
                        format!(
                            "runtime lockfile `{}` shared std package `{}` has incomplete signature fields",
                            path.display(),
                            shared_pkg.package_id
                        ),
                    ));
                }
                validate_sha256_digest(shared_pkg.provenance.statement_digest.as_str()).map_err(
                    |msg| {
                        RuntimePackageLoaderError::new(
                            "R013",
                            format!(
                                "runtime lockfile `{}` shared std package `{}` has invalid provenance statement_digest `{}`: {msg}",
                                path.display(),
                                shared_pkg.package_id,
                                shared_pkg.provenance.statement_digest
                            ),
                        )
                    },
                )?;
                if shared_pkg.provenance.statement_format.trim().is_empty() {
                    return Err(RuntimePackageLoaderError::new(
                        "R013",
                        format!(
                            "runtime lockfile `{}` shared std package `{}` has empty provenance statement_format",
                            path.display(),
                            shared_pkg.package_id
                        ),
                    ));
                }
                let mut previous_symbol: Option<&str> = None;
                let mut seen_symbols = HashSet::with_capacity(shared_pkg.symbols.len());
                for symbol in &shared_pkg.symbols {
                    if symbol.trim().is_empty() {
                        return Err(RuntimePackageLoaderError::new(
                            "R013",
                            format!(
                                "runtime lockfile `{}` shared std package `{}` has empty symbol entry",
                                path.display(),
                                shared_pkg.package_id
                            ),
                        ));
                    }
                    if let Some(prev) = previous_symbol {
                        if symbol.as_str() < prev {
                            return Err(RuntimePackageLoaderError::new(
                                "R013",
                                format!(
                                    "runtime lockfile `{}` shared std package `{}` symbols must be sorted",
                                    path.display(),
                                    shared_pkg.package_id
                                ),
                            ));
                        }
                    }
                    if !seen_symbols.insert(symbol.as_str()) {
                        return Err(RuntimePackageLoaderError::new(
                            "R013",
                            format!(
                                "runtime lockfile `{}` shared std package `{}` has duplicate symbol `{}`",
                                path.display(),
                                shared_pkg.package_id,
                                symbol
                            ),
                        ));
                    }
                    previous_symbol = Some(symbol.as_str());
                }
                let mut previous_dependency: Option<&str> = None;
                let mut seen_dependencies = HashSet::with_capacity(shared_pkg.dependencies.len());
                for dep in &shared_pkg.dependencies {
                    validate_runtime_package_id(dep.as_str()).map_err(|msg| {
                        RuntimePackageLoaderError::new(
                            "R013",
                            format!(
                                "runtime lockfile `{}` shared std package `{}` has invalid dependency id `{}`: {msg}",
                                path.display(),
                                shared_pkg.package_id,
                                dep
                            ),
                        )
                    })?;
                    if let Some(prev) = previous_dependency {
                        if dep.as_str() < prev {
                            return Err(RuntimePackageLoaderError::new(
                                "R013",
                                format!(
                                    "runtime lockfile `{}` shared std package `{}` dependencies must be sorted by exact id",
                                    path.display(),
                                    shared_pkg.package_id
                                ),
                            ));
                        }
                    }
                    if !seen_dependencies.insert(dep.as_str()) {
                        return Err(RuntimePackageLoaderError::new(
                            "R013",
                            format!(
                                "runtime lockfile `{}` shared std package `{}` has duplicate dependency id `{}`",
                                path.display(),
                                shared_pkg.package_id,
                                dep
                            ),
                        ));
                    }
                    previous_dependency = Some(dep.as_str());
                }

                let exact_id = format!("{}@{}", shared_pkg.package_id, shared_pkg.version);
                if let Some(prev) = previous_id.as_ref() {
                    if exact_id < *prev {
                        return Err(RuntimePackageLoaderError::new(
                            "R013",
                            format!(
                                "runtime lockfile `{}` shared std packages must be sorted by `package_id@version` (found `{}` before `{}`)",
                                path.display(),
                                prev,
                                exact_id
                            ),
                        ));
                    }
                }
                previous_id = Some(exact_id.clone());
                if digests_by_id.contains_key(exact_id.as_str()) {
                    return Err(RuntimePackageLoaderError::new(
                        "R013",
                        format!(
                            "runtime lockfile `{}` shared std package `{}` collides with normal package id space",
                            path.display(),
                            exact_id
                        ),
                    ));
                }
                let new_digest = shared_pkg.artifact.digest.clone();
                let new_artifact_path = shared_pkg.artifact.path.clone();
                if let Some(existing) = shared_std_by_id.insert(
                    exact_id.clone(),
                    RuntimeSharedStdLockEntry {
                        artifact_path: new_artifact_path,
                        digest: new_digest.clone(),
                        abi_major: shared_pkg.verified_std_abi.major,
                        abi_minor_min: shared_pkg.verified_std_abi.minor_min,
                        abi_minor_max: shared_pkg.verified_std_abi.minor_max,
                        dependencies: shared_pkg.dependencies,
                    },
                ) {
                    return Err(RuntimePackageLoaderError::new(
                        "R013",
                        format!(
                            "runtime lockfile `{}` has duplicate shared std package id `{}` (digests: `{}` vs `{}`)",
                            path.display(),
                            exact_id,
                            existing.digest,
                            new_digest
                        ),
                    ));
                }
            }

            Ok(RuntimeLockfileEvidence {
                resolver_version: Some(raw.resolver_version),
                digests_by_id,
                shared_std_by_id,
            })
        }
        other => Err(RuntimePackageLoaderError::new(
            "R013",
            format!(
                "runtime lockfile `{}` has unsupported schema_version {}; expected 0, 1, or 2",
                path.display(),
                other
            ),
        )),
    }
}

fn requirement_matches_version(requirement: &str, version: &str) -> bool {
    let req = requirement.trim();
    let (op, base) = if let Some(value) = req.strip_prefix('=') {
        ("=", value)
    } else if let Some(value) = req.strip_prefix('^') {
        ("^", value)
    } else if let Some(value) = req.strip_prefix('~') {
        ("~", value)
    } else {
        ("=", req)
    };

    let Some(base_triplet) = parse_semver_triplet(base) else {
        return false;
    };
    let Some(version_triplet) = parse_semver_triplet(version) else {
        return false;
    };

    match op {
        "=" => version_triplet == base_triplet,
        "^" => version_triplet.0 == base_triplet.0 && version_triplet >= base_triplet,
        "~" => {
            version_triplet.0 == base_triplet.0
                && version_triplet.1 == base_triplet.1
                && version_triplet >= base_triplet
        }
        _ => false,
    }
}

fn parse_semver_triplet(value: &str) -> Option<(u64, u64, u64)> {
    let mut parts = value.split('.');
    let major = parts.next()?.parse::<u64>().ok()?;
    let minor = parts.next()?.parse::<u64>().ok()?;
    let patch = parts.next()?.parse::<u64>().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some((major, minor, patch))
}

fn io_error_kind_label(err: &std::io::Error) -> &'static str {
    match err.kind() {
        std::io::ErrorKind::NotFound => "not_found",
        std::io::ErrorKind::PermissionDenied => "permission_denied",
        std::io::ErrorKind::AlreadyExists => "already_exists",
        std::io::ErrorKind::InvalidInput => "invalid_input",
        std::io::ErrorKind::InvalidData => "invalid_data",
        std::io::ErrorKind::TimedOut => "timed_out",
        std::io::ErrorKind::UnexpectedEof => "unexpected_eof",
        std::io::ErrorKind::WouldBlock => "would_block",
        std::io::ErrorKind::Interrupted => "interrupted",
        std::io::ErrorKind::OutOfMemory => "out_of_memory",
        _ => "other",
    }
}

fn json_error_class_label(err: &serde_json::Error) -> &'static str {
    match err.classify() {
        serde_json::error::Category::Io => "io",
        serde_json::error::Category::Syntax => "syntax",
        serde_json::error::Category::Data => "data",
        serde_json::error::Category::Eof => "eof",
    }
}

