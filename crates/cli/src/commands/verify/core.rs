use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use wasmparser::{Parser, Payload};

use crate::commands::helpers::{
    canonical_json_bytes, make_single_json_error, sha256_hex, CommandError,
};
use crate::logging::{Logger, StageTimings};
use crate::proofs::{
    decode_proof_section, default_proof_matrix_path, load_proved_surface_allowlist,
    PROOF_STATUS_NOT_PROVED_ALL, PROOF_STATUS_PROVED_ALL,
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
    proof_artifact: Option<PathBuf>,
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
            let proof_artifact_outcome = {
                let _stage = timings.start(logger, "verify_proof_artifact");
                match evaluate_proof_artifact_consistency(
                    &verified_signature.payload,
                    proof_artifact.as_deref(),
                    assurance_manifest.as_deref(),
                    &pubkey,
                ) {
                    Ok(outcome) => outcome,
                    Err(err) => return emit_verify_error(err),
                }
            };
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
                        &module,
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
                    proof_artifact_outcome.as_ref(),
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
    proof_artifact_outcome: Option<&ProofArtifactConsistencyOutcome>,
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
    if let Some(outcome) = proof_artifact_outcome {
        println!(
            "proof_artifact_checked: {}",
            outcome.proof_artifact_path.display()
        );
        println!("proof_artifact_hash: {}", outcome.proof_artifact_hash);
        if let Some(hash) = &outcome.solver_profile_hash {
            println!("solver_profile_hash: {}", hash);
        }
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
