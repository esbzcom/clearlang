fn load_advisories(
    root: &Path,
    policy: &AdvisoryPolicy,
) -> Result<Vec<AdvisoryEntry>, PkgLockError> {
    let advisory_path = root.join(ADVISORY_FILE);
    if !advisory_path.exists() {
        return Ok(Vec::new());
    }
    if !advisory_path.is_file() {
        return Err(PkgLockError::new(
            "C117",
            format!(
                "advisory input path `{}` exists but is not a file",
                advisory_path.display()
            ),
        ));
    }

    let content = fs::read_to_string(&advisory_path).map_err(|err| {
        PkgLockError::new(
            "C117",
            format!("reading {}: {err}", advisory_path.display()),
        )
    })?;
    let raw: AdvisoryRoot = serde_json::from_str(content.as_str()).map_err(|err| {
        PkgLockError::new(
            "C117",
            format!("parsing {}: {err}", advisory_path.display()),
        )
    })?;
    if raw.schema_version != 1 {
        return Err(PkgLockError::new(
            "C117",
            format!(
                "unsupported advisory schema_version {} in `{}`; expected 1",
                raw.schema_version,
                advisory_path.display()
            ),
        ));
    }
    if policy.is_strict() {
        verify_advisory_signature_strict(root, &raw, advisory_path.as_path())?;
    }

    let mut seen_ids = HashSet::with_capacity(raw.advisories.len());
    let mut duplicate_ids = BTreeSet::new();
    let mut advisories = Vec::with_capacity(raw.advisories.len());
    for item in raw.advisories {
        if item.id.trim().is_empty() {
            return Err(PkgLockError::new(
                "C117",
                format!(
                    "advisory entry in `{}` has empty id",
                    advisory_path.display()
                ),
            ));
        }
        if !seen_ids.insert(item.id.clone()) {
            duplicate_ids.insert(item.id.clone());
        }
        validate_package_id(item.package.as_str()).map_err(|msg| {
            PkgLockError::new(
                "C117",
                format!(
                    "advisory `{}` in `{}` has invalid package `{}`: {msg}",
                    item.id,
                    advisory_path.display(),
                    item.package
                ),
            )
        })?;
        validate_semver_requirement(item.affected.as_str()).map_err(|msg| {
            PkgLockError::new(
                "C117",
                format!(
                    "advisory `{}` in `{}` has invalid affected range `{}`: {msg}",
                    item.id,
                    advisory_path.display(),
                    item.affected
                ),
            )
        })?;
        let affected = ParsedRequirement::parse(item.affected.as_str()).map_err(|_| {
            PkgLockError::new(
                "C117",
                format!(
                    "advisory `{}` in `{}` has invalid affected range `{}`",
                    item.id,
                    advisory_path.display(),
                    item.affected
                ),
            )
        })?;
        if item.severity.trim().is_empty() {
            return Err(PkgLockError::new(
                "C117",
                format!(
                    "advisory `{}` in `{}` has empty severity",
                    item.id,
                    advisory_path.display()
                ),
            ));
        }
        validate_utc_rfc3339("issued_at", item.issued_at.as_str()).map_err(|msg| {
            PkgLockError::new(
                "C117",
                format!(
                    "advisory `{}` in `{}` has invalid issued_at `{}`: {msg}",
                    item.id,
                    advisory_path.display(),
                    item.issued_at
                ),
            )
        })?;
        validate_utc_rfc3339("expires_at", item.expires_at.as_str()).map_err(|msg| {
            PkgLockError::new(
                "C117",
                format!(
                    "advisory `{}` in `{}` has invalid expires_at `{}`: {msg}",
                    item.id,
                    advisory_path.display(),
                    item.expires_at
                ),
            )
        })?;
        let issued_at = parse_utc_timestamp_components(item.issued_at.as_str()).map_err(|msg| {
            PkgLockError::new(
                "C117",
                format!(
                    "advisory `{}` in `{}` has invalid issued_at `{}`: {}",
                    item.id,
                    advisory_path.display(),
                    item.issued_at,
                    msg
                ),
            )
        })?;
        let expires_at =
            parse_utc_timestamp_components(item.expires_at.as_str()).map_err(|msg| {
                PkgLockError::new(
                    "C117",
                    format!(
                        "advisory `{}` in `{}` has invalid expires_at `{}`: {}",
                        item.id,
                        advisory_path.display(),
                        item.expires_at,
                        msg
                    ),
                )
            })?;
        if expires_at <= issued_at {
            return Err(PkgLockError::new(
                "C117",
                format!(
                    "advisory `{}` in `{}` must satisfy issued_at < expires_at",
                    item.id,
                    advisory_path.display()
                ),
            ));
        }

        let action = match item.action.as_str() {
            "deny" => AdvisoryAction::Deny,
            "warn" => AdvisoryAction::Warn,
            "force_upgrade" => AdvisoryAction::ForceUpgrade,
            other => {
                return Err(PkgLockError::new(
                    "C117",
                    format!(
                        "advisory `{}` in `{}` has unsupported action `{}`",
                        item.id,
                        advisory_path.display(),
                        other
                    ),
                ))
            }
        };
        let minimum_safe_version = match (action, item.minimum_safe_version) {
            (AdvisoryAction::ForceUpgrade, Some(version)) => {
                validate_exact_semver(version.as_str()).map_err(|msg| {
                    PkgLockError::new(
                        "C117",
                        format!(
                            "advisory `{}` in `{}` has invalid minimum_safe_version `{}`: {msg}",
                            item.id,
                            advisory_path.display(),
                            version
                        ),
                    )
                })?;
                Some(SemVer::parse(version.as_str()).map_err(|_| {
                    PkgLockError::new(
                        "C117",
                        format!(
                            "advisory `{}` in `{}` has invalid minimum_safe_version `{}`",
                            item.id,
                            advisory_path.display(),
                            version
                        ),
                    )
                })?)
            }
            (AdvisoryAction::ForceUpgrade, None) => {
                return Err(PkgLockError::new(
                    "C117",
                    format!(
                        "advisory `{}` in `{}` requires `minimum_safe_version` for `force_upgrade` action",
                        item.id,
                        advisory_path.display()
                    ),
                ))
            }
            (_, version_opt) => {
                if let Some(version) = version_opt {
                    validate_exact_semver(version.as_str()).map_err(|msg| {
                        PkgLockError::new(
                            "C117",
                            format!(
                                "advisory `{}` in `{}` has invalid minimum_safe_version `{}`: {msg}",
                                item.id,
                                advisory_path.display(),
                                version
                            ),
                        )
                    })?;
                }
                None
            }
        };
        advisories.push(AdvisoryEntry {
            id: item.id,
            package: item.package,
            affected,
            severity: item.severity,
            action,
            minimum_safe_version,
            issued_at,
            expires_at,
        });
    }

    if let Some(duplicate) = duplicate_ids.iter().next() {
        return Err(PkgLockError::new(
            "C117",
            format!(
                "advisory input `{}` has duplicate advisory id `{}`",
                advisory_path.display(),
                duplicate
            ),
        ));
    }
    advisories.sort_by(|a, b| a.package.cmp(&b.package).then_with(|| a.id.cmp(&b.id)));
    Ok(advisories)
}

fn verify_advisory_signature_strict(
    root: &Path,
    raw: &AdvisoryRoot,
    advisory_path: &Path,
) -> Result<(), PkgLockError> {
    let signature = raw.signature.as_ref().ok_or_else(|| {
        PkgLockError::new(
            "C117",
            format!(
                "strict advisory trust gate requires signed advisory envelope in `{}`",
                advisory_path.display()
            ),
        )
    })?;

    if signature.key_id.trim().is_empty() {
        return Err(PkgLockError::new(
            "C117",
            format!(
                "strict advisory trust gate failed for `{}`: signature key_id is empty",
                advisory_path.display()
            ),
        ));
    }
    if signature.signature_format != "ed25519" {
        return Err(PkgLockError::new(
            "C117",
            format!(
                "strict advisory trust gate failed for `{}`: unsupported signature_format `{}`; expected `ed25519`",
                advisory_path.display(),
                signature.signature_format
            ),
        ));
    }
    validate_utc_rfc3339("signed_at", signature.signed_at.as_str()).map_err(|msg| {
        PkgLockError::new(
            "C117",
            format!(
                "strict advisory trust gate failed for `{}`: invalid signed_at `{}`: {msg}",
                advisory_path.display(),
                signature.signed_at
            ),
        )
    })?;
    let signed_at =
        parse_utc_timestamp_components(signature.signed_at.as_str()).map_err(|msg| {
            PkgLockError::new(
                "C117",
                format!(
                    "strict advisory trust gate failed for `{}`: invalid signed_at `{}`: {}",
                    advisory_path.display(),
                    signature.signed_at,
                    msg
                ),
            )
        })?;

    let trust_policy = load_required_trust_policy_v0(root).map_err(|err| {
        PkgLockError::new(
            "C117",
            format!(
                "strict advisory trust gate failed for `{}`: {}",
                advisory_path.display(),
                err.message()
            ),
        )
    })?;

    let signer = trust_policy
        .trusted_signers
        .iter()
        .find(|s| s.key_id == signature.key_id)
        .ok_or_else(|| {
            PkgLockError::new(
                "C117",
                format!(
                    "strict advisory trust gate failed for `{}`: signer `{}` is not trusted",
                    advisory_path.display(),
                    signature.key_id
                ),
            )
        })?;

    if trust_policy
        .revoked_key_ids
        .iter()
        .any(|key_id| key_id == &signature.key_id)
    {
        return Err(PkgLockError::new(
            "C117",
            format!(
                "strict advisory trust gate failed for `{}`: signer `{}` is revoked",
                advisory_path.display(),
                signature.key_id
            ),
        ));
    }

    let not_before = parse_utc_timestamp_components(signer.not_before.as_str()).map_err(|msg| {
        PkgLockError::new(
            "C117",
            format!(
                "strict advisory trust gate failed for `{}`: signer `{}` has invalid not_before `{}`: {}",
                advisory_path.display(),
                signer.key_id,
                signer.not_before,
                msg
            ),
        )
    })?;
    let not_after = parse_utc_timestamp_components(signer.not_after.as_str()).map_err(|msg| {
        PkgLockError::new(
            "C117",
            format!(
                "strict advisory trust gate failed for `{}`: signer `{}` has invalid not_after `{}`: {}",
                advisory_path.display(),
                signer.key_id,
                signer.not_after,
                msg
            ),
        )
    })?;
    if signed_at < not_before || signed_at >= not_after {
        return Err(PkgLockError::new(
            "C117",
            format!(
                "strict advisory trust gate failed for `{}`: signer `{}` is not valid at signed_at `{}`",
                advisory_path.display(),
                signer.key_id,
                signature.signed_at
            ),
        ));
    }

    let verifying = advisory_verifying_key_from_signer(signer).map_err(|msg| {
        PkgLockError::new(
            "C117",
            format!(
                "strict advisory trust gate failed for `{}`: signer `{}` public_key is invalid: {}",
                advisory_path.display(),
                signer.key_id,
                msg
            ),
        )
    })?;
    let signature_bytes =
        decode_advisory_signature(signature.signature.as_str()).map_err(|msg| {
            PkgLockError::new(
                "C117",
                format!(
                    "strict advisory trust gate failed for `{}`: signature is invalid: {}",
                    advisory_path.display(),
                    msg
                ),
            )
        })?;

    let payload = canonical_advisory_signature_payload(
        raw.schema_version,
        raw.advisories.as_slice(),
        signature.signed_at.as_str(),
    )
    .map_err(|msg| {
        PkgLockError::new(
            "C117",
            format!(
                "strict advisory trust gate failed for `{}`: {}",
                advisory_path.display(),
                msg
            ),
        )
    })?;
    verifying
        .verify_strict(payload.as_slice(), &signature_bytes)
        .map_err(|_| {
            PkgLockError::new(
                "C117",
                format!(
                    "strict advisory trust gate failed for `{}`: signature verification failed for signer `{}`",
                    advisory_path.display(),
                    signature.key_id
                ),
            )
        })?;
    Ok(())
}

fn advisory_verifying_key_from_signer(
    signer: &TrustedSignerV0,
) -> Result<VerifyingKey, &'static str> {
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

fn decode_advisory_signature(value: &str) -> Result<Signature, &'static str> {
    let raw = hex::decode(value).map_err(|_| "signature is not valid hex")?;
    Signature::try_from(raw.as_slice()).map_err(|_| "signature must be exactly 64 bytes")
}

fn canonical_advisory_signature_payload(
    schema_version: u32,
    advisories: &[RawAdvisoryEntry],
    signed_at: &str,
) -> Result<Vec<u8>, String> {
    let mut normalized = advisories.to_vec();
    normalized.sort_by(|a, b| a.package.cmp(&b.package).then_with(|| a.id.cmp(&b.id)));
    let payload = serde_json::json!({
        "schema_version": schema_version,
        "advisories": normalized,
        "signed_at": signed_at,
    });
    serde_json::to_value(payload)
        .map(|value| canonical_json_bytes(&value))
        .map_err(|err| format!("failed to canonicalize advisory payload: {err}"))
}

