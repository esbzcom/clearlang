pub(super) fn enforce_trust_gate_v0(
    root: &Path,
    package_contract: &StrictPackageContractV0,
    trust_policy: &StrictTrustPolicyV0,
) -> Result<(), StrictPackageSignaturesError> {
    let require_signatures = !package_contract.packages.is_empty();
    let signatures = load_package_signatures_v0(root, require_signatures)?;

    let mut signatures_by_name: HashMap<&str, &StrictPackageSignatureEntry> =
        HashMap::with_capacity(signatures.len());
    for signature in &signatures {
        signatures_by_name.insert(signature.name.as_str(), signature);
    }

    let trust_signers: HashMap<&str, &TrustedSignerV0> = trust_policy
        .trusted_signers
        .iter()
        .map(|signer| (signer.key_id.as_str(), signer))
        .collect();
    let revoked: HashSet<&str> = trust_policy
        .revoked_key_ids
        .iter()
        .map(|key_id| key_id.as_str())
        .collect();

    let mut used_signature_names = HashSet::with_capacity(package_contract.packages.len());
    for package in &package_contract.packages {
        let signature = signatures_by_name
            .get(package.name.as_str())
            .copied()
            .ok_or_else(|| {
                StrictPackageSignaturesError::new(
                    "C103",
                    format!(
                        "strict trust gate failed: package `{}` is missing signature entry in `{}`",
                        package.name, STRICT_PACKAGE_SIGNATURES_FILE
                    ),
                )
            })?;
        used_signature_names.insert(signature.name.as_str());

        if signature.version != package.version {
            return Err(StrictPackageSignaturesError::new(
                "C103",
                format!(
                    "strict trust gate failed for `{}`: signature version `{}` does not match package version `{}`",
                    package.name, signature.version, package.version
                ),
            ));
        }
        if signature.digest != package.digest {
            return Err(StrictPackageSignaturesError::new(
                "C103",
                format!(
                    "strict trust gate failed for `{}`: signature digest `{}` does not match package digest `{}`",
                    package.name, signature.digest, package.digest
                ),
            ));
        }
        if let Some(meta_signature) = package.signature.as_ref() {
            if meta_signature.format != "ed25519" {
                return Err(StrictPackageSignaturesError::new(
                    "C103",
                    format!(
                        "strict trust gate failed for `{}`: metadata signature format `{}` is unsupported; expected `ed25519`",
                        package.name, meta_signature.format
                    ),
                ));
            }
            if meta_signature.key_id != signature.key_id {
                return Err(StrictPackageSignaturesError::new(
                    "C103",
                    format!(
                        "strict trust gate failed for `{}`: metadata signature key_id `{}` does not match package signatures key_id `{}`",
                        package.name, meta_signature.key_id, signature.key_id
                    ),
                ));
            }
            if meta_signature.signed_at != signature.signed_at {
                return Err(StrictPackageSignaturesError::new(
                    "C103",
                    format!(
                        "strict trust gate failed for `{}`: metadata signed_at `{}` does not match package signatures signed_at `{}`",
                        package.name, meta_signature.signed_at, signature.signed_at
                    ),
                ));
            }
            if meta_signature.signature != signature.signature {
                return Err(StrictPackageSignaturesError::new(
                    "C103",
                    format!(
                        "strict trust gate failed for `{}`: metadata signature does not match package signatures entry",
                        package.name
                    ),
                ));
            }
        }

        let signer = trust_signers
            .get(signature.key_id.as_str())
            .copied()
            .ok_or_else(|| {
                StrictPackageSignaturesError::new(
                    "C103",
                    format!(
                        "strict trust gate failed for `{}`: signer `{}` is not trusted",
                        package.name, signature.key_id
                    ),
                )
            })?;
        if revoked.contains(signature.key_id.as_str()) {
            return Err(StrictPackageSignaturesError::new(
                "C103",
                format!(
                    "strict trust gate failed for `{}`: signer `{}` is revoked",
                    package.name, signature.key_id
                ),
            ));
        }
        if !package.trusted_anchor_ids.is_empty() {
            if !package
                .trusted_anchor_ids
                .iter()
                .any(|anchor| anchor == &signature.key_id)
            {
                return Err(StrictPackageSignaturesError::new(
                    "C103",
                    format!(
                        "strict trust gate failed for `{}`: signer `{}` is not permitted by package trusted anchors",
                        package.name, signature.key_id
                    ),
                ));
            }
            for anchor in &package.trusted_anchor_ids {
                if !trust_signers.contains_key(anchor.as_str()) {
                    return Err(StrictPackageSignaturesError::new(
                        "C103",
                        format!(
                            "strict trust gate failed for `{}`: trusted anchor `{}` is not present in trust policy",
                            package.name, anchor
                        ),
                    ));
                }
                if revoked.contains(anchor.as_str()) {
                    return Err(StrictPackageSignaturesError::new(
                        "C103",
                        format!(
                            "strict trust gate failed for `{}`: trusted anchor `{}` is revoked",
                            package.name, anchor
                        ),
                    ));
                }
            }
        }

        let signed_at = parse_rfc3339_utc(&signature.signed_at, "signed_at")?;
        let not_before = parse_rfc3339_utc(&signer.not_before, "not_before")?;
        let not_after = parse_rfc3339_utc(&signer.not_after, "not_after")?;
        if signed_at < not_before || signed_at >= not_after {
            return Err(StrictPackageSignaturesError::new(
                "C103",
                format!(
                    "strict trust gate failed for `{}`: signer `{}` is not valid at signed_at `{}`",
                    package.name, signature.key_id, signature.signed_at
                ),
            ));
        }

        let verifying = verifying_key_from_signer(signer).map_err(|msg| {
            StrictPackageSignaturesError::new(
                "C103",
                format!(
                    "strict trust gate failed for `{}`: signer `{}` public key is invalid: {}",
                    package.name, signer.key_id, msg
                ),
            )
        })?;
        let signature_bytes = decode_signature(&signature.signature).map_err(|msg| {
            StrictPackageSignaturesError::new(
                "C103",
                format!(
                    "strict trust gate failed for `{}`: signature is invalid: {}",
                    package.name, msg
                ),
            )
        })?;
        let payload = canonical_payload(
            package.name.as_str(),
            package.version.as_str(),
            package.digest.as_str(),
            signature.signed_at.as_str(),
        );
        verifying
            .verify_strict(payload.as_bytes(), &signature_bytes)
            .map_err(|_| {
                StrictPackageSignaturesError::new(
                    "C103",
                    format!(
                        "strict trust gate failed for `{}`: signature verification failed for signer `{}`",
                        package.name, signature.key_id
                    ),
                )
            })?;
    }

    if let Some(extra) = signatures
        .iter()
        .find(|entry| !used_signature_names.contains(entry.name.as_str()))
    {
        return Err(StrictPackageSignaturesError::new(
            "C103",
            format!(
                "strict trust gate failed: signature entry `{}` is not referenced by package metadata",
                extra.name
            ),
        ));
    }

    Ok(())
}
