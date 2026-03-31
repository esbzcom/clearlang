fn evaluate_release_policy(
    manifest_path: &Path,
    policy_path: &Path,
    verified_signature_payload: &serde_json::Value,
    pubkey_path: &Path,
) -> std::result::Result<ReleasePolicyOutcome, signing::VerifyError> {
    let manifest =
        signing::verify_assurance_manifest(manifest_path, pubkey_path).map_err(|err| {
            signing::VerifyError::new(
                signing::VerifyErrorCode::PolicyFailure,
                format!("release policy manifest validation failed: {err}"),
            )
        })?;
    let manifest_payload = manifest.payload.as_object().ok_or_else(|| {
        signing::VerifyError::new(
            signing::VerifyErrorCode::PolicyFailure,
            "assurance manifest payload must be a JSON object",
        )
    })?;
    let format = payload_str(manifest_payload, "format").unwrap_or_default();
    if format != "clg.assurance_manifest.v1" {
        return Err(signing::VerifyError::new(
            signing::VerifyErrorCode::PolicyFailure,
            format!(
                "unsupported assurance manifest format `{}` (expected `clg.assurance_manifest.v1`)",
                format
            ),
        ));
    }

    let artifacts = manifest_payload
        .get("artifacts")
        .and_then(|value| value.as_object())
        .ok_or_else(|| {
            signing::VerifyError::new(
                signing::VerifyErrorCode::PolicyFailure,
                "assurance manifest missing `artifacts` object",
            )
        })?;
    let manifest_module_hash = payload_str(artifacts, "module_hash").ok_or_else(|| {
        signing::VerifyError::new(
            signing::VerifyErrorCode::PolicyFailure,
            "assurance manifest missing artifacts.module_hash",
        )
    })?;
    let manifest_proofs_hash = payload_str(artifacts, "proofs_hash").ok_or_else(|| {
        signing::VerifyError::new(
            signing::VerifyErrorCode::PolicyFailure,
            "assurance manifest missing artifacts.proofs_hash",
        )
    })?;

    let sig_payload = verified_signature_payload.as_object().ok_or_else(|| {
        signing::VerifyError::new(
            signing::VerifyErrorCode::PolicyFailure,
            "signature payload must be a JSON object",
        )
    })?;
    let payload_module_hash = payload_str(sig_payload, "module_hash").ok_or_else(|| {
        signing::VerifyError::new(
            signing::VerifyErrorCode::PolicyFailure,
            "signature payload missing module_hash",
        )
    })?;
    let payload_proofs_hash = payload_str(sig_payload, "proofs_hash").ok_or_else(|| {
        signing::VerifyError::new(
            signing::VerifyErrorCode::PolicyFailure,
            "signature payload missing proofs_hash",
        )
    })?;

    if manifest_module_hash != payload_module_hash || manifest_proofs_hash != payload_proofs_hash {
        return Err(signing::VerifyError::new(
            signing::VerifyErrorCode::PolicyFailure,
            "assurance manifest does not match verified signature payload hashes",
        ));
    }

    let policy_bytes = fs::read(policy_path).map_err(|err| {
        signing::VerifyError::new(
            signing::VerifyErrorCode::PolicyFailure,
            format!("reading release policy {}: {err}", policy_path.display()),
        )
    })?;
    let policy: ReleasePolicyFile = serde_json::from_slice(&policy_bytes).map_err(|err| {
        signing::VerifyError::new(
            signing::VerifyErrorCode::PolicyFailure,
            format!("parsing release policy {}: {err}", policy_path.display()),
        )
    })?;
    if policy.schema_version != 1 {
        return Err(signing::VerifyError::new(
            signing::VerifyErrorCode::PolicyFailure,
            format!(
                "unsupported release policy schema_version {} (expected 1)",
                policy.schema_version
            ),
        ));
    }
    let required_tier =
        normalize_assurance_tier(&policy.minimum_assurance_tier).ok_or_else(|| {
            signing::VerifyError::new(
                signing::VerifyErrorCode::PolicyFailure,
                format!(
                    "invalid release policy minimum_assurance_tier `{}` (expected L0|L1|L2|L3)",
                    policy.minimum_assurance_tier
                ),
            )
        })?;

    let manifest_assurance = manifest_payload
        .get("assurance")
        .and_then(|value| value.as_object())
        .ok_or_else(|| {
            signing::VerifyError::new(
                signing::VerifyErrorCode::PolicyFailure,
                "assurance manifest missing `assurance` object",
            )
        })?;
    let manifest_tier_raw = payload_str(manifest_assurance, "tier").ok_or_else(|| {
        signing::VerifyError::new(
            signing::VerifyErrorCode::PolicyFailure,
            "assurance manifest missing assurance.tier",
        )
    })?;
    let manifest_tier = normalize_assurance_tier(manifest_tier_raw).ok_or_else(|| {
        signing::VerifyError::new(
            signing::VerifyErrorCode::PolicyFailure,
            format!(
                "assurance manifest has invalid assurance.tier `{}` (expected L0|L1|L2|L3)",
                manifest_tier_raw
            ),
        )
    })?;

    if assurance_tier_rank(&manifest_tier) < assurance_tier_rank(&required_tier) {
        return Err(signing::VerifyError::new(
            signing::VerifyErrorCode::PolicyFailure,
            format!(
                "release policy rejected manifest tier {} below required {}",
                manifest_tier, required_tier
            ),
        ));
    }

    Ok(ReleasePolicyOutcome {
        required_tier,
        manifest_tier,
        policy_path: policy_path.to_path_buf(),
        manifest_path: manifest_path.to_path_buf(),
    })
}

fn evaluate_required_assurance(
    required_raw: &str,
    module_path: &Path,
    verified_signature_payload: &serde_json::Value,
    assurance_manifest_path: Option<&Path>,
    pubkey_path: &Path,
) -> std::result::Result<AssuranceRequirementOutcome, signing::VerifyError> {
    let required_assurance = normalize_required_assurance(required_raw).ok_or_else(|| {
        signing::VerifyError::new(
            signing::VerifyErrorCode::PolicyFailure,
            format!(
                "invalid `--require-assurance` value `{}` (expected `proved_all`)",
                required_raw
            ),
        )
    })?;

    let sig_payload = verified_signature_payload.as_object().ok_or_else(|| {
        signing::VerifyError::new(
            signing::VerifyErrorCode::PolicyFailure,
            "signature payload must be a JSON object",
        )
    })?;
    let signature_status_raw = payload_str(sig_payload, "proof_status").ok_or_else(|| {
        signing::VerifyError::new(
            signing::VerifyErrorCode::PolicyFailure,
            "signature payload missing proof_status",
        )
    })?;
    let signature_proof_status =
        normalize_proof_status(signature_status_raw).ok_or_else(|| {
            signing::VerifyError::new(
                signing::VerifyErrorCode::PolicyFailure,
                format!(
                    "signature payload has invalid proof_status `{}` (expected `proved_all|not_proved_all`)",
                    signature_status_raw
                ),
            )
        })?;
    let signature_assumption_boundaries =
        parse_signature_assumption_boundaries(sig_payload, true).ok_or_else(|| {
            signing::VerifyError::new(
                signing::VerifyErrorCode::PolicyFailure,
                "signature payload has invalid assumption_boundaries (expected required string array)",
            )
        })?;
    let signature_bundle_symbols =
        parse_signature_bundle_symbols(sig_payload, true).ok_or_else(|| {
            signing::VerifyError::new(
                signing::VerifyErrorCode::PolicyFailure,
                "signature payload has invalid bundle_symbols (expected required string array)",
            )
        })?;
    let derived_module_claims = derive_module_proof_claims(module_path)?;
    if signature_proof_status != derived_module_claims.proof_status {
        return Err(signing::VerifyError::new(
            signing::VerifyErrorCode::PolicyFailure,
            format!(
                "signature payload proof_status `{}` does not match module proof section derived proof_status `{}`",
                signature_proof_status, derived_module_claims.proof_status
            ),
        ));
    }
    if signature_assumption_boundaries != derived_module_claims.assumption_boundaries {
        return Err(signing::VerifyError::new(
            signing::VerifyErrorCode::PolicyFailure,
            format!(
                "signature payload assumption_boundaries [{}] do not match module proof section derived boundaries [{}]",
                signature_assumption_boundaries.join(", "),
                derived_module_claims.assumption_boundaries.join(", ")
            ),
        ));
    }
    if signature_bundle_symbols != derived_module_claims.bundle_symbols {
        return Err(signing::VerifyError::new(
            signing::VerifyErrorCode::PolicyFailure,
            format!(
                "signature payload bundle_symbols [{}] do not match module proof section derived bundle_symbols [{}]",
                signature_bundle_symbols.join(", "),
                derived_module_claims.bundle_symbols.join(", ")
            ),
        ));
    }

    let mut manifest_proof_status = None;
    let mut manifest_path_out = None;
    let mut manifest_assumption_boundaries: Option<Vec<String>> = None;
    if let Some(manifest_path) = assurance_manifest_path {
        let manifest =
            signing::verify_assurance_manifest(manifest_path, pubkey_path).map_err(|err| {
                signing::VerifyError::new(
                    signing::VerifyErrorCode::PolicyFailure,
                    format!("assurance requirement manifest validation failed: {err}"),
                )
            })?;
        let payload = manifest.payload.as_object().ok_or_else(|| {
            signing::VerifyError::new(
                signing::VerifyErrorCode::PolicyFailure,
                "assurance manifest payload must be a JSON object",
            )
        })?;
        let manifest_status_raw = payload_str(payload, "proof_status").ok_or_else(|| {
            signing::VerifyError::new(
                signing::VerifyErrorCode::PolicyFailure,
                "assurance manifest payload missing proof_status",
            )
        })?;
        let parsed_manifest_status =
            normalize_proof_status(manifest_status_raw).ok_or_else(|| {
                signing::VerifyError::new(
                    signing::VerifyErrorCode::PolicyFailure,
                    format!(
                        "assurance manifest has invalid proof_status `{}` (expected `proved_all|not_proved_all`)",
                        manifest_status_raw
                    ),
                )
            })?;

        if parsed_manifest_status != signature_proof_status {
            return Err(signing::VerifyError::new(
                signing::VerifyErrorCode::PolicyFailure,
                format!(
                    "assurance manifest proof_status `{}` does not match signature payload proof_status `{}`",
                    parsed_manifest_status, signature_proof_status
                ),
            ));
        }
        let parsed_manifest_assumptions = parse_manifest_assumption_boundaries(payload)?;
        if parsed_manifest_assumptions != derived_module_claims.assumption_boundaries {
            return Err(signing::VerifyError::new(
                signing::VerifyErrorCode::PolicyFailure,
                format!(
                    "assurance manifest assumption boundaries [{}] do not match module proof section derived boundaries [{}]",
                    parsed_manifest_assumptions.join(", "),
                    derived_module_claims.assumption_boundaries.join(", ")
                ),
            ));
        }
        manifest_assumption_boundaries = Some(parsed_manifest_assumptions);
        manifest_proof_status = Some(parsed_manifest_status);
        manifest_path_out = Some(manifest_path.to_path_buf());
    }

    if signature_proof_status != required_assurance {
        return Err(signing::VerifyError::new(
            signing::VerifyErrorCode::PolicyFailure,
            format!(
                "required assurance `{}` not satisfied: artifact proof_status is `{}`",
                required_assurance, signature_proof_status
            ),
        ));
    }
    if required_assurance == "proved_all" {
        if !derived_module_claims.assumption_boundaries.is_empty() {
            return Err(signing::VerifyError::new(
                signing::VerifyErrorCode::PolicyFailure,
                format!(
                    "required assurance `proved_all` requires zero assumption boundaries in module proof section; found [{}]",
                    derived_module_claims.assumption_boundaries.join(", ")
                ),
            ));
        }
        if let Some(boundaries) = manifest_assumption_boundaries {
            if !boundaries.is_empty() {
                return Err(signing::VerifyError::new(
                    signing::VerifyErrorCode::PolicyFailure,
                    format!(
                        "required assurance `proved_all` requires zero assumption boundaries in assurance manifest; found [{}]",
                        boundaries.join(", ")
                    ),
                ));
            }
        }
        let matrix_path = default_proof_matrix_path();
        let proved_allowlist = load_proved_surface_allowlist(&matrix_path).map_err(|err| {
            signing::VerifyError::new(
                signing::VerifyErrorCode::PolicyFailure,
                format!("proof matrix allowlist gate failed: {err:#}"),
            )
        })?;
        let disallowed: Vec<String> = derived_module_claims
            .bundle_symbols
            .into_iter()
            .filter(|symbol| !proved_allowlist.contains(symbol))
            .collect();
        if !disallowed.is_empty() {
            return Err(signing::VerifyError::new(
                signing::VerifyErrorCode::PolicyFailure,
                format!(
                    "required assurance `proved_all` requires bundle symbols in proved allowlist; disallowed [{}]",
                    disallowed.join(", ")
                ),
            ));
        }
    }

    Ok(AssuranceRequirementOutcome {
        required_assurance,
        signature_proof_status,
        manifest_proof_status,
        manifest_path: manifest_path_out,
    })
}

fn parse_signature_assumption_boundaries(
    payload: &serde_json::Map<String, serde_json::Value>,
    required: bool,
) -> Option<Vec<String>> {
    let Some(value) = payload.get("assumption_boundaries") else {
        return if required { None } else { Some(Vec::new()) };
    };
    let items = value.as_array()?;
    let mut out = BTreeSet::new();
    for item in items {
        let id = item.as_str()?.trim();
        if !id.is_empty() {
            out.insert(id.to_string());
        }
    }
    Some(out.into_iter().collect())
}

fn parse_signature_bundle_symbols(
    payload: &serde_json::Map<String, serde_json::Value>,
    required: bool,
) -> Option<Vec<String>> {
    let Some(value) = payload.get("bundle_symbols") else {
        return if required { None } else { Some(Vec::new()) };
    };
    let items = value.as_array()?;
    let mut out = BTreeSet::new();
    for item in items {
        let symbol = item.as_str()?.trim();
        if !symbol.is_empty() {
            out.insert(symbol.to_string());
        }
    }
    Some(out.into_iter().collect())
}

