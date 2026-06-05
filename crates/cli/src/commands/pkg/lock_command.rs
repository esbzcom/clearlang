pub struct RunLockArgs {
    pub generate: bool,
    pub update: bool,
    pub compiler_mode: CompilerMode,
    pub advisory_as_of: Option<String>,
    pub root: PathBuf,
    pub json_errors: bool,
    pub emit_stdout_summary: bool,
}

pub fn run_lock(args: RunLockArgs, logger: Logger) -> Result<()> {
    let RunLockArgs {
        generate,
        update,
        compiler_mode,
        advisory_as_of,
        root,
        json_errors,
        emit_stdout_summary,
    } = args;
    let fail_pkg = |code: &'static str, message: String| -> Result<()> {
        if json_errors {
            let json = make_single_json_error(code, "build", message, &root, 0, 0, None);
            Err(CommandError::json(json).into())
        } else {
            Err(anyhow!(message))
        }
    };

    if generate == update {
        return fail_pkg(
            "C027",
            "pkg lock requires exactly one of `--generate` or `--update`".to_string(),
        );
    }

    let advisory_policy = match (compiler_mode, advisory_as_of.as_deref()) {
        (CompilerMode::Strict, Some(raw)) => {
            let parsed = parse_utc_timestamp_components(raw).map_err(|msg| {
                PkgLockError::new(
                    "C117",
                    format!(
                        "`--advisory-as-of` must be valid UTC RFC3339 (`YYYY-MM-DDTHH:MM:SSZ`): {}",
                        msg
                    ),
                )
            });
            match parsed {
                Ok(ts) => AdvisoryPolicy::strict(ts),
                Err(err) => return fail_pkg(err.code(), err.to_string()),
            }
        }
        (CompilerMode::Strict, None) => {
            return fail_pkg(
                "C117",
                "`--compiler-mode strict` requires `--advisory-as-of <RFC3339_UTC>` for deterministic advisory evaluation".to_string(),
            )
        }
        (CompilerMode::Standard, Some(raw)) => {
            let parsed = parse_utc_timestamp_components(raw).map_err(|msg| {
                PkgLockError::new(
                    "C117",
                    format!(
                        "`--advisory-as-of` must be valid UTC RFC3339 (`YYYY-MM-DDTHH:MM:SSZ`): {}",
                        msg
                    ),
                )
            });
            match parsed {
                Ok(ts) => AdvisoryPolicy {
                    compiler_mode,
                    advisory_as_of: Some(ts),
                },
                Err(err) => return fail_pkg(err.code(), err.to_string()),
            }
        }
        (CompilerMode::Permissive, Some(raw)) => {
            let parsed = parse_utc_timestamp_components(raw).map_err(|msg| {
                PkgLockError::new(
                    "C117",
                    format!(
                        "`--advisory-as-of` must be valid UTC RFC3339 (`YYYY-MM-DDTHH:MM:SSZ`): {}",
                        msg
                    ),
                )
            });
            match parsed {
                Ok(ts) => AdvisoryPolicy {
                    compiler_mode,
                    advisory_as_of: Some(ts),
                },
                Err(err) => return fail_pkg(err.code(), err.to_string()),
            }
        }
        (CompilerMode::Standard, None) => AdvisoryPolicy::standard(),
        (CompilerMode::Permissive, None) => AdvisoryPolicy::permissive(),
    };

    let mut timings = StageTimings::new();
    let metadata_path = root.join(CANONICAL_PACKAGE_METADATA_FILE);
    let lockfile_path = root.join(STRICT_LOCKFILE_FILE);
    let legacy_metadata_path = root.join(LEGACY_PACKAGE_METADATA_FILE);
    let project_manifest = match load_project_manifest_v1(root.as_path()) {
        Ok(value) => value,
        Err(err) => return fail_pkg("C027", err.message().to_string()),
    };
    if let Some(manifest) = project_manifest.as_ref() {
        if let Err(err) = enforce_project_clg_version_compatibility(manifest) {
            return fail_pkg("C027", err.message().to_string());
        }
    }
    if legacy_metadata_path.exists() {
        return fail_pkg(
            "C109",
            format!(
                "package metadata model coexistence conflict: legacy metadata `{}` is not supported; migrate to canonical `{}` + `clg.project.json` + `clg.lock.json`",
                legacy_metadata_path.display(),
                CANONICAL_PACKAGE_METADATA_FILE
            ),
        );
    }

    if generate && lockfile_path.exists() {
        return fail_pkg(
            "C027",
            format!(
                "lockfile `{}` already exists; use `clg pkg lock --update`",
                lockfile_path.display()
            ),
        );
    }
    if update && !lockfile_path.exists() {
        return fail_pkg(
            "C027",
            format!(
                "lockfile `{}` does not exist; use `clg pkg lock --generate`",
                lockfile_path.display()
            ),
        );
    }

    let lockfile = {
        let _stage = timings.start(logger, "pkg_load_metadata");
        let root_inputs = if let Some(manifest) = project_manifest.as_ref() {
            let manifest_roots = vec![StrictLockRootV1 {
                name: manifest.project.name.clone(),
                dependencies: manifest
                    .dependencies
                    .iter()
                    .map(|dependency| StrictLockRootDependencyV1 {
                        name: dependency.name.clone(),
                        requirement: dependency.requirement.clone(),
                    })
                    .collect(),
            }];
            if update {
                if let Err(err) = ensure_manifest_roots_match_existing_lock(
                    manifest_roots.as_slice(),
                    lockfile_path.as_path(),
                ) {
                    return fail_pkg(err.code(), err.to_string());
                }
            }
            Some(manifest_roots)
        } else if update {
            match load_root_inputs_for_update(lockfile_path.as_path()) {
                Ok(v) => Some(v),
                Err(err) => return fail_pkg(err.code(), err.to_string()),
            }
        } else {
            None
        };
        match load_lockfile_from_metadata_with_policy(
            metadata_path.as_path(),
            root_inputs,
            project_manifest.as_ref(),
            &advisory_policy,
        ) {
            Ok(value) => value,
            Err(err) => return fail_pkg(err.code(), err.to_string()),
        }
    };

    let canonical_hash = {
        let _stage = timings.start(logger, "pkg_write_lockfile");
        write_lockfile(lockfile_path.as_path(), &lockfile)?
    };
    let resolved_graph_hash = {
        let _stage = timings.start(logger, "pkg_write_resolved_graph");
        write_resolved_graph_artifact(root.as_path(), &lockfile)?
    };

    if emit_stdout_summary {
        println!(
            "wrote {} with {} pinned package(s) [sha256:{}]",
            lockfile_path.display(),
            lockfile.packages().len(),
            canonical_hash
        );
        println!(
            "wrote {} [sha256:{}]",
            root.join(RESOLVED_GRAPH_FILE).display(),
            resolved_graph_hash
        );
    }
    logger.summary(&timings);
    Ok(())
}

fn load_root_inputs_for_update(path: &Path) -> Result<Vec<StrictLockRootV1>, PkgLockError> {
    let content = fs::read_to_string(path)
        .map_err(|err| PkgLockError::new("C111", format!("reading {}: {err}", path.display())))?;
    let raw: ExistingStrictLockfileV1 = serde_json::from_str(content.as_str()).map_err(|err| {
        PkgLockError::new(
            "C111",
            format!("parsing existing lockfile {}: {err}", path.display()),
        )
    })?;
    if raw.schema_version != 1 && raw.schema_version != 2 {
        return Err(PkgLockError::new(
            "C111",
            format!(
                "existing lockfile `{}` must use schema_version 1 or 2",
                path.display()
            ),
        ));
    }
    if raw.resolver_version != 1 {
        return Err(PkgLockError::new(
            "C111",
            format!(
                "existing lockfile `{}` must use resolver_version 1",
                path.display()
            ),
        ));
    }

    let mut roots = Vec::with_capacity(raw.roots.len());
    let mut root_names = HashSet::new();
    let mut root_name_dupes = BTreeSet::new();
    for root in raw.roots {
        if root.name.trim().is_empty() {
            return Err(PkgLockError::new(
                "C111",
                format!(
                    "existing lockfile `{}` has root with empty name",
                    path.display()
                ),
            ));
        }
        if !root_names.insert(root.name.clone()) {
            root_name_dupes.insert(root.name.clone());
        }
        let mut dependencies = Vec::with_capacity(root.dependencies.len());
        let mut dep_names = HashSet::new();
        let mut dep_dupes = BTreeSet::new();
        for dep in root.dependencies {
            validate_package_id(dep.name.as_str()).map_err(|msg| {
                PkgLockError::new(
                    "C111",
                    format!(
                        "existing lockfile `{}` root `{}` has invalid dependency name `{}`: {msg}",
                        path.display(),
                        root.name,
                        dep.name
                    ),
                )
            })?;
            validate_semver_requirement(dep.requirement.as_str()).map_err(|msg| {
                PkgLockError::new(
                    "C111",
                    format!(
                        "existing lockfile `{}` root `{}` dependency `{}` has invalid requirement `{}`: {msg}",
                        path.display(),
                        root.name,
                        dep.name,
                        dep.requirement
                    ),
                )
            })?;
            if !dep_names.insert(dep.name.clone()) {
                dep_dupes.insert(dep.name.clone());
            }
            dependencies.push(StrictLockRootDependencyV1 {
                name: dep.name,
                requirement: dep.requirement,
            });
        }
        if let Some(dupe) = dep_dupes.iter().next() {
            return Err(PkgLockError::new(
                "C111",
                format!(
                    "existing lockfile `{}` root `{}` has duplicate dependency `{}`",
                    path.display(),
                    root.name,
                    dupe
                ),
            ));
        }
        dependencies.sort_by(|a, b| a.name.cmp(&b.name));
        roots.push(StrictLockRootV1 {
            name: root.name,
            dependencies,
        });
    }
    if let Some(dupe) = root_name_dupes.iter().next() {
        return Err(PkgLockError::new(
            "C111",
            format!(
                "existing lockfile `{}` has duplicate root name `{}`",
                path.display(),
                dupe
            ),
        ));
    }
    roots.sort_by(|a, b| a.name.cmp(&b.name));
    if roots.is_empty() {
        return Err(PkgLockError::new(
            "C111",
            format!("existing lockfile `{}` has empty `roots`", path.display()),
        ));
    }
    Ok(roots)
}

fn ensure_manifest_roots_match_existing_lock(
    manifest_roots: &[StrictLockRootV1],
    lockfile_path: &Path,
) -> Result<(), PkgLockError> {
    let existing_roots = load_root_inputs_for_update(lockfile_path)?;
    if root_signature(manifest_roots) == root_signature(existing_roots.as_slice()) {
        return Ok(());
    }
    Err(PkgLockError::new(
        "C109",
        format!(
            "package metadata model compatibility failure: manifest roots do not match existing lockfile roots in `{}`; expected `{}` from `clg.project.json`, found `{}` in `clg.lock.json`; regenerate lockfile from manifest",
            lockfile_path.display(),
            describe_roots(manifest_roots),
            describe_roots(existing_roots.as_slice())
        ),
    ))
}

fn root_signature(roots: &[StrictLockRootV1]) -> Vec<(String, Vec<(String, String)>)> {
    let mut out = Vec::with_capacity(roots.len());
    for root in roots {
        let mut deps = root
            .dependencies
            .iter()
            .map(|dep| (dep.name.clone(), dep.requirement.clone()))
            .collect::<Vec<_>>();
        deps.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
        out.push((root.name.clone(), deps));
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

fn describe_roots(roots: &[StrictLockRootV1]) -> String {
    let signature = root_signature(roots);
    let mut parts = Vec::with_capacity(signature.len());
    for (name, deps) in signature {
        let deps_text = deps
            .iter()
            .map(|(dep_name, requirement)| format!("{dep_name}:{requirement}"))
            .collect::<Vec<_>>()
            .join(",");
        parts.push(format!("{name}[{deps_text}]"));
    }
    parts.join(";")
}
