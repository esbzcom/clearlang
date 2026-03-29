use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};
use clg_ast::Program;
use clg_typer::{AssumptionBoundary, RefinementAttachmentDetail, VerificationCondition};
use serde::{Deserialize, Serialize};
use wasmparser::{Parser, Payload};

use crate::signing::SignScope;

const ASSUMPTION_CRYPTO_ID: &str = "crypto.uninterpreted";
const ASSUMPTION_PRIMITIVE_ID: &str = "primitive.unproved";
const ASSUMPTION_EXTERNAL_ID: &str = "external.dependency";
const ASSURANCE_TIER_L0: &str = "L0";
pub const PROOF_STATUS_PROVED_ALL: &str = "proved_all";
pub const PROOF_STATUS_NOT_PROVED_ALL: &str = "not_proved_all";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct ProofSpan {
    pub start: u32,
    pub end: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProofExpr {
    pub ast: String,
    pub smt2: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub span: Option<ProofSpan>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProofRefinementExpr {
    pub ast: String,
    pub smt2: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProofRefinementFlowDetail {
    pub flow_kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub callee: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arg_index: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variant: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arm: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", content = "detail")]
pub enum ProofRefinementAttachment {
    Param { param: String },
    Return { result: String },
    Flow(ProofRefinementFlowDetail),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProofRefinementPremise {
    pub id: String,
    pub alias: String,
    pub binder: String,
    pub substitution: ProofRefinementExpr,
    pub predicate: ProofRefinementExpr,
    pub attachment: ProofRefinementAttachment,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProofRefinements {
    pub premises: Vec<ProofRefinementPremise>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProofAssumption {
    pub id: String,
    pub category: String,
    pub status: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub symbols: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub intrinsic_levels: Vec<ProofIntrinsicLevel>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProofIntrinsicLevel {
    pub intrinsic: String,
    pub tier: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProofAssumptions {
    pub items: Vec<ProofAssumption>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProofAssuranceLevels {
    #[serde(rename = "L0")]
    pub l0: String,
    #[serde(rename = "L1")]
    pub l1: String,
    #[serde(rename = "L2")]
    pub l2: String,
    #[serde(rename = "L3")]
    pub l3: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProofAssurance {
    pub tier: String,
    pub label: String,
    pub levels: ProofAssuranceLevels,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProofAssuranceClaim {
    pub compiler_mode: String,
    pub non_strict_evidence_only: bool,
    pub release_grade_trust: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProofProof {
    pub format: String,
    #[serde(with = "serde_bytes")]
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProofVc {
    pub vc_id: String,
    pub pre: ProofExpr,
    pub post: ProofExpr,
    pub vc_smt2: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proof: Option<ProofProof>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refinements: Option<ProofRefinements>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assumptions: Option<ProofAssumptions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assurance: Option<ProofAssurance>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProofFunction {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub canonical_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requires: Option<ProofExpr>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ensures: Vec<ProofExpr>,
    pub vcs: Vec<ProofVc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProofSection {
    pub version: u32,
    pub generated_by: String,
    #[serde(with = "serde_bytes")]
    pub module_hash: Vec<u8>,
    #[serde(with = "serde_bytes")]
    pub proofs_hash: Vec<u8>,
    pub functions: Vec<ProofFunction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assurance: Option<ProofAssurance>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assurance_claim: Option<ProofAssuranceClaim>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proof_status: Option<String>,
}

pub struct ProofPackage {
    toolchain: String,
    functions: Vec<ProofFunction>,
    proofs_hash: [u8; 32],
    assurance: ProofAssurance,
    assurance_claim: ProofAssuranceClaim,
    proof_status: String,
    assumption_boundaries: Vec<String>,
    bundle_symbols: Vec<String>,
}

impl ProofPackage {
    pub fn from_program(
        program: &Program,
        vcs: &[VerificationCondition],
        mangled_name_origins: &HashMap<String, String>,
        toolchain: String,
        compiler_mode: &str,
    ) -> ProofPackage {
        let mut grouped: BTreeMap<String, Vec<ProofVc>> = BTreeMap::new();
        for vc in vcs {
            let refinements = if vc.refinements.is_empty() {
                None
            } else {
                let premises = vc
                    .refinements
                    .iter()
                    .enumerate()
                    .map(|(idx, premise)| ProofRefinementPremise {
                        id: format!("ref:{}", idx),
                        alias: premise.alias.clone(),
                        binder: premise.binder.clone(),
                        substitution: ProofRefinementExpr {
                            ast: premise.substitution.ast.clone(),
                            smt2: premise.substitution.smt2.clone(),
                        },
                        predicate: ProofRefinementExpr {
                            ast: premise.predicate.ast.clone(),
                            smt2: premise.predicate.smt2.clone(),
                        },
                        attachment: match &premise.attachment.detail {
                            RefinementAttachmentDetail::Param { param } => {
                                ProofRefinementAttachment::Param {
                                    param: param.clone(),
                                }
                            }
                            RefinementAttachmentDetail::Return { result } => {
                                ProofRefinementAttachment::Return {
                                    result: result.clone(),
                                }
                            }
                            RefinementAttachmentDetail::Flow(detail) => {
                                ProofRefinementAttachment::Flow(ProofRefinementFlowDetail {
                                    flow_kind: detail.flow_kind.as_str().to_string(),
                                    name: detail.name.clone(),
                                    callee: detail.callee.clone(),
                                    arg_index: detail.arg_index.map(|v| v as u32),
                                    variant: detail.variant.clone(),
                                    arm: detail.arm.map(|v| v as u32),
                                })
                            }
                        },
                    })
                    .collect();
                Some(ProofRefinements { premises })
            };
            let assumptions = if vc.assumptions.is_empty() {
                None
            } else {
                Some(ProofAssumptions {
                    items: vc
                        .assumptions
                        .iter()
                        .map(proof_assumption_from_boundary)
                        .collect(),
                })
            };
            grouped
                .entry(vc.function.clone())
                .or_default()
                .push(ProofVc {
                    vc_id: vc.vc_id.clone(),
                    pre: ProofExpr::from_contract_expr(&vc.pre),
                    post: ProofExpr::from_contract_expr(&vc.post),
                    vc_smt2: vc.vc_smt2.clone(),
                    status: vc.status.to_string(),
                    proof: None,
                    refinements,
                    assumptions,
                    assurance: Some(assurance_for_assumptions(&vc.assumptions)),
                });
        }

        let mut functions = Vec::new();
        for func in &program.funcs {
            let mut fn_vcs = grouped.remove(&func.name).unwrap_or_default();
            fn_vcs.sort_by(|a, b| a.vc_id.cmp(&b.vc_id));

            let prefers = fn_vcs
                .iter()
                .find(|vc| vc.vc_id.starts_with("vc:"))
                .or_else(|| fn_vcs.first())
                .and_then(|vc| {
                    if vc.pre.ast == "true" && vc.pre.span.is_none() {
                        None
                    } else {
                        Some(vc.pre.clone())
                    }
                });

            let mut ensures = Vec::new();
            let mut seen_posts = BTreeSet::new();
            for vc in &fn_vcs {
                if !vc.vc_id.starts_with("vc:") {
                    continue;
                }
                let key = (
                    vc.post.ast.clone(),
                    vc.post.smt2.clone(),
                    vc.post.span.clone(),
                );
                if seen_posts.insert(key) {
                    ensures.push(vc.post.clone());
                }
            }

            functions.push(ProofFunction {
                name: func.name.clone(),
                canonical_name: canonical_name_for(&func.name, mangled_name_origins),
                requires: prefers,
                ensures,
                vcs: fn_vcs,
            });
        }
        functions.sort_by(|a, b| a.name.cmp(&b.name));

        let mut proof_entries = Vec::new();
        for func in &functions {
            for vc in &func.vcs {
                proof_entries.push((func.name.clone(), vc.clone()));
            }
        }
        let proofs_hash = compute_proofs_hash_from_vcs(&proof_entries);
        ProofPackage {
            toolchain,
            functions,
            proofs_hash,
            assurance: assurance_for_vcs(vcs),
            assurance_claim: assurance_claim_for_compiler_mode(compiler_mode),
            proof_status: proof_status_for_vcs(vcs, compiler_mode).to_string(),
            assumption_boundaries: assumption_boundary_ids_for_vcs(vcs),
            bundle_symbols: bundle_symbols_for_program(program),
        }
    }

    pub fn proofs_hash_hex(&self) -> String {
        hex::encode(self.proofs_hash)
    }

    pub fn encode_section(&self, module_hash: &[u8; 32]) -> Result<Vec<u8>> {
        let section = ProofSection {
            version: 2,
            generated_by: self.toolchain.clone(),
            module_hash: module_hash.to_vec(),
            proofs_hash: self.proofs_hash.to_vec(),
            functions: self.functions.clone(),
            assurance: Some(self.assurance.clone()),
            assurance_claim: Some(self.assurance_claim.clone()),
            proof_status: Some(self.proof_status.clone()),
        };
        to_cbor_bytes(&section)
    }

    pub fn build_signing_payload(
        &self,
        module_hash_hex: &str,
        scope: SignScope,
        timestamp: &str,
    ) -> serde_json::Value {
        serde_json::json!({
            "module_hash": module_hash_hex,
            "proofs_hash": self.proofs_hash_hex(),
            "toolchain": self.toolchain,
            "timestamp": timestamp,
            "scope": scope.as_str(),
            "assurance": self.assurance.clone(),
            "assurance_claim": self.assurance_claim.clone(),
            "proof_status": self.proof_status,
            "assumption_boundaries": self.assumption_boundaries.clone(),
            "bundle_symbols": self.bundle_symbols.clone(),
        })
    }
}

impl ProofExpr {
    fn from_contract_expr(expr: &clg_typer::ContractExpr) -> ProofExpr {
        ProofExpr {
            ast: expr.ast.clone(),
            smt2: expr.smt2.clone(),
            span: expr.span.map(|s| ProofSpan {
                start: s.start as u32,
                end: s.end as u32,
            }),
        }
    }
}

fn proof_assumption_from_boundary(boundary: &AssumptionBoundary) -> ProofAssumption {
    ProofAssumption {
        id: boundary.id.to_string(),
        category: boundary.category.as_str().to_string(),
        status: boundary.status.to_string(),
        message: boundary.message.to_string(),
        symbols: boundary.symbols.clone(),
        intrinsic_levels: intrinsic_levels_for_boundary(boundary),
    }
}

fn intrinsic_levels_for_boundary(boundary: &AssumptionBoundary) -> Vec<ProofIntrinsicLevel> {
    if boundary.id != ASSUMPTION_CRYPTO_ID {
        return Vec::new();
    }
    boundary
        .symbols
        .iter()
        .map(|symbol| ProofIntrinsicLevel {
            intrinsic: symbol.clone(),
            tier: ASSURANCE_TIER_L0.to_string(),
            label: assurance_label_for_tier(ASSURANCE_TIER_L0).to_string(),
        })
        .collect()
}

pub fn assurance_for_assumptions(assumptions: &[AssumptionBoundary]) -> ProofAssurance {
    let tier = if assumptions.is_empty() { "L1" } else { "L0" };
    proof_assurance_for_tier(tier)
}

pub fn assurance_for_vcs(vcs: &[VerificationCondition]) -> ProofAssurance {
    if vcs.iter().any(|vc| !vc.assumptions.is_empty()) {
        return proof_assurance_for_tier("L0");
    }
    proof_assurance_for_tier("L1")
}

pub fn assurance_claim_for_compiler_mode(compiler_mode: &str) -> ProofAssuranceClaim {
    let normalized = match compiler_mode.trim().to_ascii_lowercase().as_str() {
        "strict" => "strict",
        "standard" => "standard",
        "permissive" => "permissive",
        _ => "unknown",
    }
    .to_string();
    let strict = normalized == "strict";
    ProofAssuranceClaim {
        compiler_mode: normalized,
        non_strict_evidence_only: !strict,
        release_grade_trust: strict,
    }
}

pub fn proof_status_for_vcs(vcs: &[VerificationCondition], compiler_mode: &str) -> &'static str {
    let strict_mode = compiler_mode.trim().eq_ignore_ascii_case("strict");
    let all_vcs_proved = !vcs.is_empty() && vcs.iter().all(|vc| vc.status == "proved");
    let zero_assumptions = vcs.iter().all(|vc| vc.assumptions.is_empty());
    if strict_mode && all_vcs_proved && zero_assumptions {
        PROOF_STATUS_PROVED_ALL
    } else {
        PROOF_STATUS_NOT_PROVED_ALL
    }
}

fn assumption_boundary_ids_for_vcs(vcs: &[VerificationCondition]) -> Vec<String> {
    let mut ids = BTreeSet::new();
    for vc in vcs {
        for assumption in &vc.assumptions {
            ids.insert(assumption.id.to_string());
        }
    }
    ids.into_iter().collect()
}

pub fn bundle_symbols_for_program(program: &Program) -> Vec<String> {
    let mut symbols = BTreeSet::new();
    for func in &program.funcs {
        collect_bundle_symbols_from_expr(&func.body, &mut symbols);
    }
    symbols.into_iter().collect()
}

pub fn load_proved_surface_allowlist(matrix_path: &Path) -> Result<BTreeSet<String>> {
    let bytes = fs::read(matrix_path)
        .map_err(|err| anyhow!("reading proof matrix {}: {err}", matrix_path.display()))?;
    let value: serde_json::Value = serde_json::from_slice(&bytes)
        .map_err(|err| anyhow!("parsing proof matrix {}: {err}", matrix_path.display()))?;
    let entries = value
        .get("entries")
        .and_then(|v| v.as_array())
        .ok_or_else(|| anyhow!("proof matrix {} missing entries[]", matrix_path.display()))?;
    let mut surfaces = BTreeSet::new();
    for entry in entries {
        let Some(obj) = entry.as_object() else {
            continue;
        };
        if obj.get("status").and_then(|v| v.as_str()) != Some("proved") {
            continue;
        }
        if let Some(surface) = obj.get("surface").and_then(|v| v.as_str()) {
            let surface = surface.trim();
            if !surface.is_empty() {
                surfaces.insert(surface.to_string());
            }
        }
    }
    Ok(surfaces)
}

pub fn proof_matrix_path_from_env() -> PathBuf {
    std::env::var("CLG_PROOF_MATRIX_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("docs/proofs/proof-coverage-matrix.json"))
}

pub fn default_proof_matrix_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("docs")
        .join("proofs")
        .join("proof-coverage-matrix.json")
}

fn collect_bundle_symbols_from_block(block: &clg_ast::Block, out: &mut BTreeSet<String>) {
    for stmt in &block.statements {
        match stmt {
            clg_ast::Stmt::Let { expr, .. } => collect_bundle_symbols_from_expr(expr, out),
            clg_ast::Stmt::Expr { expr, .. } => collect_bundle_symbols_from_expr(expr, out),
            clg_ast::Stmt::While {
                cond,
                invariant,
                variant,
                body,
                ..
            } => {
                collect_bundle_symbols_from_expr(cond, out);
                collect_bundle_symbols_from_expr(invariant, out);
                if let Some(variant_expr) = variant {
                    collect_bundle_symbols_from_expr(variant_expr, out);
                }
                collect_bundle_symbols_from_block(body, out);
            }
        }
    }
    if let Some(tail) = &block.tail {
        collect_bundle_symbols_from_expr(tail, out);
    }
}

fn collect_bundle_symbols_from_expr(expr: &clg_ast::Expr, out: &mut BTreeSet<String>) {
    use clg_ast::Expr;
    match expr {
        Expr::Call { callee, args, .. } => {
            if callee.starts_with("std::") {
                out.insert(callee.clone());
            }
            for arg in args {
                collect_bundle_symbols_from_expr(arg, out);
            }
        }
        Expr::Bin { lhs, rhs, .. } => {
            collect_bundle_symbols_from_expr(lhs, out);
            collect_bundle_symbols_from_expr(rhs, out);
        }
        Expr::Unary { expr, .. } | Expr::Return { expr, .. } | Expr::Try { expr, .. } => {
            collect_bundle_symbols_from_expr(expr, out);
        }
        Expr::Index { base, index, .. } => {
            collect_bundle_symbols_from_expr(base, out);
            collect_bundle_symbols_from_expr(index, out);
        }
        Expr::Match {
            scrutinee, arms, ..
        } => {
            collect_bundle_symbols_from_expr(scrutinee, out);
            for arm in arms {
                collect_bundle_symbols_from_expr(&arm.expr, out);
            }
        }
        Expr::If {
            cond,
            then_br,
            else_br,
            ..
        } => {
            collect_bundle_symbols_from_expr(cond, out);
            collect_bundle_symbols_from_expr(then_br, out);
            collect_bundle_symbols_from_expr(else_br, out);
        }
        Expr::Block { block } => collect_bundle_symbols_from_block(block, out),
        Expr::ArrayLit { elems, .. } | Expr::TupleLit { elems, .. } => {
            for elem in elems {
                collect_bundle_symbols_from_expr(elem, out);
            }
        }
        Expr::StructLit { fields, .. } => {
            for field in fields {
                collect_bundle_symbols_from_expr(&field.expr, out);
            }
        }
        Expr::FieldAccess { base, .. } => collect_bundle_symbols_from_expr(base, out),
        Expr::Lambda { body, .. } => collect_bundle_symbols_from_expr(body, out),
        Expr::Int(..) | Expr::Bool(..) | Expr::String(..) | Expr::Var(..) => {}
    }
}

pub fn build_assurance_manifest_payload(
    vcs: &[VerificationCondition],
    toolchain: &str,
    compiler_mode: &str,
    proof_strict: bool,
    module_hash_hex: &str,
    proofs_hash_hex: &str,
    generated_at: &str,
) -> serde_json::Value {
    use serde_json::json;

    #[derive(Default)]
    struct AssumptionAggregate {
        symbols: BTreeSet<String>,
        vc_refs: BTreeSet<(String, String)>,
        intrinsic_levels: BTreeSet<String>,
    }

    let mut ordered_vcs: Vec<&VerificationCondition> = vcs.iter().collect();
    ordered_vcs.sort_by(|left, right| {
        left.function
            .cmp(&right.function)
            .then_with(|| left.vc_id.cmp(&right.vc_id))
    });

    let mut assumptions: BTreeMap<(String, String, String, String), AssumptionAggregate> =
        BTreeMap::new();
    let mut dependency_trust: BTreeMap<(String, String), String> = BTreeMap::new();
    let mut assumption_count = 0usize;
    let assurance_claim = assurance_claim_for_compiler_mode(compiler_mode);
    let proof_status = proof_status_for_vcs(vcs, compiler_mode);

    for vc in ordered_vcs {
        for boundary in &vc.assumptions {
            assumption_count += 1;
            let key = (
                boundary.id.to_string(),
                boundary.category.as_str().to_string(),
                boundary.status.to_string(),
                boundary.message.to_string(),
            );
            let aggregate = assumptions.entry(key).or_default();
            aggregate
                .vc_refs
                .insert((vc.function.clone(), vc.vc_id.clone()));
            for symbol in &boundary.symbols {
                if !symbol.trim().is_empty() {
                    aggregate.symbols.insert(symbol.clone());
                }
            }
            for level in intrinsic_levels_for_boundary(boundary) {
                aggregate.intrinsic_levels.insert(level.intrinsic);
            }

            let dependency_kind = match boundary.id {
                ASSUMPTION_PRIMITIVE_ID => Some("primitive"),
                ASSUMPTION_EXTERNAL_ID => Some("external"),
                _ => None,
            };
            if let Some(kind) = dependency_kind {
                for symbol in &boundary.symbols {
                    if symbol.trim().is_empty() {
                        continue;
                    }
                    dependency_trust
                        .entry((symbol.clone(), kind.to_string()))
                        .or_insert_with(|| boundary.status.to_string());
                }
            }
        }
    }

    let assumption_items: Vec<serde_json::Value> = assumptions
        .into_iter()
        .map(|((id, category, status, message), aggregate)| {
            let symbols: Vec<String> = aggregate.symbols.into_iter().collect();
            let vc_refs: Vec<serde_json::Value> = aggregate
                .vc_refs
                .into_iter()
                .map(|(function, vc_id)| json!({ "function": function, "vc_id": vc_id }))
                .collect();
            let intrinsic_levels: Vec<serde_json::Value> = aggregate
                .intrinsic_levels
                .into_iter()
                .map(|intrinsic| {
                    json!({
                        "intrinsic": intrinsic,
                        "tier": ASSURANCE_TIER_L0,
                        "label": assurance_label_for_tier(ASSURANCE_TIER_L0),
                    })
                })
                .collect();
            let mut item = serde_json::Map::new();
            item.insert("id".to_string(), json!(id));
            item.insert("category".to_string(), json!(category));
            item.insert("status".to_string(), json!(status));
            item.insert("message".to_string(), json!(message));
            item.insert("symbols".to_string(), json!(symbols));
            item.insert("vc_refs".to_string(), json!(vc_refs));
            if !intrinsic_levels.is_empty() {
                item.insert("intrinsic_levels".to_string(), json!(intrinsic_levels));
            }
            serde_json::Value::Object(item)
        })
        .collect();

    let dependency_trust_labels: Vec<serde_json::Value> = dependency_trust
        .into_iter()
        .map(|((dependency, kind), label)| {
            json!({
                "dependency": dependency,
                "kind": kind,
                "label": label,
            })
        })
        .collect();

    json!({
        "format": "clg.assurance_manifest.v1",
        "generated_at": generated_at,
        "toolchain": {
            "name": toolchain,
            "fingerprint_sha256": sha256_hex(toolchain.as_bytes()),
        },
        "build": {
            "compiler_mode": compiler_mode,
            "proof_strict": proof_strict,
        },
        "assurance_claim": assurance_claim,
        "proof_status": proof_status,
        "artifacts": {
            "module_hash": module_hash_hex,
            "proofs_hash": proofs_hash_hex,
        },
        "assurance": assurance_for_vcs(vcs),
        "assumptions": {
            "total": assumption_count,
            "items": assumption_items,
        },
        "dependency_trust_labels": dependency_trust_labels,
    })
}

fn proof_assurance_for_tier(tier: &str) -> ProofAssurance {
    ProofAssurance {
        tier: tier.to_string(),
        label: assurance_label_for_tier(tier).to_string(),
        levels: ProofAssuranceLevels {
            l0: assurance_label_for_tier("L0").to_string(),
            l1: assurance_label_for_tier("L1").to_string(),
            l2: assurance_label_for_tier("L2").to_string(),
            l3: assurance_label_for_tier("L3").to_string(),
        },
    }
}

fn assurance_label_for_tier(tier: &str) -> &'static str {
    match tier {
        "L0" => "assumed",
        "L1" => "checked core",
        "L2" => "verified module",
        "L3" => "verified package profile",
        _ => "unknown",
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    use std::fmt::Write;

    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    let mut output = String::with_capacity(digest.len() * 2);
    for b in digest {
        write!(&mut output, "{:02x}", b).expect("write hex");
    }
    output
}

fn canonical_name_for(
    emitted_name: &str,
    mangled_name_origins: &HashMap<String, String>,
) -> Option<String> {
    let canonical = mangled_name_origins.get(emitted_name)?;
    if canonical == emitted_name {
        return None;
    }
    Some(canonical.clone())
}

pub fn compute_proofs_hash_from_vcs(vcs: &[(String, ProofVc)]) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    for (name, vc) in vcs {
        let bytes = to_cbor_bytes(&(name, vc)).expect("serialize vc");
        hasher.update(bytes);
    }
    hasher.finalize().into()
}

pub fn to_cbor_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    let mut buf = Vec::new();
    let mut serializer = serde_cbor::ser::Serializer::new(&mut buf);
    value.serialize(&mut serializer)?;
    Ok(buf)
}

pub fn decode_proof_section(bytes: &[u8]) -> Result<ProofSection> {
    let section: ProofSection = serde_cbor::from_slice(bytes)?;
    Ok(section)
}

pub fn hash_module(bytes: &[u8]) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher.finalize().into()
}

pub fn proofs_hash_from_section(section: &ProofSection) -> Result<[u8; 32]> {
    let mut entries = Vec::new();
    for func in &section.functions {
        for vc in &func.vcs {
            entries.push((func.name.clone(), vc.clone()));
        }
    }
    Ok(compute_proofs_hash_from_vcs(&entries))
}

fn overwrite_module_hash(data: &mut [u8], new_hash: &[u8; 32]) -> Result<()> {
    const KEY_BYTES: &[u8] = b"module_hash";
    const HEADER: u8 = 0x60 | (KEY_BYTES.len() as u8);
    const BYTE_STRING_MARKER: u8 = 0x58;
    const BYTE_STRING_LEN: u8 = 0x20;

    let mut i = 0;
    while i + KEY_BYTES.len() + 2 <= data.len() {
        if data[i] == HEADER && &data[i + 1..i + 1 + KEY_BYTES.len()] == KEY_BYTES {
            let value_start = i + 1 + KEY_BYTES.len();
            if value_start + 2 + new_hash.len() <= data.len()
                && data[value_start] == BYTE_STRING_MARKER
                && data[value_start + 1] == BYTE_STRING_LEN
            {
                let bytes_start = value_start + 2;
                data[bytes_start..bytes_start + new_hash.len()].copy_from_slice(new_hash);
                return Ok(());
            }
        }
        i += 1;
    }
    Err(anyhow!("module hash field not found in proof section"))
}

pub fn module_bytes_with_zeroed_hash(bytes: &[u8]) -> Result<Vec<u8>> {
    const ZERO: [u8; 32] = [0u8; 32];
    rewrite_module_hash(bytes, &ZERO)
}

fn rewrite_module_hash(bytes: &[u8], new_hash: &[u8; 32]) -> Result<Vec<u8>> {
    let mut rewritten = bytes.to_vec();
    for payload in Parser::new(0).parse_all(bytes) {
        if let Payload::CustomSection(section) = payload? {
            if section.name() == "clearlang.proof" {
                let data_len = section.data().len();
                let range = section.range();
                let data_start = range.end - data_len;
                let data_end = range.end;
                overwrite_module_hash(&mut rewritten[data_start..data_end], new_hash)?;
                return Ok(rewritten);
            }
        }
    }
    Err(anyhow!("clearlang.proof section not found"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use clg_ast::Span;
    use clg_typer::{AssumptionBoundary, AssumptionCategory, ContractExpr, VerificationCondition};

    fn sample_vc(status: &'static str) -> VerificationCondition {
        VerificationCondition {
            function: "main".to_string(),
            vc_id: "vc:0".to_string(),
            pre: ContractExpr {
                ast: "true".to_string(),
                smt2: "true".to_string(),
                span: Some(Span { start: 0, end: 0 }),
            },
            post: ContractExpr {
                ast: "true".to_string(),
                smt2: "true".to_string(),
                span: Some(Span { start: 0, end: 0 }),
            },
            vc_smt2: "(=> true true)".to_string(),
            status,
            refinements: Vec::new(),
            assumptions: Vec::new(),
        }
    }

    #[test]
    fn proof_status_is_proved_all_only_for_strict_all_proved_without_assumptions() {
        let vcs = vec![sample_vc("proved"), sample_vc("proved")];
        assert_eq!(
            proof_status_for_vcs(&vcs, "strict"),
            PROOF_STATUS_PROVED_ALL
        );
    }

    #[test]
    fn proof_status_is_not_proved_all_when_any_vc_not_proved() {
        let vcs = vec![sample_vc("proved"), sample_vc("generated")];
        assert_eq!(
            proof_status_for_vcs(&vcs, "strict"),
            PROOF_STATUS_NOT_PROVED_ALL
        );
    }

    #[test]
    fn proof_status_is_not_proved_all_when_assumptions_exist_or_not_strict() {
        let mut vc = sample_vc("proved");
        vc.assumptions.push(AssumptionBoundary {
            id: "crypto.uninterpreted",
            category: AssumptionCategory::Crypto,
            status: "assumed",
            message: "crypto modeled as uninterpreted",
            symbols: vec!["std::crypto::hash".to_string()],
        });
        assert_eq!(
            proof_status_for_vcs(&[vc], "strict"),
            PROOF_STATUS_NOT_PROVED_ALL
        );

        let vcs = vec![sample_vc("proved")];
        assert_eq!(
            proof_status_for_vcs(&vcs, "standard"),
            PROOF_STATUS_NOT_PROVED_ALL
        );
    }
}
