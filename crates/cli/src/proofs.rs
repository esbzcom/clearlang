use std::collections::{BTreeMap, BTreeSet, HashMap};

use anyhow::{anyhow, Result};
use clg_ast::Program;
use clg_typer::RefinementAttachmentDetail;
use clg_typer::VerificationCondition;
use serde::{Deserialize, Serialize};
use wasmparser::{Parser, Payload};

use crate::signing::SignScope;

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
}

pub struct ProofPackage {
    toolchain: String,
    functions: Vec<ProofFunction>,
    proofs_hash: [u8; 32],
}

impl ProofPackage {
    pub fn from_program(
        program: &Program,
        vcs: &[VerificationCondition],
        mangled_name_origins: &HashMap<String, String>,
        toolchain: String,
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
