use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use wasmparser::{Parser, Payload};

use crate::commands::helpers::{make_single_json_error, CommandError};
use crate::logging::{Logger, StageTimings};
use crate::proofs::{
    decode_proof_section, default_proof_matrix_path, load_proved_surface_allowlist,
};
use crate::signing;

#[derive(Clone, Copy, Debug, Eq, PartialEq, clap::ValueEnum)]
pub enum VerifyMode {
    Runtime,
    CompileTime,
}

impl VerifyMode {
    fn as_str(self) -> &'static str {
        match self {
            VerifyMode::Runtime => "runtime",
            VerifyMode::CompileTime => "compile-time",
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn run(
    module: PathBuf,
    sig: PathBuf,
    pubkey: PathBuf,
    verify_mode: VerifyMode,
    trust_policy: Option<PathBuf>,
    assurance_manifest: Option<PathBuf>,
    release_policy: Option<PathBuf>,
    require_assurance: Option<String>,
    explain: bool,
    json_errors: bool,
    logger: Logger,
) -> Result<()> {
    let emit_verify_error = |err: signing::VerifyError| -> Result<()> {
        if json_errors {
            let json =
                make_single_json_error(err.code(), "verify", err.to_string(), &module, 0, 0, None);
            Err(CommandError::json(json).into())
        } else {
            Err(anyhow!(err).context("verification failed"))
        }
    };

    let mut timings = StageTimings::new();
    let verified_signature = {
        let _stage = timings.start(logger, "verify_signature");
        match verify_mode {
            VerifyMode::Runtime => {
                if trust_policy.is_some() {
                    Err(signing::VerifyError::new(
                        signing::VerifyErrorCode::TrustAnchorFailure,
                        "`--trust-policy` requires `--verify-mode compile-time`",
                    ))
                } else {
                    signing::verify_signature_details(&module, &sig, &pubkey)
                }
            }
            VerifyMode::CompileTime => match trust_policy.as_ref() {
                Some(policy) => signing::verify_signature_with_trust_policy_details(
                    &module, &sig, &pubkey, policy,
                ),
                None => Err(signing::VerifyError::new(
                    signing::VerifyErrorCode::TrustAnchorFailure,
                    "`--verify-mode compile-time` requires `--trust-policy <FILE>`",
                )),
            },
        }
    };
    match verified_signature {
        Ok(verified_signature) => {
            let release_policy_outcome = match release_policy.as_ref() {
                Some(policy_path) => {
                    let Some(manifest_path) = assurance_manifest.as_ref() else {
                        return emit_verify_error(signing::VerifyError::new(
                            signing::VerifyErrorCode::PolicyFailure,
                            "`--release-policy` requires `--assurance-manifest <FILE>`",
                        ));
                    };
                    let _stage = timings.start(logger, "verify_release_policy");
                    match evaluate_release_policy(
                        manifest_path,
                        policy_path,
                        &verified_signature.payload,
                        &pubkey,
                    ) {
                        Ok(outcome) => Some(outcome),
                        Err(err) => return emit_verify_error(err),
                    }
                }
                None => None,
            };

            let assurance_requirement_outcome = match require_assurance.as_deref() {
                Some(required) => {
                    let _stage = timings.start(logger, "verify_assurance_requirement");
                    match evaluate_required_assurance(
                        required,
                        &verified_signature.payload,
                        assurance_manifest.as_deref(),
                        &pubkey,
                    ) {
                        Ok(outcome) => Some(outcome),
                        Err(err) => return emit_verify_error(err),
                    }
                }
                None => None,
            };

            if explain {
                let _stage = timings.start(logger, "verify_explain");
                if let Err(err) = emit_explain_summary(
                    &module,
                    &verified_signature,
                    verify_mode,
                    trust_policy.as_deref(),
                    release_policy_outcome.as_ref(),
                    assurance_requirement_outcome.as_ref(),
                ) {
                    return emit_verify_error(signing::VerifyError::new(
                        signing::VerifyErrorCode::SignatureFailure,
                        format!("failed to emit verification explanation: {err:#}"),
                    ));
                }
            }
            logger.summary(&timings);
            Ok(())
        }
        Err(err) => emit_verify_error(err),
    }
}

fn emit_explain_summary(
    module: &Path,
    verified_signature: &signing::SignatureFile,
    verify_mode: VerifyMode,
    trust_policy: Option<&Path>,
    release_policy_outcome: Option<&ReleasePolicyOutcome>,
    assurance_requirement_outcome: Option<&AssuranceRequirementOutcome>,
) -> Result<()> {
    #[derive(Default)]
    struct AssumptionAggregate {
        id: String,
        category: String,
        status: String,
        message: String,
        vc_refs: BTreeSet<(String, String)>,
        symbols: BTreeSet<String>,
    }

    let module_bytes = fs::read(module).with_context(|| format!("reading {}", module.display()))?;
    let section_bytes = proof_section_bytes(&module_bytes)?;
    let section = decode_proof_section(&section_bytes).context("decoding proof section")?;
    let sig_payload = verified_signature
        .payload
        .as_object()
        .ok_or_else(|| anyhow!("signature payload must be a JSON object"))?;

    let payload_module_hash = payload_str(sig_payload, "module_hash").unwrap_or("<missing>");
    let payload_proofs_hash = payload_str(sig_payload, "proofs_hash").unwrap_or("<missing>");
    let payload_proof_status = payload_str(sig_payload, "proof_status").unwrap_or("unknown");
    let (tier, label) = payload_assurance(sig_payload);
    let assurance_claim = payload_assurance_claim(sig_payload);

    let mut total_vcs = 0usize;
    let mut checked_core_vcs = 0usize;
    let mut assumed_vcs = 0usize;
    let mut assumption_items = 0usize;
    let mut assumed_boundaries: BTreeMap<String, AssumptionAggregate> = BTreeMap::new();
    let mut dependency_trust: BTreeMap<(String, String), String> = BTreeMap::new();

    for function in &section.functions {
        for vc in &function.vcs {
            total_vcs += 1;
            let items = vc
                .assumptions
                .as_ref()
                .map(|assumptions| assumptions.items.as_slice())
                .unwrap_or(&[]);
            if items.is_empty() {
                checked_core_vcs += 1;
            } else {
                assumed_vcs += 1;
            }
            for assumption in items {
                assumption_items += 1;
                let key = format!(
                    "{}|{}|{}|{}",
                    assumption.id, assumption.category, assumption.status, assumption.message
                );
                let aggregate =
                    assumed_boundaries
                        .entry(key)
                        .or_insert_with(|| AssumptionAggregate {
                            id: assumption.id.clone(),
                            category: assumption.category.clone(),
                            status: assumption.status.clone(),
                            message: assumption.message.clone(),
                            vc_refs: BTreeSet::new(),
                            symbols: BTreeSet::new(),
                        });
                aggregate
                    .vc_refs
                    .insert((function.name.clone(), vc.vc_id.clone()));
                for symbol in &assumption.symbols {
                    if !symbol.trim().is_empty() {
                        aggregate.symbols.insert(symbol.clone());
                    }
                }

                let dependency_kind = match assumption.id.as_str() {
                    "primitive.unproved" => Some("primitive"),
                    "external.dependency" => Some("external"),
                    _ => None,
                };
                if let Some(kind) = dependency_kind {
                    for symbol in &assumption.symbols {
                        if !symbol.trim().is_empty() {
                            dependency_trust
                                .entry((symbol.clone(), kind.to_string()))
                                .or_insert_with(|| assumption.status.clone());
                        }
                    }
                }
            }
        }
    }

    println!("Verification explanation");
    println!("mode: {}", verify_mode.as_str());
    println!("result: verified");
    println!("scope: {}", verified_signature.scope.as_str());
    println!("module_hash: {}", payload_module_hash);
    println!("proofs_hash: {}", payload_proofs_hash);
    println!("proof_status: {}", payload_proof_status);
    println!("assurance: {} ({})", tier, label);
    if let Some(claim) = assurance_claim {
        if claim.non_strict_evidence_only {
            println!(
                "assurance_claim: non-strict evidence only (compiler_mode={})",
                claim.compiler_mode
            );
        } else if claim.release_grade_trust {
            println!(
                "assurance_claim: strict release-grade trust claim (compiler_mode={})",
                claim.compiler_mode
            );
        } else {
            println!(
                "assurance_claim: policy marker present (compiler_mode={})",
                claim.compiler_mode
            );
        }
    }
    println!("functions: {}", section.functions.len());
    println!("vcs_total: {}", total_vcs);
    println!("vcs_checked_core: {}", checked_core_vcs);
    println!("vcs_with_assumptions: {}", assumed_vcs);
    println!("assumption_items: {}", assumption_items);

    if assumed_boundaries.is_empty() {
        println!("assumed_boundaries: none");
    } else {
        println!("assumed_boundaries:");
        for boundary in assumed_boundaries.values() {
            let symbols = if boundary.symbols.is_empty() {
                "<none>".to_string()
            } else {
                boundary
                    .symbols
                    .iter()
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            println!(
                "- {} [{}|{}]: vcs={}, symbols=[{}], why={}",
                boundary.id,
                boundary.category,
                boundary.status,
                boundary.vc_refs.len(),
                symbols,
                boundary.message
            );
        }
    }

    if dependency_trust.is_empty() {
        println!("dependency_trust_labels: none");
    } else {
        println!("dependency_trust_labels:");
        for ((dependency, kind), label) in dependency_trust {
            println!("- {} ({}) => {}", dependency, kind, label);
        }
    }

    if let Some(trust) = payload_trust_anchors(sig_payload) {
        println!(
            "trust_anchors: lean={}, coq={}",
            trust.0.as_str(),
            trust.1.as_str()
        );
    }
    if let Some(path) = trust_policy {
        println!("trust_policy: {}", path.display());
    }
    if let Some(outcome) = release_policy_outcome {
        println!(
            "release_policy: pass (required >= {}, manifest = {})",
            outcome.required_tier, outcome.manifest_tier
        );
        println!("release_policy_file: {}", outcome.policy_path.display());
        println!("assurance_manifest: {}", outcome.manifest_path.display());
    }
    if let Some(outcome) = assurance_requirement_outcome {
        println!(
            "assurance_requirement: pass (required = {}, signature = {})",
            outcome.required_assurance, outcome.signature_proof_status
        );
        if let Some(manifest_status) = &outcome.manifest_proof_status {
            println!("assurance_manifest_proof_status: {}", manifest_status);
        }
        if let Some(path) = &outcome.manifest_path {
            println!("assurance_manifest_checked: {}", path.display());
        }
    }

    Ok(())
}

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

#[derive(Debug, serde::Deserialize)]
struct ReleasePolicyFile {
    schema_version: u32,
    minimum_assurance_tier: String,
}

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
    let signature_assumption_boundaries = parse_signature_assumption_boundaries(sig_payload)
        .ok_or_else(|| {
            signing::VerifyError::new(
                signing::VerifyErrorCode::PolicyFailure,
                "signature payload has invalid assumption_boundaries (expected string array)",
            )
        })?;
    let signature_bundle_symbols =
        parse_signature_bundle_symbols(sig_payload).ok_or_else(|| {
            signing::VerifyError::new(
                signing::VerifyErrorCode::PolicyFailure,
                "signature payload has invalid bundle_symbols (expected string array)",
            )
        })?;

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
        manifest_assumption_boundaries = Some(parse_manifest_assumption_boundaries(payload)?);
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
        if !signature_assumption_boundaries.is_empty() {
            return Err(signing::VerifyError::new(
                signing::VerifyErrorCode::PolicyFailure,
                format!(
                    "required assurance `proved_all` requires zero assumption boundaries in signature payload; found [{}]",
                    signature_assumption_boundaries.join(", ")
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
        if !signature_bundle_symbols.is_empty() {
            let matrix_path = default_proof_matrix_path();
            let proved_allowlist = load_proved_surface_allowlist(&matrix_path).map_err(|err| {
                signing::VerifyError::new(
                    signing::VerifyErrorCode::PolicyFailure,
                    format!("proof matrix allowlist gate failed: {err:#}"),
                )
            })?;
            let disallowed: Vec<String> = signature_bundle_symbols
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
) -> Option<Vec<String>> {
    let Some(value) = payload.get("assumption_boundaries") else {
        return Some(Vec::new());
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
) -> Option<Vec<String>> {
    let Some(value) = payload.get("bundle_symbols") else {
        return Some(Vec::new());
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
