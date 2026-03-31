fn parse_manifest_assumption_boundaries(
    payload: &serde_json::Map<String, serde_json::Value>,
) -> std::result::Result<Vec<String>, signing::VerifyError> {
    let assumptions = payload
        .get("assumptions")
        .and_then(|value| value.as_object())
        .ok_or_else(|| {
            signing::VerifyError::new(
                signing::VerifyErrorCode::PolicyFailure,
                "assurance manifest payload missing `assumptions` object",
            )
        })?;
    let items = assumptions
        .get("items")
        .and_then(|value| value.as_array())
        .ok_or_else(|| {
            signing::VerifyError::new(
                signing::VerifyErrorCode::PolicyFailure,
                "assurance manifest payload assumptions.items must be an array",
            )
        })?;
    let mut out = BTreeSet::new();
    for item in items {
        let id = item
            .as_object()
            .and_then(|obj| obj.get("id"))
            .and_then(|value| value.as_str())
            .map(str::trim)
            .ok_or_else(|| {
                signing::VerifyError::new(
                    signing::VerifyErrorCode::PolicyFailure,
                    "assurance manifest payload assumptions.items[] entries must include string `id`",
                )
            })?;
        if !id.is_empty() {
            out.insert(id.to_string());
        }
    }
    Ok(out.into_iter().collect())
}

struct ModuleProofClaims {
    proof_status: String,
    assumption_boundaries: Vec<String>,
    bundle_symbols: Vec<String>,
}

fn derive_module_proof_claims(
    module_path: &Path,
) -> std::result::Result<ModuleProofClaims, signing::VerifyError> {
    let module_bytes = fs::read(module_path).map_err(|err| {
        signing::VerifyError::new(
            signing::VerifyErrorCode::PolicyFailure,
            format!(
                "reading module for assurance derivation {}: {err}",
                module_path.display()
            ),
        )
    })?;
    let section_bytes = proof_section_bytes(&module_bytes).map_err(|err| {
        signing::VerifyError::new(
            signing::VerifyErrorCode::PolicyFailure,
            format!("loading clearlang.proof section for assurance derivation: {err:#}"),
        )
    })?;
    let section = decode_proof_section(&section_bytes).map_err(|err| {
        signing::VerifyError::new(
            signing::VerifyErrorCode::PolicyFailure,
            format!("decoding clearlang.proof section for assurance derivation: {err:#}"),
        )
    })?;
    let strict_mode = section
        .assurance_claim
        .as_ref()
        .map(|claim| claim.compiler_mode.eq_ignore_ascii_case("strict"))
        .unwrap_or(false);
    let mut has_any_vc = false;
    let mut all_vcs_proved = true;
    let mut boundaries = BTreeSet::new();
    for function in &section.functions {
        for vc in &function.vcs {
            has_any_vc = true;
            if !vc.status.eq_ignore_ascii_case("proved") {
                all_vcs_proved = false;
            }
            if let Some(assumptions) = vc.assumptions.as_ref() {
                for item in &assumptions.items {
                    let id = item.id.trim();
                    if !id.is_empty() {
                        boundaries.insert(id.to_string());
                    }
                }
            }
        }
    }
    let zero_assumptions = boundaries.is_empty();
    let derived_status = if strict_mode && has_any_vc && all_vcs_proved && zero_assumptions {
        PROOF_STATUS_PROVED_ALL
    } else {
        PROOF_STATUS_NOT_PROVED_ALL
    };
    if let Some(section_status_raw) = section.proof_status.as_deref() {
        let section_status = normalize_proof_status(section_status_raw).ok_or_else(|| {
            signing::VerifyError::new(
                signing::VerifyErrorCode::PolicyFailure,
                format!(
                    "module proof section has invalid proof_status `{}` (expected `proved_all|not_proved_all`)",
                    section_status_raw
                ),
            )
        })?;
        if section_status != derived_status {
            return Err(signing::VerifyError::new(
                signing::VerifyErrorCode::PolicyFailure,
                format!(
                    "module proof section proof_status `{}` does not match derived proof_status `{}`",
                    section_status, derived_status
                ),
            ));
        }
    }
    let section_bundle_symbols = section.bundle_symbols.ok_or_else(|| {
        signing::VerifyError::new(
            signing::VerifyErrorCode::PolicyFailure,
            "module proof section missing bundle_symbols; rebuild with current toolchain",
        )
    })?;
    let mut bundle_symbols = BTreeSet::new();
    for symbol in section_bundle_symbols {
        let symbol = symbol.trim();
        if !symbol.is_empty() {
            bundle_symbols.insert(symbol.to_string());
        }
    }
    Ok(ModuleProofClaims {
        proof_status: derived_status.to_string(),
        assumption_boundaries: boundaries.into_iter().collect(),
        bundle_symbols: bundle_symbols.into_iter().collect(),
    })
}

fn normalize_assurance_tier(value: &str) -> Option<String> {
    let trimmed = value.trim().to_ascii_uppercase();
    match trimmed.as_str() {
        "L0" | "L1" | "L2" | "L3" => Some(trimmed),
        _ => None,
    }
}

fn assurance_tier_rank(tier: &str) -> u8 {
    match tier {
        "L0" => 0,
        "L1" => 1,
        "L2" => 2,
        "L3" => 3,
        _ => 0,
    }
}

fn normalize_required_assurance(value: &str) -> Option<String> {
    let trimmed = value.trim().to_ascii_lowercase();
    match trimmed.as_str() {
        "proved_all" => Some(trimmed),
        _ => None,
    }
}

fn normalize_proof_status(value: &str) -> Option<String> {
    let trimmed = value.trim().to_ascii_lowercase();
    match trimmed.as_str() {
        "proved_all" | "not_proved_all" => Some(trimmed),
        _ => None,
    }
}

fn payload_assurance(payload: &serde_json::Map<String, serde_json::Value>) -> (String, String) {
    let Some(assurance) = payload.get("assurance").and_then(|value| value.as_object()) else {
        return ("unknown".to_string(), "unknown".to_string());
    };
    let tier = assurance
        .get("tier")
        .and_then(|value| value.as_str())
        .unwrap_or("unknown")
        .to_string();
    let label = assurance
        .get("label")
        .and_then(|value| value.as_str())
        .unwrap_or("unknown")
        .to_string();
    (tier, label)
}

struct PayloadAssuranceClaim {
    compiler_mode: String,
    non_strict_evidence_only: bool,
    release_grade_trust: bool,
}

fn payload_assurance_claim(
    payload: &serde_json::Map<String, serde_json::Value>,
) -> Option<PayloadAssuranceClaim> {
    let claim = payload.get("assurance_claim")?.as_object()?;
    Some(PayloadAssuranceClaim {
        compiler_mode: claim
            .get("compiler_mode")
            .and_then(|value| value.as_str())
            .unwrap_or("unknown")
            .to_string(),
        non_strict_evidence_only: claim
            .get("non_strict_evidence_only")
            .and_then(|value| value.as_bool())
            .unwrap_or(false),
        release_grade_trust: claim
            .get("release_grade_trust")
            .and_then(|value| value.as_bool())
            .unwrap_or(false),
    })
}

fn payload_trust_anchors(
    payload: &serde_json::Map<String, serde_json::Value>,
) -> Option<(String, String)> {
    let trust = payload.get("trust_anchors")?.as_object()?;
    let lean = trust.get("lean_checker")?.as_str()?.to_string();
    let coq = trust.get("coq_checker")?.as_str()?.to_string();
    Some((lean, coq))
}

fn payload_str<'a>(
    payload: &'a serde_json::Map<String, serde_json::Value>,
    key: &str,
) -> Option<&'a str> {
    payload.get(key)?.as_str()
}

fn proof_section_bytes(module_bytes: &[u8]) -> Result<Vec<u8>> {
    for payload in Parser::new(0).parse_all(module_bytes) {
        let payload = payload.context("parsing module payload")?;
        if let Payload::CustomSection(section) = payload {
            if section.name() == "clearlang.proof" {
                return Ok(section.data().to_vec());
            }
        }
    }
    Err(anyhow!("clearlang.proof section not found"))
}
