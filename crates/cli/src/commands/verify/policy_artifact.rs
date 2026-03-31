#[derive(Debug)]
struct ReleasePolicyOutcome {
    required_tier: String,
    manifest_tier: String,
    policy_path: PathBuf,
    manifest_path: PathBuf,
}

#[derive(Debug)]
struct AssuranceRequirementOutcome {
    required_assurance: String,
    signature_proof_status: String,
    manifest_proof_status: Option<String>,
    manifest_path: Option<PathBuf>,
}

#[derive(Debug)]
struct ProofArtifactConsistencyOutcome {
    proof_artifact_path: PathBuf,
    proof_artifact_hash: String,
    solver_profile_hash: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct ReleasePolicyFile {
    schema_version: u32,
    minimum_assurance_tier: String,
}

fn evaluate_proof_artifact_consistency(
    verified_signature_payload: &serde_json::Value,
    proof_artifact_path: Option<&Path>,
    assurance_manifest_path: Option<&Path>,
    pubkey_path: &Path,
) -> std::result::Result<Option<ProofArtifactConsistencyOutcome>, signing::VerifyError> {
    let sig_payload = verified_signature_payload.as_object().ok_or_else(|| {
        signing::VerifyError::new(
            signing::VerifyErrorCode::ProofArtifactFailure,
            "signature payload must be a JSON object",
        )
    })?;
    let payload_proof_artifact_hash = payload_str(sig_payload, "proof_artifact_hash")
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    let payload_solver_profile_hash = payload_str(sig_payload, "solver_profile_hash")
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    let payload_claim_present =
        payload_proof_artifact_hash.is_some() || payload_solver_profile_hash.is_some();
    if !payload_claim_present {
        return Ok(None);
    }

    let mut manifest_proof_artifact_hash = None;
    let mut manifest_solver_profile_hash = None;
    if let Some(manifest_path) = assurance_manifest_path {
        let manifest =
            signing::verify_assurance_manifest(manifest_path, pubkey_path).map_err(|err| {
                signing::VerifyError::new(
                    signing::VerifyErrorCode::ProofArtifactFailure,
                    format!("proof-artifact manifest validation failed: {err}"),
                )
            })?;
        let payload = manifest.payload.as_object().ok_or_else(|| {
            signing::VerifyError::new(
                signing::VerifyErrorCode::ProofArtifactFailure,
                "assurance manifest payload must be a JSON object",
            )
        })?;
        let artifacts = payload
            .get("artifacts")
            .and_then(|value| value.as_object())
            .ok_or_else(|| {
                signing::VerifyError::new(
                    signing::VerifyErrorCode::ProofArtifactFailure,
                    "assurance manifest missing `artifacts` object",
                )
            })?;
        manifest_proof_artifact_hash = payload_str(artifacts, "proof_artifact_hash")
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);
        manifest_solver_profile_hash = payload_str(artifacts, "solver_profile_hash")
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);
    }

    if let Some(manifest_hash) = manifest_proof_artifact_hash.as_ref() {
        let Some(payload_hash) = payload_proof_artifact_hash.as_ref() else {
            return Err(signing::VerifyError::new(
                signing::VerifyErrorCode::ProofArtifactFailure,
                "assurance manifest includes artifacts.proof_artifact_hash but signature payload does not",
            ));
        };
        if manifest_hash != payload_hash {
            return Err(signing::VerifyError::new(
                signing::VerifyErrorCode::ProofArtifactFailure,
                "assurance manifest proof_artifact_hash does not match signature payload",
            ));
        }
    }
    if let Some(manifest_hash) = manifest_solver_profile_hash.as_ref() {
        let Some(payload_hash) = payload_solver_profile_hash.as_ref() else {
            return Err(signing::VerifyError::new(
                signing::VerifyErrorCode::ProofArtifactFailure,
                "assurance manifest includes artifacts.solver_profile_hash but signature payload does not",
            ));
        };
        if manifest_hash != payload_hash {
            return Err(signing::VerifyError::new(
                signing::VerifyErrorCode::ProofArtifactFailure,
                "assurance manifest solver_profile_hash does not match signature payload",
            ));
        }
    }

    let proof_claim_present =
        payload_proof_artifact_hash.is_some() || manifest_proof_artifact_hash.is_some();
    let solver_claim_present =
        payload_solver_profile_hash.is_some() || manifest_solver_profile_hash.is_some();
    if !proof_claim_present && !solver_claim_present {
        return Ok(None);
    }

    let Some(path) = proof_artifact_path else {
        return Err(signing::VerifyError::new(
            signing::VerifyErrorCode::ProofArtifactFailure,
            "proof artifact claim is present; provide `--proof-artifact <FILE>`",
        ));
    };
    let bytes = fs::read(path).map_err(|err| {
        signing::VerifyError::new(
            signing::VerifyErrorCode::ProofArtifactFailure,
            format!("reading proof artifact {}: {err}", path.display()),
        )
    })?;
    let artifact_value: serde_json::Value = serde_json::from_slice(&bytes).map_err(|err| {
        signing::VerifyError::new(
            signing::VerifyErrorCode::ProofArtifactFailure,
            format!("parsing proof artifact {}: {err}", path.display()),
        )
    })?;
    let canonical_hash = format!(
        "sha256:{}",
        sha256_hex(&canonical_json_bytes(&artifact_value))
    );

    if let Some(expected) = payload_proof_artifact_hash.as_ref() {
        if expected != &canonical_hash {
            return Err(signing::VerifyError::new(
                signing::VerifyErrorCode::ProofArtifactFailure,
                format!(
                    "proof artifact hash mismatch: signature payload expects `{expected}`, actual `{canonical_hash}`"
                ),
            ));
        }
    }
    if let Some(expected) = manifest_proof_artifact_hash.as_ref() {
        if expected != &canonical_hash {
            return Err(signing::VerifyError::new(
                signing::VerifyErrorCode::ProofArtifactFailure,
                format!(
                    "proof artifact hash mismatch: assurance manifest expects `{expected}`, actual `{canonical_hash}`"
                ),
            ));
        }
    }

    let artifact_obj = artifact_value.as_object().ok_or_else(|| {
        signing::VerifyError::new(
            signing::VerifyErrorCode::ProofArtifactFailure,
            "proof artifact root must be a JSON object",
        )
    })?;
    let artifact_solver_profile_hash = payload_str(artifact_obj, "solver_profile_hash")
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    if let Some(expected) = payload_solver_profile_hash.as_ref() {
        let Some(actual) = artifact_solver_profile_hash.as_ref() else {
            return Err(signing::VerifyError::new(
                signing::VerifyErrorCode::ProofArtifactFailure,
                "signature payload includes solver_profile_hash but proof artifact does not",
            ));
        };
        if expected != actual {
            return Err(signing::VerifyError::new(
                signing::VerifyErrorCode::ProofArtifactFailure,
                format!(
                    "solver_profile_hash mismatch: signature payload expects `{expected}`, proof artifact has `{actual}`"
                ),
            ));
        }
    }
    if let Some(expected) = manifest_solver_profile_hash.as_ref() {
        let Some(actual) = artifact_solver_profile_hash.as_ref() else {
            return Err(signing::VerifyError::new(
                signing::VerifyErrorCode::ProofArtifactFailure,
                "assurance manifest includes solver_profile_hash but proof artifact does not",
            ));
        };
        if expected != actual {
            return Err(signing::VerifyError::new(
                signing::VerifyErrorCode::ProofArtifactFailure,
                format!(
                    "solver_profile_hash mismatch: assurance manifest expects `{expected}`, proof artifact has `{actual}`"
                ),
            ));
        }
    }

    Ok(Some(ProofArtifactConsistencyOutcome {
        proof_artifact_path: path.to_path_buf(),
        proof_artifact_hash: canonical_hash,
        solver_profile_hash: artifact_solver_profile_hash,
    }))
}

