use crate::commands::modules::verified_std_abi_supported_minor_range;

#[allow(clippy::too_many_arguments)]
fn resolve_runtime_link_packages(
    root: &Path,
    packages: &[RuntimeLinkPackageV0],
    lockfile: &RuntimeLockfileEvidence,
    signatures: &HashMap<String, RuntimePackageSignatureEntry>,
    trust_signers: &HashMap<&str, &crate::commands::build::strict_trust_policy::TrustedSignerV0>,
    revoked: &std::collections::HashSet<&str>,
    store_by_id_digest: &HashMap<(String, String), PackageStoreArtifactV0>,
    availability_policy: &RuntimeAvailabilityPolicy,
) -> Result<Vec<LoadedRuntimePackage>, RuntimePackageLoaderError> {
    let mut loaded_packages = Vec::with_capacity(packages.len());
    for pkg in packages {
        let locked_shared_std = lockfile.shared_std_by_id.get(pkg.id.as_str());
        validate_runtime_package_id(pkg.id.as_str()).map_err(|msg| {
            RuntimePackageLoaderError::new(
                "R012",
                format!("runtime link package id `{}` is invalid: {msg}", pkg.id),
            )
        })?;
        validate_sha256_digest(pkg.digest.as_str()).map_err(|msg| {
            RuntimePackageLoaderError::new(
                "R013",
                format!(
                    "runtime link package `{}` has invalid digest `{}`: {msg}",
                    pkg.id, pkg.digest
                ),
            )
        })?;
        validate_relative_artifact_path(pkg.artifact_path.as_str()).map_err(|msg| {
            RuntimePackageLoaderError::new(
                "R012",
                format!(
                    "runtime link package `{}` has invalid artifact_path `{}`: {msg}",
                    pkg.id, pkg.artifact_path
                ),
            )
        })?;
        if pkg.abi_id.trim().is_empty() {
            return Err(RuntimePackageLoaderError::new(
                "R015",
                format!("runtime link package `{}` has empty abi_id", pkg.id),
            ));
        }
        if let Some(shared_std) = locked_shared_std {
            if shared_std.digest != pkg.digest {
                return Err(RuntimePackageLoaderError::new(
                    "R013",
                    format!(
                        "runtime link shared std package `{}` digest `{}` does not match lockfile std digest `{}`",
                        pkg.id, pkg.digest, shared_std.digest
                    ),
                ));
            }
            if shared_std.artifact_path != pkg.artifact_path {
                return Err(RuntimePackageLoaderError::new(
                    "R017",
                    format!(
                        "runtime link shared std package `{}` artifact path `{}` does not match lockfile std artifact path `{}`",
                        pkg.id, pkg.artifact_path, shared_std.artifact_path
                    ),
                ));
            }
        } else {
            match lockfile.digests_by_id.get(pkg.id.as_str()) {
                Some(locked_digest) if locked_digest == &pkg.digest => {}
                Some(locked_digest) => {
                    return Err(RuntimePackageLoaderError::new(
                        "R013",
                        format!(
                            "runtime link package `{}` digest `{}` does not match lockfile digest `{}`",
                            pkg.id, pkg.digest, locked_digest
                        ),
                    ));
                }
                None => {
                    return Err(RuntimePackageLoaderError::new(
                        "R013",
                        format!(
                            "runtime link package `{}` is not pinned in `{}`",
                            pkg.id, STRICT_LOCKFILE_FILE
                        ),
                    ));
                }
            }
        }
        let selected_digest = locked_shared_std
            .map(|entry| entry.digest.as_str())
            .unwrap_or(pkg.digest.as_str());
        let selected_artifact_path = locked_shared_std
            .map(|entry| entry.artifact_path.as_str())
            .unwrap_or(pkg.artifact_path.as_str());
        if let Some(shared_std) = locked_shared_std {
            let (supported_major, supported_minor_min, supported_minor_max) =
                verified_std_abi_supported_minor_range();
            if shared_std.abi_major != supported_major {
                return Err(RuntimePackageLoaderError::new(
                    "R015",
                    format!(
                        "runtime shared std ABI mismatch for `{}`: package requires verified std ABI major `{}`, but compiler supports major `{}`",
                        pkg.id, shared_std.abi_major, supported_major
                    ),
                ));
            }
            if shared_std.abi_minor_min < supported_minor_min
                || shared_std.abi_minor_max > supported_minor_max
            {
                return Err(RuntimePackageLoaderError::new(
                    "R015",
                    format!(
                        "runtime shared std ABI mismatch for `{}`: package requires verified std ABI minor range `{}..={}`, but compiler supports `{}..={}`",
                        pkg.id,
                        shared_std.abi_minor_min,
                        shared_std.abi_minor_max,
                        supported_minor_min,
                        supported_minor_max
                    ),
                ));
            }
        }

        let signature = signatures.get(pkg.id.as_str()).ok_or_else(|| {
            RuntimePackageLoaderError::new(
                "R014",
                format!(
                    "runtime trust gate failed: package `{}` is missing signature entry in `{}`",
                    pkg.id, STRICT_PACKAGE_SIGNATURES_FILE
                ),
            )
        })?;
        if signature.digest != selected_digest {
            return Err(RuntimePackageLoaderError::new(
                "R014",
                format!(
                    "runtime trust gate failed for `{}`: signature digest `{}` does not match runtime-link digest `{}`",
                    pkg.id, signature.digest, selected_digest
                ),
            ));
        }
        let signer = trust_signers
            .get(signature.key_id.as_str())
            .copied()
            .ok_or_else(|| {
                RuntimePackageLoaderError::new(
                    "R014",
                    format!(
                        "runtime trust gate failed for `{}`: signer `{}` is not trusted",
                        pkg.id, signature.key_id
                    ),
                )
            })?;
        if revoked.contains(signature.key_id.as_str()) {
            return Err(RuntimePackageLoaderError::new(
                "R014",
                format!(
                    "runtime trust gate failed for `{}`: signer `{}` is revoked",
                    pkg.id, signature.key_id
                ),
            ));
        }
        let signed_at =
            parse_utc_timestamp_components(signature.signed_at.as_str()).map_err(|msg| {
                RuntimePackageLoaderError::new(
                    "R014",
                    format!(
                        "runtime trust gate failed for `{}`: signed_at `{}` is invalid: {msg}",
                        pkg.id, signature.signed_at
                    ),
                )
            })?;
        let signer_not_before = parse_utc_timestamp_components(signer.not_before.as_str())
            .map_err(|msg| {
                RuntimePackageLoaderError::new(
                    "R014",
                    format!(
                    "runtime trust gate failed for `{}`: signer `{}` not_before is invalid: {msg}",
                    pkg.id, signer.key_id
                ),
                )
            })?;
        let signer_not_after =
            parse_utc_timestamp_components(signer.not_after.as_str()).map_err(|msg| {
                RuntimePackageLoaderError::new(
                    "R014",
                    format!(
                    "runtime trust gate failed for `{}`: signer `{}` not_after is invalid: {msg}",
                    pkg.id, signer.key_id
                ),
                )
            })?;
        if signed_at < signer_not_before || signed_at >= signer_not_after {
            return Err(RuntimePackageLoaderError::new(
                "R014",
                format!(
                    "runtime trust gate failed for `{}`: signer `{}` is not valid at signed_at `{}`",
                    pkg.id, signer.key_id, signature.signed_at
                ),
            ));
        }
        let verifying =
            verifying_key_from_hex_public_key(signer.public_key.as_str()).map_err(|msg| {
                RuntimePackageLoaderError::new(
                    "R014",
                    format!(
                    "runtime trust gate failed for `{}`: signer `{}` public key is invalid: {msg}",
                    pkg.id, signer.key_id
                ),
                )
            })?;
        let signature_bytes = decode_signature(signature.signature.as_str()).map_err(|msg| {
            RuntimePackageLoaderError::new(
                "R014",
                format!(
                    "runtime trust gate failed for `{}`: signature is invalid: {msg}",
                    pkg.id
                ),
            )
        })?;
        let (name, version) = split_runtime_package_id(pkg.id.as_str()).map_err(|msg| {
            RuntimePackageLoaderError::new(
                "R014",
                format!("runtime trust gate failed for `{}`: {msg}", pkg.id),
            )
        })?;
        let payload =
            canonical_signature_payload_v0(name, version, selected_digest, signature.signed_at.as_str());
        verifying
            .verify_strict(payload.as_bytes(), &signature_bytes)
            .map_err(|_| {
                RuntimePackageLoaderError::new(
                    "R014",
                    format!(
                        "runtime trust gate failed for `{}`: signature verification failed for signer `{}`",
                        pkg.id, signer.key_id
                    ),
                )
            })?;

        let Some(store_artifact) =
            store_by_id_digest.get(&(pkg.id.clone(), selected_digest.to_string()))
        else {
            return Err(RuntimePackageLoaderError::new(
                "R012",
                format!(
                    "runtime package artifact `{}` with digest `{}` is missing from trusted local store/index",
                    pkg.id, selected_digest
                ),
            ));
        };
        if store_artifact.path != selected_artifact_path {
            return Err(RuntimePackageLoaderError::new(
                "R017",
                format!(
                    "runtime link package `{}` artifact path `{}` does not match store index path `{}`",
                    pkg.id, selected_artifact_path, store_artifact.path
                ),
            ));
        }
        let resolved_path = resolve_runtime_artifact_with_availability_policy(
            root,
            pkg.id.as_str(),
            selected_digest,
            selected_artifact_path,
            availability_policy,
        )?;
        loaded_packages.push(LoadedRuntimePackage {
            id: pkg.id.clone(),
            resolved_path,
        });
    }
    Ok(loaded_packages)
}

