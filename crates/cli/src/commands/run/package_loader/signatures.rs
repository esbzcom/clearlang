fn load_runtime_package_signatures(
    root: &Path,
) -> Result<HashMap<String, RuntimePackageSignatureEntry>, RuntimePackageLoaderError> {
    let path = root.join(STRICT_PACKAGE_SIGNATURES_FILE);
    if !path.exists() {
        return Err(RuntimePackageLoaderError::new(
            "R014",
            format!(
                "runtime trust gate requires `{}` at `{}`",
                STRICT_PACKAGE_SIGNATURES_FILE,
                path.display()
            ),
        ));
    }
    if !path.is_file() {
        return Err(RuntimePackageLoaderError::new(
            "R014",
            format!(
                "runtime package signatures path `{}` exists but is not a file",
                path.display()
            ),
        ));
    }
    let content = fs::read_to_string(&path).map_err(|err| {
        let _ = err;
        RuntimePackageLoaderError::new(
            "R014",
            format!(
                "reading runtime package signatures `{}` failed",
                path.display()
            ),
        )
    })?;
    let raw: RawSignatureRootV0 = serde_json::from_str(content.as_str()).map_err(|err| {
        let _ = err;
        RuntimePackageLoaderError::new(
            "R014",
            format!(
                "runtime package signatures `{}` do not match schema v0 (`schema_version`, `signatures[]`)",
                path.display()
            ),
        )
    })?;
    if raw.schema_version != 0 {
        return Err(RuntimePackageLoaderError::new(
            "R014",
            format!(
                "runtime package signatures `{}` have unsupported schema_version {}; expected 0",
                path.display(),
                raw.schema_version
            ),
        ));
    }
    let mut entries = HashMap::with_capacity(raw.signatures.len());
    for entry in raw.signatures {
        validate_package_id(entry.name.as_str()).map_err(|msg| {
            RuntimePackageLoaderError::new(
                "R014",
                format!(
                    "runtime package signatures `{}` package `{}` is invalid: {msg}",
                    path.display(),
                    entry.name
                ),
            )
        })?;
        validate_exact_semver(entry.version.as_str()).map_err(|msg| {
            RuntimePackageLoaderError::new(
                "R014",
                format!(
                    "runtime package signatures `{}` package `{}` has invalid version `{}`: {msg}",
                    path.display(),
                    entry.name,
                    entry.version
                ),
            )
        })?;
        validate_sha256_digest(entry.digest.as_str()).map_err(|msg| {
            RuntimePackageLoaderError::new(
                "R014",
                format!(
                    "runtime package signatures `{}` package `{}` has invalid digest `{}`: {msg}",
                    path.display(),
                    entry.name,
                    entry.digest
                ),
            )
        })?;
        if entry.key_id.trim().is_empty() {
            return Err(RuntimePackageLoaderError::new(
                "R014",
                format!(
                    "runtime package signatures `{}` package `{}` has empty key_id",
                    path.display(),
                    entry.name
                ),
            ));
        }
        parse_utc_timestamp_components(entry.signed_at.as_str()).map_err(|msg| {
            RuntimePackageLoaderError::new(
                "R014",
                format!(
                    "runtime package signatures `{}` package `{}` has invalid signed_at `{}`: {msg}",
                    path.display(),
                    entry.name,
                    entry.signed_at
                ),
            )
        })?;
        if entry.signature_format != "ed25519" {
            return Err(RuntimePackageLoaderError::new(
                "R014",
                format!(
                    "runtime package signatures `{}` package `{}` has unsupported signature_format `{}`; expected `ed25519`",
                    path.display(),
                    entry.name,
                    entry.signature_format
                ),
            ));
        }
        decode_signature(entry.signature.as_str()).map_err(|msg| {
            RuntimePackageLoaderError::new(
                "R014",
                format!(
                    "runtime package signatures `{}` package `{}` has invalid signature: {msg}",
                    path.display(),
                    entry.name
                ),
            )
        })?;
        let id = format!("{}@{}", entry.name, entry.version);
        let value = RuntimePackageSignatureEntry {
            digest: entry.digest,
            key_id: entry.key_id,
            signed_at: entry.signed_at,
            signature: entry.signature,
        };
        if entries.insert(id.clone(), value).is_some() {
            return Err(RuntimePackageLoaderError::new(
                "R014",
                format!(
                    "runtime package signatures `{}` have duplicate package id `{}`",
                    path.display(),
                    id
                ),
            ));
        }
    }
    Ok(entries)
}

