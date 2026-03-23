fn load_runtime_availability_policy(
    root: &Path,
) -> Result<RuntimeAvailabilityPolicy, RuntimePackageLoaderError> {
    let path = root.join(RUNTIME_LOADER_POLICY_FILE);
    if !path.exists() {
        return Ok(RuntimeAvailabilityPolicy::default());
    }
    if !path.is_file() {
        return Err(RuntimePackageLoaderError::new(
            "R012",
            format!(
                "runtime loader policy path `{}` exists but is not a file",
                path.display()
            ),
        ));
    }
    let content = fs::read_to_string(&path).map_err(|err| {
        let _ = err;
        RuntimePackageLoaderError::new(
            "R012",
            format!("reading runtime loader policy `{}` failed", path.display()),
        )
    })?;
    let raw: RawRuntimeLoaderPolicyV0 = serde_json::from_str(content.as_str()).map_err(|err| {
        let _ = err;
        RuntimePackageLoaderError::new(
            "R012",
            format!(
                "runtime loader policy `{}` does not match schema v0 (`schema_version`, `offline_mode`, `mirror_paths`, `artifact_read_retries`)",
                path.display()
            ),
        )
    })?;
    if raw.schema_version != 0 {
        return Err(RuntimePackageLoaderError::new(
            "R012",
            format!(
                "runtime loader policy `{}` has unsupported schema_version {}; expected 0",
                path.display(),
                raw.schema_version
            ),
        ));
    }
    if !raw.offline_mode {
        return Err(RuntimePackageLoaderError::new(
            "R012",
            format!(
                "runtime loader policy `{}` sets `offline_mode=false`, but remote fetching is not supported in phase 23.0.4",
                path.display()
            ),
        ));
    }
    if raw.artifact_read_retries == 0 {
        return Err(RuntimePackageLoaderError::new(
            "R012",
            format!(
                "runtime loader policy `{}` must set `artifact_read_retries` to >= 1",
                path.display()
            ),
        ));
    }
    if raw.artifact_read_retries > 8 {
        return Err(RuntimePackageLoaderError::new(
            "R012",
            format!(
                "runtime loader policy `{}` must keep `artifact_read_retries` <= 8 for deterministic runtime bounds",
                path.display()
            ),
        ));
    }

    let mut seen_paths = std::collections::HashSet::with_capacity(raw.mirror_paths.len());
    let mut mirror_roots = Vec::with_capacity(raw.mirror_paths.len());
    for value in raw.mirror_paths {
        validate_relative_artifact_path(value.as_str()).map_err(|msg| {
            RuntimePackageLoaderError::new(
                "R012",
                format!(
                    "runtime loader policy `{}` has invalid mirror path `{}`: {msg}",
                    path.display(),
                    value
                ),
            )
        })?;
        if !seen_paths.insert(value.clone()) {
            return Err(RuntimePackageLoaderError::new(
                "R012",
                format!(
                    "runtime loader policy `{}` has duplicate mirror path `{}`",
                    path.display(),
                    value
                ),
            ));
        }
        mirror_roots.push(PathBuf::from(value));
    }

    Ok(RuntimeAvailabilityPolicy {
        mirror_roots,
        artifact_read_retries: raw.artifact_read_retries,
    })
}

fn load_runtime_production_profile_if_present(
    root: &Path,
) -> Result<Option<RuntimeProductionHostProfile>, RuntimePackageLoaderError> {
    let path = root.join(HOST_PROFILE_FILE);
    if !path.exists() {
        return Ok(None);
    }
    if !path.is_file() {
        return Err(RuntimePackageLoaderError::new(
            "R016",
            format!(
                "runtime host profile path `{}` exists but is not a file",
                path.display()
            ),
        ));
    }
    let content = fs::read_to_string(&path).map_err(|err| {
        let _ = err;
        RuntimePackageLoaderError::new(
            "R016",
            format!("reading runtime host profile `{}` failed", path.display()),
        )
    })?;
    let raw: RawHostProfileV0 = serde_json::from_str(content.as_str()).map_err(|err| {
        let _ = err;
        RuntimePackageLoaderError::new(
            "R016",
            format!(
                "runtime host profile `{}` does not match schema v0 (`schema_version`, `profile`, `capabilities`)",
                path.display()
            ),
        )
    })?;
    if raw.schema_version != 0 {
        return Err(RuntimePackageLoaderError::new(
            "R016",
            format!(
                "runtime host profile `{}` has unsupported schema_version {}; expected 0",
                path.display(),
                raw.schema_version
            ),
        ));
    }
    match raw.profile.as_str() {
        "contract_static" | "shared_app" => {
            let capabilities =
                validate_runtime_host_profile_capabilities(path.as_path(), raw.capabilities.as_slice())?;
            Ok(Some(RuntimeProductionHostProfile {
                profile: raw.profile,
                capabilities,
            }))
        }
        _ => Err(RuntimePackageLoaderError::new(
            "R016",
            format!(
                "runtime host profile `{}` has unsupported profile `{}`; expected `contract_static` or `shared_app`",
                path.display(),
                raw.profile
            ),
        )),
    }
}

fn validate_runtime_host_profile_capabilities(
    path: &Path,
    capabilities: &[String],
) -> Result<Vec<String>, RuntimePackageLoaderError> {
    let mut sorted = capabilities.to_vec();
    sorted.sort();

    let mut seen = HashSet::with_capacity(sorted.len());
    let mut duplicates = BTreeSet::new();
    for capability in &sorted {
        if !seen.insert(capability.clone()) {
            duplicates.insert(capability.clone());
        }
    }
    if let Some(duplicate) = duplicates.iter().next() {
        return Err(RuntimePackageLoaderError::new(
            "R016",
            format!(
                "runtime host profile `{}` has duplicate capability `{}`",
                path.display(),
                duplicate
            ),
        ));
    }

    for capability in &sorted {
        if capability.trim().is_empty() {
            return Err(RuntimePackageLoaderError::new(
                "R016",
                format!(
                    "runtime host profile `{}` contains an empty capability id",
                    path.display()
                ),
            ));
        }
        if !is_known_host_capability(capability.as_str()) {
            return Err(RuntimePackageLoaderError::new(
                "R016",
                format!(
                    "runtime host profile `{}` has unsupported capability `{}`",
                    path.display(),
                    capability
                ),
            ));
        }
    }
    Ok(sorted)
}

fn validate_required_runtime_host_capabilities(
    profile: Option<&RuntimeProductionHostProfile>,
    required_capabilities: &[String],
) -> Result<(), RuntimePackageLoaderError> {
    let Some(profile) = profile else {
        return Ok(());
    };

    let configured: HashSet<&str> = profile.capabilities.iter().map(String::as_str).collect();
    let mut missing = BTreeSet::new();
    for required in required_capabilities {
        if !configured.contains(required.as_str()) {
            missing.insert(required.clone());
        }
    }
    if missing.is_empty() {
        return Ok(());
    }
    let missing_list = missing.iter().cloned().collect::<Vec<_>>().join(", ");
    Err(RuntimePackageLoaderError::new(
        "R016",
        format!(
            "runtime host profile `{}` is missing required capabilities for active module imports: {}",
            profile.profile, missing_list
        ),
    ))
}

