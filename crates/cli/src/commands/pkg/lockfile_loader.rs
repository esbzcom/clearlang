#[cfg(test)]
fn load_lockfile_from_metadata(
    path: &Path,
    root_inputs: Option<Vec<StrictLockRootV1>>,
    project_manifest: Option<&ProjectManifestV1>,
) -> Result<StrictLockfile, PkgLockError> {
    let policy = AdvisoryPolicy::standard();
    load_lockfile_from_metadata_with_policy(path, root_inputs, project_manifest, &policy)
}

fn load_lockfile_from_metadata_with_policy(
    path: &Path,
    root_inputs: Option<Vec<StrictLockRootV1>>,
    project_manifest: Option<&ProjectManifestV1>,
    advisory_policy: &AdvisoryPolicy,
) -> Result<StrictLockfile, PkgLockError> {
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
            verified_std_abi,
            provenance,
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
            signature,
            verified_std_abi,
            provenance,
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

    let locked_packages = locked_packages_from_selection(&selected)?;
    let Some(manifest) = project_manifest else {
        return Ok(StrictLockfile::V1(StrictLockfileV1 {
            schema_version: 1,
            resolver_version: 1,
            roots,
            packages: locked_packages,
        }));
    };
    let Some(std) = manifest.std.as_ref() else {
        return Ok(StrictLockfile::V1(StrictLockfileV1 {
            schema_version: 1,
            resolver_version: 1,
            roots,
            packages: locked_packages,
        }));
    };

    let std_section = build_locked_std_section(LockedStdSectionInputs {
        metadata_path: path,
        metadata_root: advisory_root,
        catalog: &catalog,
        roots: roots.as_slice(),
        locked_packages: locked_packages.as_slice(),
        selected_packages: &selected,
        std,
        advisories: advisories.as_slice(),
        advisory_policy,
    })?;
    Ok(StrictLockfile::V2(StrictLockfileV2 {
        schema_version: 2,
        resolver_version: 1,
        roots,
        packages: locked_packages,
        std: std_section,
    }))
}

fn locked_packages_from_selection(
    selected: &HashMap<String, ValidatedPackage>,
) -> Result<Vec<StrictLockedPackageV1>, PkgLockError> {
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
    Ok(locked_packages)
}

struct LockedStdSectionInputs<'a> {
    metadata_path: &'a Path,
    metadata_root: &'a Path,
    catalog: &'a HashMap<String, Vec<ValidatedPackage>>,
    roots: &'a [StrictLockRootV1],
    locked_packages: &'a [StrictLockedPackageV1],
    selected_packages: &'a HashMap<String, ValidatedPackage>,
    std: &'a ProjectStdConfigV2,
    advisories: &'a [AdvisoryEntry],
    advisory_policy: &'a AdvisoryPolicy,
}

fn build_locked_std_section(
    inputs: LockedStdSectionInputs<'_>,
) -> Result<StrictStdSectionV2, PkgLockError> {
    let LockedStdSectionInputs {
        metadata_path,
        metadata_root,
        catalog,
        roots,
        locked_packages,
        selected_packages,
        std,
        advisories,
        advisory_policy,
    } = inputs;
    if std.delivery == "embedded" {
        return Ok(StrictStdSectionV2 {
            delivery: "embedded".to_string(),
            packages: Vec::new(),
        });
    }

    let shared_std_roots = vec![StrictLockRootV1 {
        name: "shared-std".to_string(),
        dependencies: std
            .packages
            .iter()
            .map(|package| StrictLockRootDependencyV1 {
                name: package.package_id.clone(),
                requirement: package.version_requirement.clone(),
            })
            .collect(),
    }];
    let shared_selection = solve_deterministic_versions(
        catalog,
        shared_std_roots.as_slice(),
        advisories,
        advisory_policy.advisory_as_of,
    )?;
    if let Some(cycle) = detect_resolved_cycle(&shared_selection) {
        return Err(PkgLockError::new(
            "C112",
            format!("deterministic shared std dependency cycle detected: {cycle}"),
        ));
    }

    for package in &std.packages {
        if selected_packages.contains_key(package.package_id.as_str()) {
            return Err(PkgLockError::new(
                "C109",
                format!(
                    "shared std package `{}` collides with normal locked package space; keep shared std out of `dependencies[]`/`packages[]`",
                    package.package_id
                ),
            ));
        }
    }

    let mut shared_locked = Vec::with_capacity(std.packages.len());
    for package in &std.packages {
        if !is_bundled_std_package_id(package.package_id.as_str()) {
            return Err(PkgLockError::new(
                "C111",
                format!(
                    "shared std package `{}` is not a supported externalizable std package id",
                    package.package_id
                ),
            ));
        }
        let selected = shared_selection
            .get(package.package_id.as_str())
            .ok_or_else(|| {
                PkgLockError::new(
                    "C113",
                    format!(
                        "deterministic semver solver found no satisfiable shared std version set for `{}`",
                        package.package_id
                    ),
                )
            })?;
        ensure_locked_shared_std_abi(metadata_path, selected, &package.verified_std_abi)?;
        let signature = selected.signature.as_ref().ok_or_else(|| {
            PkgLockError::new(
                "C111",
                format!(
                    "shared std package `{}` is missing canonical signature metadata",
                    package.package_id
                ),
            )
        })?;
        if signature.format != "ed25519"
            || signature.key_id.trim().is_empty()
            || signature.signed_at.trim().is_empty()
            || signature.signature.trim().is_empty()
        {
            return Err(PkgLockError::new(
                "C111",
                format!(
                    "shared std package `{}` has incomplete canonical signature metadata",
                    package.package_id
                ),
            ));
        }
        let provenance = selected.provenance.as_ref().ok_or_else(|| {
            PkgLockError::new(
                "C111",
                format!(
                    "shared std package `{}` is missing canonical provenance metadata",
                    package.package_id
                ),
            )
        })?;
        validate_sha256_digest(provenance.statement_digest.as_str()).map_err(|msg| {
            PkgLockError::new(
                "C111",
                format!(
                    "shared std package `{}` has invalid provenance statement_digest `{}`: {msg}",
                    package.package_id, provenance.statement_digest
                ),
            )
        })?;
        if provenance.statement_format.trim().is_empty() {
            return Err(PkgLockError::new(
                "C111",
                format!(
                    "shared std package `{}` has empty provenance statement_format",
                    package.package_id
                ),
            ));
        }
        let symbols = bundled_std_package_symbols(package.package_id.as_str())
            .ok_or_else(|| {
                PkgLockError::new(
                    "C111",
                    format!(
                        "shared std package `{}` is missing bundled symbol metadata",
                        package.package_id
                    ),
                )
            })?
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        let artifact_path = metadata_root.join(selected.artifact_path.as_str());
        let artifact_size = fs::metadata(&artifact_path)
            .map_err(|err| {
                PkgLockError::new(
                    "C111",
                    format!(
                        "reading shared std artifact `{}` for `{}`: {err}",
                        artifact_path.display(),
                        package.package_id
                    ),
                )
            })?
            .len();
        if artifact_size == 0 {
            return Err(PkgLockError::new(
                "C111",
                format!(
                    "shared std artifact `{}` for `{}` has zero size",
                    artifact_path.display(),
                    package.package_id
                ),
            ));
        }
        let mut dependency_ids = Vec::new();
        for dependency in &selected.dependencies {
            let dep_pkg = shared_selection.get(dependency.name.as_str()).ok_or_else(|| {
                PkgLockError::new(
                    "C111",
                    format!(
                        "shared std package `{}` depends on `{}` outside the locked shared std package set",
                        package.package_id,
                        dependency.name
                    ),
                )
            })?;
            dependency_ids.push(format!("{}@{}", dep_pkg.name, dep_pkg.version));
        }
        shared_locked.push(StrictLockedSharedStdPackageV2 {
            package_id: selected.name.clone(),
            version: selected.version.clone(),
            verified_std_abi: crate::commands::shared_std_lock::SharedStdAbiClaim {
                major: selected
                    .verified_std_abi
                    .as_ref()
                    .expect("validated shared std abi")
                    .major,
                minor_min: selected
                    .verified_std_abi
                    .as_ref()
                    .expect("validated shared std abi")
                    .minor_min,
                minor_max: selected
                    .verified_std_abi
                    .as_ref()
                    .expect("validated shared std abi")
                    .minor_max,
            },
            artifact: crate::commands::shared_std_lock::ValidatedSharedStdArtifactV2 {
                format: "wasm".to_string(),
                path: selected.artifact_path.clone(),
                digest: selected.digest.clone(),
                size_bytes: artifact_size,
            },
            signature: crate::commands::shared_std_lock::ValidatedSharedStdSignatureV2 {
                key_id: signature.key_id.clone(),
                algorithm: signature.format.clone(),
                signed_at: signature.signed_at.clone(),
                signature: signature.signature.clone(),
            },
            provenance: crate::commands::shared_std_lock::ValidatedSharedStdProvenanceV2 {
                statement_digest: provenance.statement_digest.clone(),
                statement_format: provenance.statement_format.clone(),
            },
            symbols,
            dependencies: dependency_ids,
        });
    }

    crate::commands::shared_std_lock::normalize_and_validate_shared_std_lock_section_for_lockfile(
        roots,
        locked_packages,
        StrictStdSectionV2 {
            delivery: "shared".to_string(),
            packages: shared_locked,
        },
        metadata_path,
        "pkg lock shared std section",
    )
    .map_err(|message| PkgLockError::new("C111", message))
}

fn ensure_locked_shared_std_abi(
    metadata_path: &Path,
    package: &ValidatedPackage,
    requirement: &ProjectStdAbiRequirementV2,
) -> Result<(), PkgLockError> {
    let abi = package.verified_std_abi.as_ref().ok_or_else(|| {
        PkgLockError::new(
            "C111",
            format!(
                "shared std package `{}` in `{}` is missing canonical verified_std_abi metadata",
                package.name,
                metadata_path.display()
            ),
        )
    })?;
    if abi.minor_max < abi.minor_min {
        return Err(PkgLockError::new(
            "C111",
            format!(
                "shared std package `{}` has invalid canonical verified_std_abi range {}.{}..{}",
                package.name, abi.major, abi.minor_min, abi.minor_max
            ),
        ));
    }
    if abi.major != requirement.major
        || abi.minor_min < requirement.minor_min
        || abi.minor_max > requirement.minor_max
    {
        return Err(PkgLockError::new(
            "C111",
            format!(
                "shared std package `{}` locked ABI {}.{}..{} does not satisfy manifest requirement {}.{}..{}",
                package.name,
                abi.major,
                abi.minor_min,
                abi.minor_max,
                requirement.major,
                requirement.minor_min,
                requirement.minor_max
            ),
        ));
    }
    Ok(())
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

