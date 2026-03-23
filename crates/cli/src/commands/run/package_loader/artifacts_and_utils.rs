fn resolve_runtime_artifact_with_availability_policy(
    root: &Path,
    package_id: &str,
    expected_digest: &str,
    artifact_path: &str,
    policy: &RuntimeAvailabilityPolicy,
) -> Result<PathBuf, RuntimePackageLoaderError> {
    let mut candidates = Vec::with_capacity(1 + policy.mirror_roots.len());
    candidates.push(root.join(artifact_path));
    for mirror_root in &policy.mirror_roots {
        candidates.push(root.join(mirror_root).join(artifact_path));
    }

    let mut saw_digest_mismatch = false;
    let mut available_digest_mismatches = Vec::new();
    for candidate in &candidates {
        if !candidate.exists() || !candidate.is_file() {
            continue;
        }
        let bytes = match read_artifact_bytes_with_retries(policy.artifact_read_retries, || {
            fs::read(candidate)
        }) {
            Some(value) => value,
            None => continue,
        };
        let actual_digest = format!("sha256:{}", sha256_hex(bytes.as_slice()));
        if actual_digest != expected_digest {
            saw_digest_mismatch = true;
            available_digest_mismatches.push(format!("{}=>{}", candidate.display(), actual_digest));
            continue;
        }
        return Ok(candidate.clone());
    }

    if saw_digest_mismatch {
        let details = available_digest_mismatches.join(", ");
        return Err(RuntimePackageLoaderError::new(
            "R013",
            format!(
                "runtime package artifact `{}` digest mismatch across configured availability roots; expected `{}` ({})",
                package_id, expected_digest, details
            ),
        ));
    }

    let attempted = candidates
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>()
        .join(", ");
    Err(RuntimePackageLoaderError::new(
        "R012",
        format!(
            "runtime package artifact `{}` was unavailable at all configured roots: {}",
            package_id, attempted
        ),
    ))
}

fn read_artifact_bytes_with_retries<F>(retries: u32, mut reader: F) -> Option<Vec<u8>>
where
    F: FnMut() -> Result<Vec<u8>, std::io::Error>,
{
    for _ in 0..retries {
        if let Ok(bytes) = reader() {
            return Some(bytes);
        }
    }
    None
}

fn load_package_store_index(root: &Path) -> Result<PackageStoreIndexV0, RuntimePackageLoaderError> {
    let index_path = root.join(PACKAGE_STORE_INDEX_FILE);
    if !index_path.exists() {
        return Err(RuntimePackageLoaderError::new(
            "R012",
            format!(
                "trusted package store index `{}` is missing",
                index_path.display()
            ),
        ));
    }
    if !index_path.is_file() {
        return Err(RuntimePackageLoaderError::new(
            "R012",
            format!(
                "trusted package store index path `{}` exists but is not a file",
                index_path.display()
            ),
        ));
    }

    let text = fs::read_to_string(&index_path).map_err(|err| {
        let _ = err;
        RuntimePackageLoaderError::new("R012", format!("reading {} failed", index_path.display()))
    })?;
    let index: PackageStoreIndexV0 = serde_json::from_str(text.as_str()).map_err(|err| {
        let _ = err;
        RuntimePackageLoaderError::new("R012", format!("parsing {} failed", index_path.display()))
    })?;
    if index.schema_version != 0 {
        return Err(RuntimePackageLoaderError::new(
            "R012",
            format!(
                "trusted package store index `{}` has unsupported schema_version {}; expected 0",
                index_path.display(),
                index.schema_version
            ),
        ));
    }
    Ok(index)
}

fn validate_runtime_link_hash(value: &str) -> Result<(), String> {
    if value.len() != 64 {
        return Err("hash must have exactly 64 lowercase hex characters".to_string());
    }
    if !value
        .chars()
        .all(|ch| ch.is_ascii_digit() || ('a'..='f').contains(&ch))
    {
        return Err("hash must be lowercase hex (`0-9`, `a-f`)".to_string());
    }
    Ok(())
}

fn validate_runtime_package_id(value: &str) -> Result<(), String> {
    let Some((name, version)) = value.rsplit_once('@') else {
        return Err("expected `name@version` format".to_string());
    };
    validate_package_id(name)?;
    validate_exact_semver(version)
}

fn split_runtime_package_id(value: &str) -> Result<(&str, &str), String> {
    let Some((name, version)) = value.rsplit_once('@') else {
        return Err("expected `name@version` format".to_string());
    };
    Ok((name, version))
}

fn validate_relative_artifact_path(path: &str) -> Result<(), String> {
    if path.trim().is_empty() {
        return Err("artifact path is empty".to_string());
    }
    let path_ref = Path::new(path);
    if path_ref.is_absolute() {
        return Err("artifact path must be relative".to_string());
    }
    for component in path_ref.components() {
        match component {
            Component::ParentDir => {
                return Err(
                    "artifact path must not contain parent-directory traversal (`..`)".to_string(),
                )
            }
            Component::Prefix(_) | Component::RootDir => {
                return Err("artifact path must be relative".to_string())
            }
            Component::CurDir | Component::Normal(_) => {}
        }
    }
    Ok(())
}

fn enforce_runtime_link_deterministic_shape(link: &RuntimeLinkRootV0) -> Result<(), String> {
    let mut prev_pkg: Option<&str> = None;
    let mut seen_packages = std::collections::HashSet::with_capacity(link.packages.len());
    for pkg in &link.packages {
        if let Some(prev) = prev_pkg {
            if pkg.id.as_str() < prev {
                return Err(format!(
                    "packages must be sorted by id (found `{}` before `{}`)",
                    prev, pkg.id
                ));
            }
        }
        if !seen_packages.insert(pkg.id.clone()) {
            return Err(format!("duplicate package id `{}`", pkg.id));
        }
        prev_pkg = Some(pkg.id.as_str());
    }

    let mut prev_binding: Option<(&str, &str, &str, &str)> = None;
    let mut seen_bindings = std::collections::HashSet::with_capacity(link.bindings.len());
    for binding in &link.bindings {
        if binding.import_module.trim().is_empty()
            || binding.import_name.trim().is_empty()
            || binding.provider_package_id.trim().is_empty()
            || binding.provider_symbol.trim().is_empty()
        {
            return Err("binding fields must be non-empty".to_string());
        }
        let current = (
            binding.import_module.as_str(),
            binding.import_name.as_str(),
            binding.provider_package_id.as_str(),
            binding.provider_symbol.as_str(),
        );
        if let Some(prev) = prev_binding {
            if current < prev {
                return Err("bindings must be sorted by (import_module, import_name, provider_package_id, provider_symbol)".to_string());
            }
        }
        if !seen_bindings.insert(current) {
            return Err(format!(
                "duplicate binding ({}, {}, {}, {})",
                current.0, current.1, current.2, current.3
            ));
        }
        prev_binding = Some(current);
    }
    Ok(())
}

fn verifying_key_from_hex_public_key(value: &str) -> Result<VerifyingKey, &'static str> {
    const PREFIX: &str = "hex:";
    if !value.starts_with(PREFIX) {
        return Err("public_key must start with `hex:`");
    }
    let key_hex = &value[PREFIX.len()..];
    let key_raw = hex::decode(key_hex).map_err(|_| "public_key is not valid hex")?;
    let key_bytes: [u8; 32] = key_raw
        .try_into()
        .map_err(|_| "public_key must be exactly 32 bytes")?;
    VerifyingKey::from_bytes(&key_bytes).map_err(|_| "public_key bytes are invalid")
}

fn decode_signature(value: &str) -> Result<Signature, &'static str> {
    let raw = hex::decode(value).map_err(|_| "signature is not valid hex")?;
    Signature::try_from(raw.as_slice()).map_err(|_| "signature must be exactly 64 bytes")
}

fn canonical_signature_payload_v0(
    name: &str,
    version: &str,
    digest: &str,
    signed_at: &str,
) -> String {
    format!("clg-package-signature-v0\n{name}\n{version}\n{digest}\n{signed_at}\n")
}
