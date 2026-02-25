use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use wasmparser::{Parser, Payload};

use crate::commands::helpers::{make_single_json_error, CommandError};
use crate::logging::{Logger, StageTimings};
use crate::proofs::decode_proof_section;
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

pub fn run(
    module: PathBuf,
    sig: PathBuf,
    pubkey: PathBuf,
    verify_mode: VerifyMode,
    trust_policy: Option<PathBuf>,
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
    let result = {
        let _stage = timings.start(logger, "verify_signature");
        match verify_mode {
            VerifyMode::Runtime => {
                if trust_policy.is_some() {
                    Err(signing::VerifyError::new(
                        signing::VerifyErrorCode::TrustAnchorFailure,
                        "`--trust-policy` requires `--verify-mode compile-time`",
                    ))
                } else {
                    signing::verify_signature(&module, &sig, &pubkey)
                }
            }
            VerifyMode::CompileTime => match trust_policy.as_ref() {
                Some(policy) => {
                    signing::verify_signature_with_trust_policy(&module, &sig, &pubkey, policy)
                }
                None => Err(signing::VerifyError::new(
                    signing::VerifyErrorCode::TrustAnchorFailure,
                    "`--verify-mode compile-time` requires `--trust-policy <FILE>`",
                )),
            },
        }
    };
    match result {
        Ok(()) => {
            if explain {
                let _stage = timings.start(logger, "verify_explain");
                if let Err(err) =
                    emit_explain_summary(&module, &sig, verify_mode, trust_policy.as_deref())
                {
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
    sig: &Path,
    verify_mode: VerifyMode,
    trust_policy: Option<&Path>,
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
    let sig_bytes = fs::read(sig).with_context(|| format!("reading {}", sig.display()))?;
    let sig_file: signing::SignatureFile =
        serde_json::from_slice(&sig_bytes).context("parsing signature file")?;
    let sig_payload = sig_file
        .payload
        .as_object()
        .ok_or_else(|| anyhow!("signature payload must be a JSON object"))?;

    let payload_module_hash = payload_str(sig_payload, "module_hash").unwrap_or("<missing>");
    let payload_proofs_hash = payload_str(sig_payload, "proofs_hash").unwrap_or("<missing>");
    let (tier, label) = payload_assurance(sig_payload);

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
    println!("scope: {}", sig_file.scope.as_str());
    println!("module_hash: {}", payload_module_hash);
    println!("proofs_hash: {}", payload_proofs_hash);
    println!("assurance: {} ({})", tier, label);
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

    Ok(())
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
