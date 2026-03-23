pub(super) fn load_runtime_packages_from_local_store_if_present(
    root: &Path,
    config: RuntimeLoaderConfig,
) -> Result<Option<LoadedRuntimePackageSet>, RuntimePackageLoaderError> {
    if config.allow_remote_fetch {
        return Err(RuntimePackageLoaderError::new(
            "R012",
            "runtime loader remote fetch is disabled by default in phase 23.0.1; provide trusted local store/index inputs only",
        ));
    }

    let production_profile = load_runtime_production_profile_if_present(root)?;
    let runtime_link_path = root.join(RUNTIME_LINK_FILE);
    if !runtime_link_path.exists() {
        if let Some(profile) = production_profile.as_ref() {
            return Err(RuntimePackageLoaderError::new(
                "R012",
                format!(
                    "runtime loader fail-closed: production host profile `{profile}` requires `{}` to enable mandatory runtime trust checks",
                    RUNTIME_LINK_FILE,
                    profile = profile.profile
                ),
            ));
        }
        return Ok(None);
    }
    if !runtime_link_path.is_file() {
        return Err(RuntimePackageLoaderError::new(
            "R012",
            format!(
                "runtime link artifact path `{}` exists but is not a file",
                runtime_link_path.display()
            ),
        ));
    }
    let runtime_link_hash_path = root.join(RUNTIME_LINK_HASH_FILE);
    if !runtime_link_hash_path.exists() {
        return Err(RuntimePackageLoaderError::new(
            "R017",
            format!(
                "runtime link hash companion `{}` is missing",
                runtime_link_hash_path.display()
            ),
        ));
    }
    if !runtime_link_hash_path.is_file() {
        return Err(RuntimePackageLoaderError::new(
            "R017",
            format!(
                "runtime link hash path `{}` exists but is not a file",
                runtime_link_hash_path.display()
            ),
        ));
    }

    let runtime_link_text = fs::read_to_string(&runtime_link_path).map_err(|err| {
        RuntimePackageLoaderError::new(
            "R012",
            format!(
                "reading {} failed (io_kind={})",
                runtime_link_path.display(),
                io_error_kind_label(&err)
            ),
        )
    })?;
    let runtime_link_value: serde_json::Value = serde_json::from_str(runtime_link_text.as_str())
        .map_err(|err| {
            RuntimePackageLoaderError::new(
                "R012",
                format!(
                    "parsing {} failed (json_class={})",
                    runtime_link_path.display(),
                    json_error_class_label(&err)
                ),
            )
        })?;
    let runtime_link: RuntimeLinkRootV0 = serde_json::from_value(runtime_link_value.clone())
        .map_err(|err| {
            let _ = err;
            RuntimePackageLoaderError::new(
                "R012",
                format!(
                    "runtime link artifact `{}` does not match schema v0",
                    runtime_link_path.display()
                ),
            )
        })?;
    if runtime_link.schema_version != 0 {
        return Err(RuntimePackageLoaderError::new(
            "R012",
            format!(
                "runtime link artifact `{}` has unsupported schema_version {}; expected 0",
                runtime_link_path.display(),
                runtime_link.schema_version
            ),
        ));
    }

    enforce_runtime_link_deterministic_shape(&runtime_link).map_err(|msg| {
        RuntimePackageLoaderError::new(
            "R017",
            format!(
                "runtime link artifact `{}` violates deterministic ordering: {msg}",
                runtime_link_path.display()
            ),
        )
    })?;

    let canonical_link_hash = sha256_hex(canonical_json_bytes(&runtime_link_value).as_slice());
    let expected_hash = fs::read_to_string(&runtime_link_hash_path).map_err(|err| {
        RuntimePackageLoaderError::new(
            "R017",
            format!(
                "reading {} failed (io_kind={})",
                runtime_link_hash_path.display(),
                io_error_kind_label(&err)
            ),
        )
    })?;
    let expected_hash = expected_hash.trim();
    validate_runtime_link_hash(expected_hash).map_err(|msg| {
        RuntimePackageLoaderError::new(
            "R017",
            format!(
                "runtime link hash companion `{}` is invalid: {msg}",
                runtime_link_hash_path.display()
            ),
        )
    })?;
    if expected_hash != canonical_link_hash {
        return Err(RuntimePackageLoaderError::new(
            "R017",
            format!(
                "runtime link hash mismatch for `{}`: expected `{}`, got `{}`",
                runtime_link_path.display(),
                expected_hash,
                canonical_link_hash
            ),
        ));
    }

    validate_required_runtime_host_capabilities(
        production_profile.as_ref(),
        config.required_host_capabilities.as_slice(),
    )?;

    let lockfile = load_runtime_lockfile_evidence(root)?;
    if let Some(expected_resolver_version) = lockfile.resolver_version {
        if expected_resolver_version != runtime_link.resolver_version {
            return Err(RuntimePackageLoaderError::new(
                "R013",
                format!(
                    "runtime-link resolver_version `{}` does not match lockfile resolver_version `{}`",
                    runtime_link.resolver_version, expected_resolver_version
                ),
            ));
        }
    }
    let signatures = load_runtime_package_signatures(root)?;
    let trust_policy = load_required_trust_policy_v0(root).map_err(|err| {
        RuntimePackageLoaderError::new(
            "R014",
            format!(
                "loading trust policy for runtime package verification failed (strict code {})",
                err.code()
            ),
        )
    })?;
    let availability_policy = load_runtime_availability_policy(root)?;
    let trust_signers: HashMap<
        &str,
        &crate::commands::build::strict_trust_policy::TrustedSignerV0,
    > = trust_policy
        .trusted_signers
        .iter()
        .map(|signer| (signer.key_id.as_str(), signer))
        .collect();
    let revoked: std::collections::HashSet<&str> = trust_policy
        .revoked_key_ids
        .iter()
        .map(|key_id| key_id.as_str())
        .collect();

    let store_index = load_package_store_index(root)?;
    let store_by_id_digest = build_store_artifact_lookup(store_index.artifacts.as_slice())?;

    validate_runtime_link_bindings_reference_known_providers(&runtime_link)?;
    let loaded_packages = resolve_runtime_link_packages(
        root,
        runtime_link.packages.as_slice(),
        &lockfile,
        &signatures,
        &trust_signers,
        &revoked,
        &store_by_id_digest,
        &availability_policy,
    )?;

    Ok(Some(LoadedRuntimePackageSet {
        packages: loaded_packages,
        bindings: runtime_link
            .bindings
            .iter()
            .map(|binding| LoadedRuntimeBinding {
                import_module: binding.import_module.clone(),
                import_name: binding.import_name.clone(),
                provider_package_id: binding.provider_package_id.clone(),
                provider_symbol: binding.provider_symbol.clone(),
            })
            .collect(),
        active_profile_capabilities: production_profile.map(|profile| profile.capabilities),
    }))
}

fn build_store_artifact_lookup(
    artifacts: &[PackageStoreArtifactV0],
) -> Result<HashMap<(String, String), PackageStoreArtifactV0>, RuntimePackageLoaderError> {
    let mut store_by_id_digest: HashMap<(String, String), PackageStoreArtifactV0> =
        HashMap::with_capacity(artifacts.len());
    for artifact in artifacts {
        validate_runtime_package_id(artifact.id.as_str()).map_err(|msg| {
            RuntimePackageLoaderError::new(
                "R012",
                format!("store artifact id `{}` is invalid: {msg}", artifact.id),
            )
        })?;
        validate_sha256_digest(artifact.digest.as_str()).map_err(|msg| {
            RuntimePackageLoaderError::new(
                "R013",
                format!(
                    "store artifact `{}` has invalid digest `{}`: {msg}",
                    artifact.id, artifact.digest
                ),
            )
        })?;
        validate_relative_artifact_path(artifact.path.as_str()).map_err(|msg| {
            RuntimePackageLoaderError::new(
                "R012",
                format!(
                    "store artifact `{}` has invalid path `{}`: {msg}",
                    artifact.id, artifact.path
                ),
            )
        })?;
        let key = (artifact.id.clone(), artifact.digest.clone());
        if store_by_id_digest.contains_key(&key) {
            return Err(RuntimePackageLoaderError::new(
                "R017",
                format!(
                    "store index `{}` contains duplicate artifact `(id={}, digest={})`",
                    PACKAGE_STORE_INDEX_FILE, artifact.id, artifact.digest
                ),
            ));
        }
        store_by_id_digest.insert(key, artifact.clone());
    }
    Ok(store_by_id_digest)
}

fn validate_runtime_link_bindings_reference_known_providers(
    runtime_link: &RuntimeLinkRootV0,
) -> Result<(), RuntimePackageLoaderError> {
    let runtime_link_package_ids: std::collections::HashSet<&str> = runtime_link
        .packages
        .iter()
        .map(|pkg| pkg.id.as_str())
        .collect();
    for binding in &runtime_link.bindings {
        if !runtime_link_package_ids.contains(binding.provider_package_id.as_str()) {
            return Err(RuntimePackageLoaderError::new(
                "R015",
                format!(
                    "runtime link binding `{}`::`{}` references unknown provider_package_id `{}`",
                    binding.import_module, binding.import_name, binding.provider_package_id
                ),
            ));
        }
    }
    Ok(())
}

