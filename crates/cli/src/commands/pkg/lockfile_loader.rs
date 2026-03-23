#[cfg(test)]
fn load_lockfile_from_metadata(
    path: &Path,
    root_inputs: Option<Vec<StrictLockRootV1>>,
) -> Result<StrictLockfileV1, PkgLockError> {
    let policy = AdvisoryPolicy::standard();
    load_lockfile_from_metadata_with_policy(path, root_inputs, &policy)
}

fn load_lockfile_from_metadata_with_policy(
    path: &Path,
    root_inputs: Option<Vec<StrictLockRootV1>>,
    advisory_policy: &AdvisoryPolicy,
) -> Result<StrictLockfileV1, PkgLockError> {
    if !path.exists() {
        return Err(PkgLockError::new(
            "C027",
            format!("canonical package metadata `{}` is missing", path.display()),
        ));
    }
    if !path.is_file() {
        return Err(PkgLockError::new(
            "C027",
            format!(
                "canonical package metadata path `{}` exists but is not a file",
                path.display()
            ),
        ));
    }
    let advisory_root = path.parent().unwrap_or(Path::new("."));
    let advisories = load_advisories(advisory_root, advisory_policy)?;

    let content = fs::read_to_string(path)
        .map_err(|err| PkgLockError::new("C027", format!("reading {}: {err}", path.display())))?;
    let raw: PackageMetadataRoot = serde_json::from_str(content.as_str())
        .map_err(|err| PkgLockError::new("C027", format!("parsing {}: {err}", path.display())))?;
    if !matches!(raw.schema_version, 0 | 1) {
        return Err(PkgLockError::new(
            "C027",
            format!(
                "unsupported package metadata schema_version {} in `{}`; expected 0 or 1",
                raw.schema_version,
                path.display()
            ),
        ));
    }

    let mut seen_package_ids = HashSet::with_capacity(raw.packages.len());
    let mut duplicate_package_ids = BTreeSet::new();
    let mut packages = Vec::with_capacity(raw.packages.len());
    for pkg in raw.packages {
        let PackageMetadataEntry {
            name,
            version,
            digest,
            artifact,
            abi_id,
            dependencies: raw_dependencies,
            signature,
            trust,
        } = pkg;
        validate_package_id(name.as_str()).map_err(|msg| {
            PkgLockError::new("C027", format!("invalid package name `{}`: {msg}", name))
        })?;
        validate_exact_semver(version.as_str()).map_err(|msg| {
            PkgLockError::new(
                "C027",
                format!(
                    "invalid package version `{}` for `{}`: {msg}",
                    version, name
                ),
            )
        })?;
        let semver = SemVer::parse(version.as_str())?;
        validate_sha256_digest(digest.as_str()).map_err(|msg| {
            PkgLockError::new(
                "C027",
                format!("invalid digest `{}` for `{}`: {msg}", digest, name),
            )
        })?;
        if artifact.format != "wasm" {
            return Err(PkgLockError::new(
                "C027",
                format!(
                    "invalid artifact format `{}` for `{}`; expected `wasm`",
                    artifact.format, name
                ),
            ));
        }
        if artifact.path.trim().is_empty() {
            return Err(PkgLockError::new(
                "C027",
                format!("invalid artifact path for `{}`: path is empty", name),
            ));
        }
        let artifact_path_ref = Path::new(artifact.path.as_str());
        if artifact_path_ref.is_absolute() {
            return Err(PkgLockError::new(
                "C027",
                format!(
                    "invalid artifact path `{}` for `{}`: path must be relative",
                    artifact.path, name
                ),
            ));
        }
        for component in artifact_path_ref.components() {
            match component {
                Component::ParentDir => {
                    return Err(PkgLockError::new(
                        "C027",
                        format!(
                            "invalid artifact path `{}` for `{}`: path must not contain parent-directory traversal (`..`)",
                            artifact.path, name
                        ),
                    ));
                }
                Component::Prefix(_) | Component::RootDir => {
                    return Err(PkgLockError::new(
                        "C027",
                        format!(
                            "invalid artifact path `{}` for `{}`: path must be relative",
                            artifact.path, name
                        ),
                    ));
                }
                Component::CurDir | Component::Normal(_) => {}
            }
        }
        let artifact_path = artifact.path;
        if abi_id.trim().is_empty() {
            return Err(PkgLockError::new(
                "C027",
                format!("invalid abi_id for `{}`: abi_id is empty", name),
            ));
        }
        if let Some(signature) = signature.as_ref() {
            let _ = (
                &signature.format,
                &signature.key_id,
                &signature.signed_at,
                &signature.signature,
            );
        }
        if let Some(trust) = trust.as_ref() {
            let _ = &trust.trusted_anchor_ids;
        }
        let package_id = format!("{}@{}", name, version);
        if !seen_package_ids.insert(package_id.clone()) {
            duplicate_package_ids.insert(package_id);
            continue;
        }

        let mut dependency_seen = HashSet::with_capacity(raw_dependencies.len());
        let mut dependency_dupes = BTreeSet::new();
        for dep in &raw_dependencies {
            validate_package_id(dep.name.as_str()).map_err(|msg| {
                PkgLockError::new(
                    "C027",
                    format!(
                        "invalid dependency name `{}` for package `{}`: {msg}",
                        dep.name, name
                    ),
                )
            })?;
            validate_semver_requirement(dep.requirement.as_str()).map_err(|msg| {
                PkgLockError::new(
                    "C027",
                    format!(
                        "invalid dependency requirement `{}` for package `{}` dependency `{}`: {msg}",
                        dep.requirement, name, dep.name
                    ),
                )
            })?;
            if !dependency_seen.insert(dep.name.clone()) {
                dependency_dupes.insert(dep.name.clone());
            }
        }
        if let Some(first) = dependency_dupes.iter().next() {
            return Err(PkgLockError::new(
                "C027",
                format!(
                    "duplicate dependency name `{}` in package `{}` in canonical package metadata",
                    first, name
                ),
            ));
        }

        let mut dependencies = raw_dependencies;
        dependencies.sort_by(|a, b| a.name.cmp(&b.name));
        packages.push(ValidatedPackage {
            name,
            version,
            semver,
            digest,
            artifact_path,
            abi_id,
            dependencies,
        });
    }
    if let Some(first) = duplicate_package_ids.iter().next() {
        return Err(PkgLockError::new(
            "C027",
            format!(
                "duplicate package id `{}` in canonical package metadata",
                first
            ),
        ));
    }

    packages.sort_by(|a, b| {
        a.name
            .cmp(&b.name)
            .then_with(|| a.version.cmp(&b.version))
            .then_with(|| a.digest.cmp(&b.digest))
    });
    let mut catalog: HashMap<String, Vec<ValidatedPackage>> = HashMap::new();
    for pkg in packages {
        catalog.entry(pkg.name.clone()).or_default().push(pkg);
    }
    for candidates in catalog.values_mut() {
        candidates.sort_by(|a, b| {
            b.semver
                .cmp(&a.semver)
                .then_with(|| a.digest.cmp(&b.digest))
                .then_with(|| a.artifact_path.cmp(&b.artifact_path))
        });
    }

    for candidates in catalog.values() {
        for pkg in candidates {
            for dep in &pkg.dependencies {
                if !catalog.contains_key(dep.name.as_str()) {
                    return Err(PkgLockError::new(
                        "C027",
                        format!(
                            "package `{}` references dependency `{}` which is not present in canonical package metadata",
                            pkg.name, dep.name
                        ),
                    ));
                }
            }
        }
    }

    let roots = root_inputs.unwrap_or_else(|| derive_root_inputs_for_generate(&catalog));
    let selected = solve_deterministic_versions(
        &catalog,
        roots.as_slice(),
        advisories.as_slice(),
        advisory_policy.advisory_as_of,
    )?;
    if let Some(cycle) = detect_resolved_cycle(&selected) {
        return Err(PkgLockError::new(
            "C112",
            format!("deterministic transitive dependency cycle detected: {cycle}"),
        ));
    }

    let mut selected_names: Vec<&str> = selected.keys().map(|name| name.as_str()).collect();
    selected_names.sort();
    let mut locked_packages = Vec::with_capacity(selected_names.len());
    for name in selected_names {
        let pkg = selected
            .get(name)
            .expect("selected package name should map to a package");
        let mut dependency_ids = Vec::with_capacity(pkg.dependencies.len());
        for dep in &pkg.dependencies {
            let dep_pkg = selected.get(dep.name.as_str()).ok_or_else(|| {
                PkgLockError::new(
                    "C113",
                    format!(
                        "deterministic semver solver found no satisfiable version set for `{}`",
                        dep.name
                    ),
                )
            })?;
            dependency_ids.push(format!("{}@{}", dep_pkg.name, dep_pkg.version));
        }
        dependency_ids.sort();
        locked_packages.push(StrictLockedPackageV1 {
            id: format!("{}@{}", pkg.name, pkg.version),
            name: pkg.name.clone(),
            version: pkg.version.clone(),
            digest: pkg.digest.clone(),
            abi_id: pkg.abi_id.clone(),
            dependencies: dependency_ids,
        });
    }
    locked_packages.sort_by(|a, b| a.id.cmp(&b.id));

    Ok(StrictLockfileV1 {
        schema_version: 1,
        resolver_version: 1,
        roots,
        packages: locked_packages,
    })
}

fn derive_root_inputs_for_generate(
    catalog: &HashMap<String, Vec<ValidatedPackage>>,
) -> Vec<StrictLockRootV1> {
    let mut referenced = HashSet::new();
    for candidates in catalog.values() {
        for pkg in candidates {
            for dep in &pkg.dependencies {
                referenced.insert(dep.name.clone());
            }
        }
    }
    let mut root_names: Vec<String> = catalog
        .keys()
        .filter(|name| !referenced.contains(*name))
        .cloned()
        .collect();
    if root_names.is_empty() {
        root_names = catalog.keys().cloned().collect();
    }
    root_names.sort();

    let mut dependencies = Vec::with_capacity(root_names.len());
    for root_name in root_names {
        let selected = catalog
            .get(root_name.as_str())
            .and_then(|candidates| candidates.first())
            .expect("root package name should have a candidate");
        dependencies.push(StrictLockRootDependencyV1 {
            name: selected.name.clone(),
            requirement: format!("={}", selected.version),
        });
    }
    vec![StrictLockRootV1 {
        name: "app".to_string(),
        dependencies,
    }]
}

