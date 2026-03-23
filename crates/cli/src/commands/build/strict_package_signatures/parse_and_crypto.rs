fn load_package_signatures_v0(
    root: &Path,
    required: bool,
) -> Result<Vec<StrictPackageSignatureEntry>, StrictPackageSignaturesError> {
    let path = root.join(STRICT_PACKAGE_SIGNATURES_FILE);
    if !path.exists() {
        if required {
            return Err(StrictPackageSignaturesError::new(
                "C103",
                format!(
                    "strict mode requires `{}` at `{}` when package metadata contains dependencies",
                    STRICT_PACKAGE_SIGNATURES_FILE,
                    path.display()
                ),
            ));
        }
        return Ok(Vec::new());
    }
    if !path.is_file() {
        return Err(StrictPackageSignaturesError::new(
            "C103",
            format!(
                "strict package signatures path `{}` exists but is not a file",
                path.display()
            ),
        ));
    }
    let content = fs::read_to_string(&path).map_err(|err| {
        StrictPackageSignaturesError::new(
            "C103",
            format!(
                "failed to read strict package signatures `{}`: {err}",
                path.display()
            ),
        )
    })?;
    parse_package_signatures_v0(&content, &path)
}

fn parse_package_signatures_v0(
    content: &str,
    path: &Path,
) -> Result<Vec<StrictPackageSignatureEntry>, StrictPackageSignaturesError> {
    let value: serde_json::Value = serde_json::from_str(content).map_err(|_| {
        StrictPackageSignaturesError::new(
            "C103",
            format!(
                "strict package signatures `{}` are not valid JSON (expected schema v0 object)",
                path.display()
            ),
        )
    })?;
    let raw: RawSignatureRoot = serde_json::from_value(value).map_err(|_| {
        StrictPackageSignaturesError::new(
            "C103",
            format!(
                "strict package signatures `{}` do not match schema v0 (`schema_version`, `signatures[]`)",
                path.display()
            ),
        )
    })?;
    if raw.schema_version != 0 {
        return Err(StrictPackageSignaturesError::new(
            "C103",
            format!(
                "strict package signatures `{}` have unsupported schema_version {}; expected 0",
                path.display(),
                raw.schema_version
            ),
        ));
    }

    let mut signatures: Vec<StrictPackageSignatureEntry> = raw
        .signatures
        .into_iter()
        .map(|entry| StrictPackageSignatureEntry {
            name: entry.name,
            version: entry.version,
            digest: entry.digest,
            key_id: entry.key_id,
            signed_at: entry.signed_at,
            signature: entry.signature,
        })
        .collect();
    signatures.sort_by(|a, b| a.name.cmp(&b.name));

    let mut seen = HashSet::with_capacity(signatures.len());
    let mut duplicates = BTreeSet::new();
    for signature in &signatures {
        if !seen.insert(signature.name.clone()) {
            duplicates.insert(signature.name.clone());
        }
    }
    if let Some(dupe) = duplicates.iter().next() {
        return Err(StrictPackageSignaturesError::new(
            "C103",
            format!(
                "strict package signatures `{}` have duplicate package name `{}`",
                path.display(),
                dupe
            ),
        ));
    }

    for entry in &signatures {
        validate_package_id(entry.name.as_str()).map_err(|msg| {
            StrictPackageSignaturesError::new(
                "C103",
                format!(
                    "strict package signatures `{}` package `{}` is invalid: {msg}",
                    path.display(),
                    entry.name
                ),
            )
        })?;
        validate_exact_semver(entry.version.as_str()).map_err(|msg| {
            StrictPackageSignaturesError::new(
                "C103",
                format!(
                    "strict package signatures `{}` package `{}` has invalid version `{}`: {msg}",
                    path.display(),
                    entry.name,
                    entry.version
                ),
            )
        })?;
        validate_sha256_digest(entry.digest.as_str()).map_err(|msg| {
            StrictPackageSignaturesError::new(
                "C103",
                format!(
                    "strict package signatures `{}` package `{}` has invalid digest `{}`: {msg}",
                    path.display(),
                    entry.name,
                    entry.digest
                ),
            )
        })?;
        if entry.key_id.trim().is_empty() {
            return Err(StrictPackageSignaturesError::new(
                "C103",
                format!(
                    "strict package signatures `{}` package `{}` has empty key_id",
                    path.display(),
                    entry.name
                ),
            ));
        }
        parse_rfc3339_utc(entry.signed_at.as_str(), "signed_at").map_err(|_| {
            StrictPackageSignaturesError::new(
                "C103",
                format!(
                    "strict package signatures `{}` package `{}` has invalid signed_at `{}`",
                    path.display(),
                    entry.name,
                    entry.signed_at
                ),
            )
        })?;
    }

    // Validate signature_format from raw input while still preserving deny_unknown_fields behavior.
    let raw_again: RawSignatureRoot = serde_json::from_str(content).map_err(|_| {
        StrictPackageSignaturesError::new(
            "C103",
            format!(
                "strict package signatures `{}` are not valid JSON (expected schema v0 object)",
                path.display()
            ),
        )
    })?;
    for entry in &raw_again.signatures {
        if entry.signature_format != "ed25519" {
            return Err(StrictPackageSignaturesError::new(
                "C103",
                format!(
                    "strict package signatures `{}` package `{}` has unsupported signature_format `{}`; expected `ed25519`",
                    path.display(),
                    entry.name,
                    entry.signature_format
                ),
            ));
        }
        decode_signature(entry.signature.as_str()).map_err(|msg| {
            StrictPackageSignaturesError::new(
                "C103",
                format!(
                    "strict package signatures `{}` package `{}` has invalid signature: {msg}",
                    path.display(),
                    entry.name
                ),
            )
        })?;
    }

    Ok(signatures)
}

fn parse_rfc3339_utc(
    value: &str,
    field: &str,
) -> Result<(u16, u8, u8, u8, u8, u8), StrictPackageSignaturesError> {
    parse_utc_timestamp_components(value).map_err(|msg| {
        StrictPackageSignaturesError::new(
            "C103",
            format!("{field} must be a valid UTC RFC3339 timestamp: {msg}"),
        )
    })
}

fn verifying_key_from_signer(signer: &TrustedSignerV0) -> Result<VerifyingKey, &'static str> {
    const PREFIX: &str = "hex:";
    if !signer.public_key.starts_with(PREFIX) {
        return Err("public_key must start with `hex:`");
    }
    let key_hex = &signer.public_key[PREFIX.len()..];
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

fn canonical_payload(name: &str, version: &str, digest: &str, signed_at: &str) -> String {
    format!("clg-package-signature-v0\n{name}\n{version}\n{digest}\n{signed_at}\n")
}
