fn parse_package_metadata_abi_v0(
    metadata_content: &str,
    abi_content: &str,
    metadata_path: &Path,
    abi_path: &Path,
) -> Result<StrictPackageContractV0, StrictPackageContractError> {
    let (metadata_value, schema_version) = parse_metadata_json(metadata_content, metadata_path)?;
    let raw_abi = parse_abi_json(abi_content, abi_path)?;

    let mut packages: Vec<StrictPackageMetadataEntry> = match schema_version {
        0 => {
            let raw_metadata: RawPackageMetadataRootV0 = serde_json::from_value(metadata_value)
                .map_err(|_| {
                    StrictPackageContractError::new(
                        "C104",
                        format!(
                            "strict package metadata `{}` does not match schema v0 (`schema_version`, `packages[]`)",
                            metadata_path.display()
                        ),
                    )
                })?;
            debug_assert_eq!(raw_metadata.schema_version, 0);
            raw_metadata
                .packages
                .into_iter()
                .map(|pkg| StrictPackageMetadataEntry {
                    name: pkg.name,
                    version: pkg.version,
                    digest: pkg.digest,
                    artifact_format: pkg.artifact.format,
                    artifact_path: pkg.artifact.path,
                    abi_id: pkg.abi_id,
                    dependencies: Vec::new(),
                    signature: None,
                    trusted_anchor_ids: Vec::new(),
                })
                .collect()
        }
        1 => {
            let raw_metadata: RawPackageMetadataRootV1 = serde_json::from_value(metadata_value)
                .map_err(|_| {
                    StrictPackageContractError::new(
                        "C104",
                        format!(
                            "strict package metadata `{}` does not match schema v1 (`schema_version`, `packages[]` with `signature`, `trust`, optional `dependencies`)",
                            metadata_path.display()
                        ),
                    )
                })?;
            debug_assert_eq!(raw_metadata.schema_version, 1);
            raw_metadata
                .packages
                .into_iter()
                .map(|pkg| StrictPackageMetadataEntry {
                    dependencies: pkg
                        .dependencies
                        .into_iter()
                        .map(|dep| StrictPackageDependencyRequirement {
                            name: dep.name,
                            requirement: dep.requirement,
                        })
                        .collect(),
                    name: pkg.name,
                    version: pkg.version,
                    digest: pkg.digest,
                    artifact_format: pkg.artifact.format,
                    artifact_path: pkg.artifact.path,
                    abi_id: pkg.abi_id,
                    signature: Some(StrictPackageMetadataSignature {
                        format: pkg.signature.format,
                        key_id: pkg.signature.key_id,
                        signed_at: pkg.signature.signed_at,
                        signature: pkg.signature.signature,
                    }),
                    trusted_anchor_ids: pkg.trust.trusted_anchor_ids,
                })
                .collect()
        }
        other => {
            return Err(StrictPackageContractError::new(
                "C104",
                format!(
                    "strict package metadata `{}` has unsupported schema_version {}; expected 0 or 1",
                    metadata_path.display(),
                    other
                ),
            ));
        }
    };
    packages.sort_by(|a, b| a.name.cmp(&b.name));

    let mut package_names = HashSet::with_capacity(packages.len());
    let mut package_dupes = BTreeSet::new();
    for pkg in &mut packages {
        if !package_names.insert(pkg.name.clone()) {
            package_dupes.insert(pkg.name.clone());
        }
    }
    if let Some(dupe) = package_dupes.iter().next() {
        return Err(StrictPackageContractError::new(
            "C104",
            format!(
                "strict package metadata `{}` has duplicate package name `{}`",
                metadata_path.display(),
                dupe
            ),
        ));
    }

    for pkg in &mut packages {
        validate_package_id(&pkg.name).map_err(|msg| {
            StrictPackageContractError::new(
                "C104",
                format!(
                    "strict package metadata `{}` package `{}` is invalid: {msg}",
                    metadata_path.display(),
                    pkg.name
                ),
            )
        })?;
        validate_exact_semver(&pkg.version).map_err(|msg| {
            StrictPackageContractError::new(
                "C104",
                format!(
                    "strict package metadata `{}` package `{}` has invalid version `{}`: {msg}",
                    metadata_path.display(),
                    pkg.name,
                    pkg.version
                ),
            )
        })?;
        validate_sha256_digest(&pkg.digest).map_err(|msg| {
            StrictPackageContractError::new(
                "C104",
                format!(
                    "strict package metadata `{}` package `{}` has invalid digest `{}`: {msg}",
                    metadata_path.display(),
                    pkg.name,
                    pkg.digest
                ),
            )
        })?;
        if pkg.artifact_path.trim().is_empty() {
            return Err(StrictPackageContractError::new(
                "C104",
                format!(
                    "strict package metadata `{}` package `{}` has empty artifact path",
                    metadata_path.display(),
                    pkg.name
                ),
            ));
        }
        if pkg.artifact_format != "wasm" {
            return Err(StrictPackageContractError::new(
                "C104",
                format!(
                    "strict package metadata `{}` package `{}` has unsupported artifact format `{}`; expected `wasm`",
                    metadata_path.display(),
                    pkg.name,
                    pkg.artifact_format
                ),
            ));
        }
        validate_relative_artifact_path(pkg.artifact_path.as_str()).map_err(|msg| {
            StrictPackageContractError::new(
                "C104",
                format!(
                    "strict package metadata `{}` package `{}` has invalid artifact path `{}`: {msg}",
                    metadata_path.display(),
                    pkg.name,
                    pkg.artifact_path
                ),
            )
        })?;
        if pkg.abi_id.trim().is_empty() {
            return Err(StrictPackageContractError::new(
                "C104",
                format!(
                    "strict package metadata `{}` package `{}` has empty abi_id",
                    metadata_path.display(),
                    pkg.name
                ),
            ));
        }
        if let Some(signature) = pkg.signature.as_ref() {
            if pkg.trusted_anchor_ids.is_empty() {
                return Err(StrictPackageContractError::new(
                    "C104",
                    format!(
                        "strict package metadata `{}` package `{}` must declare at least one trusted anchor id when signature metadata is present",
                        metadata_path.display(),
                        pkg.name
                    ),
                ));
            }
            if signature.format != "ed25519" {
                return Err(StrictPackageContractError::new(
                    "C104",
                    format!(
                        "strict package metadata `{}` package `{}` has unsupported signature format `{}`; expected `ed25519`",
                        metadata_path.display(),
                        pkg.name,
                        signature.format
                    ),
                ));
            }
            if signature.key_id.trim().is_empty() {
                return Err(StrictPackageContractError::new(
                    "C104",
                    format!(
                        "strict package metadata `{}` package `{}` has empty signature key_id",
                        metadata_path.display(),
                        pkg.name
                    ),
                ));
            }
            validate_utc_rfc3339("signed_at", &signature.signed_at).map_err(|msg| {
                StrictPackageContractError::new(
                    "C104",
                    format!(
                        "strict package metadata `{}` package `{}` has invalid signature signed_at `{}`: {msg}",
                        metadata_path.display(),
                        pkg.name,
                        signature.signed_at
                    ),
                )
            })?;
            validate_signature_hex(&signature.signature).map_err(|msg| {
                StrictPackageContractError::new(
                    "C104",
                    format!(
                        "strict package metadata `{}` package `{}` has invalid signature `{}`: {msg}",
                        metadata_path.display(),
                        pkg.name,
                        signature.signature
                    ),
                )
            })?;
        }
        if !pkg.trusted_anchor_ids.is_empty() {
            let mut seen_anchors = HashSet::new();
            let mut anchor_dupes = BTreeSet::new();
            for anchor_id in &pkg.trusted_anchor_ids {
                if anchor_id.trim().is_empty() {
                    return Err(StrictPackageContractError::new(
                        "C104",
                        format!(
                            "strict package metadata `{}` package `{}` has empty trusted anchor id",
                            metadata_path.display(),
                            pkg.name
                        ),
                    ));
                }
                if !seen_anchors.insert(anchor_id.clone()) {
                    anchor_dupes.insert(anchor_id.clone());
                }
            }
            if let Some(dupe) = anchor_dupes.iter().next() {
                return Err(StrictPackageContractError::new(
                    "C104",
                    format!(
                        "strict package metadata `{}` package `{}` has duplicate trusted anchor id `{}`",
                        metadata_path.display(),
                        pkg.name,
                        dupe
                    ),
                ));
            }
        }

        pkg.dependencies.sort_by(|a, b| a.name.cmp(&b.name));
        let mut seen_deps = HashSet::with_capacity(pkg.dependencies.len());
        let mut dep_dupes = BTreeSet::new();
        for dep in &pkg.dependencies {
            validate_package_id(dep.name.as_str()).map_err(|msg| {
                StrictPackageContractError::new(
                    "C104",
                    format!(
                        "strict package metadata `{}` package `{}` has invalid dependency name `{}`: {msg}",
                        metadata_path.display(),
                        pkg.name,
                        dep.name
                    ),
                )
            })?;
            validate_semver_requirement(dep.requirement.as_str()).map_err(|msg| {
                StrictPackageContractError::new(
                    "C104",
                    format!(
                        "strict package metadata `{}` package `{}` has invalid dependency requirement `{}` for `{}`: {msg}",
                        metadata_path.display(),
                        pkg.name,
                        dep.requirement,
                        dep.name
                    ),
                )
            })?;
            if !seen_deps.insert(dep.name.clone()) {
                dep_dupes.insert(dep.name.clone());
            }
        }
        if let Some(dupe) = dep_dupes.iter().next() {
            return Err(StrictPackageContractError::new(
                "C104",
                format!(
                    "strict package metadata `{}` package `{}` has duplicate dependency name `{}`",
                    metadata_path.display(),
                    pkg.name,
                    dupe
                ),
            ));
        }
    }
    let mut package_abi_ids = HashSet::with_capacity(packages.len());
    let mut duplicate_package_abi_ids = BTreeSet::new();
    for pkg in &packages {
        if !package_abi_ids.insert(pkg.abi_id.clone()) {
            duplicate_package_abi_ids.insert(pkg.abi_id.clone());
        }
    }
    if let Some(dupe) = duplicate_package_abi_ids.iter().next() {
        return Err(StrictPackageContractError::new(
            "C104",
            format!(
                "strict package metadata `{}` has duplicate abi_id `{}` across packages",
                metadata_path.display(),
                dupe
            ),
        ));
    }

    let mut contracts: Vec<StrictAbiContractEntry> = raw_abi
        .contracts
        .into_iter()
        .map(|contract| StrictAbiContractEntry {
            abi_id: contract.abi_id,
            package: contract.package,
            version: contract.version,
            imports: contract
                .imports
                .into_iter()
                .map(|entry| StrictAbiImportEntry {
                    symbol: entry.symbol,
                    effect: entry.effect,
                    params: entry.params,
                    ret: entry.ret,
                    capability: entry.capability,
                })
                .collect(),
        })
        .collect();
    contracts.sort_by(|a, b| a.abi_id.cmp(&b.abi_id));

    let mut contract_ids = HashSet::with_capacity(contracts.len());
    let mut contract_dupes = BTreeSet::new();
    for contract in &contracts {
        if !contract_ids.insert(contract.abi_id.clone()) {
            contract_dupes.insert(contract.abi_id.clone());
        }
    }
    if let Some(dupe) = contract_dupes.iter().next() {
        return Err(StrictPackageContractError::new(
            "C104",
            format!(
                "strict package ABI `{}` has duplicate abi_id `{}`",
                abi_path.display(),
                dupe
            ),
        ));
    }

    for contract in &mut contracts {
        if contract.abi_id.trim().is_empty() {
            return Err(StrictPackageContractError::new(
                "C104",
                format!(
                    "strict package ABI `{}` contains contract with empty abi_id",
                    abi_path.display()
                ),
            ));
        }
        validate_package_id(&contract.package).map_err(|msg| {
            StrictPackageContractError::new(
                "C104",
                format!(
                    "strict package ABI `{}` contract `{}` has invalid package `{}`: {msg}",
                    abi_path.display(),
                    contract.abi_id,
                    contract.package
                ),
            )
        })?;
        validate_exact_semver(&contract.version).map_err(|msg| {
            StrictPackageContractError::new(
                "C104",
                format!(
                    "strict package ABI `{}` contract `{}` has invalid version `{}`: {msg}",
                    abi_path.display(),
                    contract.abi_id,
                    contract.version
                ),
            )
        })?;

        contract.imports.sort_by(|a, b| a.symbol.cmp(&b.symbol));
        let mut seen_symbols = HashSet::with_capacity(contract.imports.len());
        let mut symbol_dupes = BTreeSet::new();
        for import in &contract.imports {
            if !seen_symbols.insert(import.symbol.clone()) {
                symbol_dupes.insert(import.symbol.clone());
            }
        }
        if let Some(dupe) = symbol_dupes.iter().next() {
            return Err(StrictPackageContractError::new(
                "C104",
                format!(
                    "strict package ABI `{}` contract `{}` has duplicate symbol `{}`",
                    abi_path.display(),
                    contract.abi_id,
                    dupe
                ),
            ));
        }

        for import in &contract.imports {
            if import.symbol.trim().is_empty() {
                return Err(StrictPackageContractError::new(
                    "C104",
                    format!(
                        "strict package ABI `{}` contract `{}` contains empty symbol name",
                        abi_path.display(),
                        contract.abi_id
                    ),
                ));
            }
            match import.effect.as_str() {
                "pure" | "mut" | "io" => {}
                _ => {
                    return Err(StrictPackageContractError::new(
                        "C104",
                        format!(
                            "strict package ABI `{}` contract `{}` symbol `{}` has unsupported effect `{}`",
                            abi_path.display(),
                            contract.abi_id,
                            import.symbol,
                            import.effect
                        ),
                    ));
                }
            }
            if import.ret.trim().is_empty() {
                return Err(StrictPackageContractError::new(
                    "C104",
                    format!(
                        "strict package ABI `{}` contract `{}` symbol `{}` has empty return type",
                        abi_path.display(),
                        contract.abi_id,
                        import.symbol
                    ),
                ));
            }
            for ty in &import.params {
                if ty.trim().is_empty() {
                    return Err(StrictPackageContractError::new(
                        "C104",
                        format!(
                            "strict package ABI `{}` contract `{}` symbol `{}` has empty parameter type",
                            abi_path.display(),
                            contract.abi_id,
                            import.symbol
                        ),
                    ));
                }
            }
            if let Some(capability) = import.capability.as_ref() {
                if capability.trim().is_empty() {
                    return Err(StrictPackageContractError::new(
                        "C104",
                        format!(
                            "strict package ABI `{}` contract `{}` symbol `{}` has empty capability",
                            abi_path.display(),
                            contract.abi_id,
                            import.symbol
                        ),
                    ));
                }
            }
        }
    }

    let metadata_by_abi: HashMap<&str, (&str, &str)> = packages
        .iter()
        .map(|pkg| {
            (
                pkg.abi_id.as_str(),
                (pkg.name.as_str(), pkg.version.as_str()),
            )
        })
        .collect();
    let abi_by_id: HashMap<&str, (&str, &str)> = contracts
        .iter()
        .map(|abi| {
            (
                abi.abi_id.as_str(),
                (abi.package.as_str(), abi.version.as_str()),
            )
        })
        .collect();

    let mut missing_contract_ids = BTreeSet::new();
    for pkg in &packages {
        if !abi_by_id.contains_key(pkg.abi_id.as_str()) {
            missing_contract_ids.insert(pkg.abi_id.clone());
        }
    }
    if let Some(missing) = missing_contract_ids.iter().next() {
        return Err(StrictPackageContractError::new(
            "C105",
            format!(
                "strict package metadata/ABI mismatch: abi_id `{}` is referenced in metadata but missing from ABI contracts",
                missing
            ),
        ));
    }

    let mut orphan_contract_ids = BTreeSet::new();
    for contract in &contracts {
        if !metadata_by_abi.contains_key(contract.abi_id.as_str()) {
            orphan_contract_ids.insert(contract.abi_id.clone());
        }
    }
    if let Some(orphan) = orphan_contract_ids.iter().next() {
        return Err(StrictPackageContractError::new(
            "C105",
            format!(
                "strict package metadata/ABI mismatch: abi_id `{}` exists in ABI contracts but is not referenced by metadata",
                orphan
            ),
        ));
    }

    let mut mismatched = BTreeSet::new();
    for (abi_id, (meta_pkg, meta_ver)) in &metadata_by_abi {
        let (abi_pkg, abi_ver) = abi_by_id.get(abi_id).expect("checked presence above");
        if meta_pkg != abi_pkg || meta_ver != abi_ver {
            mismatched.insert((*abi_id).to_string());
        }
    }
    if let Some(abi_id) = mismatched.iter().next() {
        return Err(StrictPackageContractError::new(
            "C105",
            format!(
                "strict package metadata/ABI mismatch: abi_id `{}` has inconsistent package/version between metadata and ABI contract",
                abi_id
            ),
        ));
    }

    Ok(StrictPackageContractV0 {
        packages,
        contracts,
    })
}

