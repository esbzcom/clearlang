fn strict_import_map_artifact_path(out: &Path) -> PathBuf {
    let file_name = out
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("out.wasm");
    let artifact_name = if let Some(stem) = file_name.strip_suffix(".wasm") {
        format!("{stem}.strict-import-map.json")
    } else {
        format!("{file_name}.strict-import-map.json")
    };
    match out.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent.join(artifact_name),
        _ => PathBuf::from(artifact_name),
    }
}

fn write_strict_import_map_artifact(
    out: &Path,
    artifact: &StrictImportMapArtifact,
) -> Result<PathBuf> {
    let path = strict_import_map_artifact_path(out);
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
        }
    }
    fs::write(&path, artifact.canonical_bytes.as_slice())
        .with_context(|| format!("writing {}", path.display()))?;
    Ok(path)
}

fn strict_import_map_artifact_with_determinism_check(
    expected_profiles: &std::collections::BTreeMap<String, AbiLinkProfile>,
    host_profile: &StrictHostProfileV0,
    source_files: &[String],
    baseline_outcome: &StrictGateOutcome,
    replay_outcome: &StrictGateOutcome,
) -> std::result::Result<StrictImportMapArtifact, String> {
    let baseline_json = strict_import_map_artifact_json(
        expected_profiles,
        host_profile,
        source_files,
        baseline_outcome.linked_imports.as_slice(),
        baseline_outcome.violations.as_slice(),
    );
    let baseline_bytes = canonical_json_bytes(&baseline_json);
    let baseline_hash = sha256_hex(baseline_bytes.as_slice());

    let replay_json = strict_import_map_artifact_json(
        expected_profiles,
        host_profile,
        source_files,
        replay_outcome.linked_imports.as_slice(),
        replay_outcome.violations.as_slice(),
    );
    let replay_bytes = canonical_json_bytes(&replay_json);
    let replay_hash = sha256_hex(replay_bytes.as_slice());

    if baseline_outcome.violations.as_slice() != replay_outcome.violations.as_slice()
        || baseline_bytes != replay_bytes
        || baseline_hash != replay_hash
    {
        return Err(
            "strict determinism replay failed: identical inputs produced different canonical direct-dependency import map or diagnostics ordering"
                .to_string(),
        );
    }

    Ok(StrictImportMapArtifact {
        canonical_bytes: baseline_bytes,
        canonical_hash: baseline_hash,
    })
}

fn strict_import_map_artifact_json(
    expected_profiles: &std::collections::BTreeMap<String, AbiLinkProfile>,
    host_profile: &StrictHostProfileV0,
    source_files: &[String],
    linked_imports: &[(String, AbiLinkProfile)],
    diagnostics: &[StrictGateViolation],
) -> serde_json::Value {
    use serde_json::json;

    let mut imports = linked_imports
        .iter()
        .map(|(symbol, linked_profile)| {
            let required_capability = expected_profiles
                .get(symbol)
                .and_then(|profile| profile.capability.as_deref());
            json!({
                "symbol": symbol,
                "package": package_id_from_symbol(symbol),
                "effect": linked_profile.effect,
                "params": linked_profile.params,
                "ret": linked_profile.ret,
                "required_capability": required_capability,
            })
        })
        .collect::<Vec<_>>();
    imports.sort_by(|lhs, rhs| {
        let lhs_symbol = lhs
            .get("symbol")
            .and_then(|value| value.as_str())
            .unwrap_or_default();
        let rhs_symbol = rhs
            .get("symbol")
            .and_then(|value| value.as_str())
            .unwrap_or_default();
        lhs_symbol.cmp(rhs_symbol)
    });

    let diagnostics_json = diagnostics
        .iter()
        .map(|diagnostic| {
            json!({
                "code": diagnostic.code,
                "package": diagnostic.package,
                "symbol": diagnostic.symbol,
                "message": diagnostic.message,
            })
        })
        .collect::<Vec<_>>();

    json!({
        "schema_version": 0,
        "kind": "clg.strict_direct_dependency_import_map.v0",
        "host_profile": host_profile.profile,
        "source_files": source_files,
        "imports": imports,
        "diagnostics": diagnostics_json,
    })
}

#[cfg(test)]
fn strict_determinism_violation(
    expected_profiles: &std::collections::BTreeMap<String, AbiLinkProfile>,
    host_profile: &StrictHostProfileV0,
    source_files: &[String],
    linked_imports: &[(String, AbiLinkProfile)],
    baseline_diagnostics: &[StrictGateViolation],
) -> Option<String> {
    let baseline_outcome = StrictGateOutcome {
        linked_imports: linked_imports.to_vec(),
        violations: baseline_diagnostics.to_vec(),
    };
    let mut replay_inputs = linked_imports.to_vec();
    replay_inputs.reverse();
    let replay_outcome = StrictGateOutcome {
        linked_imports: replay_inputs.clone(),
        violations: evaluate_strict_gates(
            expected_profiles,
            replay_inputs.as_slice(),
            host_profile,
        ),
    };
    strict_import_map_artifact_with_determinism_check(
        expected_profiles,
        host_profile,
        source_files,
        &baseline_outcome,
        &replay_outcome,
    )
    .err()
}

fn strict_artifact_identity_violation(
    lockfile: &StrictLockfileV0,
    package_contract: &StrictPackageContractV0,
) -> Option<String> {
    let mut lock_idx = 0usize;
    let mut package_idx = 0usize;
    while lock_idx < lockfile.dependencies.len() && package_idx < package_contract.packages.len() {
        let lock = &lockfile.dependencies[lock_idx];
        let package = &package_contract.packages[package_idx];
        match lock.name.cmp(&package.name) {
            std::cmp::Ordering::Less => {
                return Some(format!(
                    "strict artifact identity mismatch: lockfile dependency `{}` is missing from package metadata",
                    lock.name
                ));
            }
            std::cmp::Ordering::Greater => {
                return Some(format!(
                    "strict artifact identity mismatch: package metadata entry `{}` is not pinned in lockfile",
                    package.name
                ));
            }
            std::cmp::Ordering::Equal => {
                if lock.version != package.version {
                    return Some(format!(
                        "strict artifact identity mismatch for `{}`: lockfile version `{}` does not match package metadata version `{}`",
                        lock.name, lock.version, package.version
                    ));
                }
                if lock.digest != package.digest {
                    return Some(format!(
                        "strict artifact identity mismatch for `{}`: lockfile digest `{}` does not match package metadata digest `{}`",
                        lock.name, lock.digest, package.digest
                    ));
                }
                lock_idx += 1;
                package_idx += 1;
            }
        }
    }
    if let Some(lock) = lockfile.dependencies.get(lock_idx) {
        return Some(format!(
            "strict artifact identity mismatch: lockfile dependency `{}` is missing from package metadata",
            lock.name
        ));
    }
    if let Some(package) = package_contract.packages.get(package_idx) {
        return Some(format!(
            "strict artifact identity mismatch: package metadata entry `{}` is not pinned in lockfile",
            package.name
        ));
    }
    None
}

fn default_assurance_manifest_path(sig_path: &Path) -> PathBuf {
    let file_name = sig_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("assurance-manifest.json");
    let replacement = if let Some(stripped) = file_name.strip_suffix(".sig.json") {
        format!("{stripped}.assurance.json")
    } else if let Some(stem) = sig_path.file_stem().and_then(|stem| stem.to_str()) {
        format!("{stem}.assurance.json")
    } else {
        "assurance-manifest.json".to_string()
    };
    match sig_path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent.join(replacement),
        _ => PathBuf::from(replacement),
    }
}

