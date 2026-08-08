use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use clg_typer::{AssumptionBoundary, RefinementAttachmentDetail, VerificationCondition};

use crate::proofs::{
    assurance_claim_for_compiler_mode, assurance_for_assumptions, proof_status_for_vcs,
};

const ASSUMPTION_CRYPTO_ID: &str = "crypto.uninterpreted";

pub(super) fn write_vcs_json(
    vcs: &[VerificationCondition],
    mangled_name_origins: &HashMap<String, String>,
    path: &Path,
    src: &Path,
    compiler_mode: &str,
) -> Result<()> {
    use serde_json::json;

    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
        }
    }

    let file_str = src.to_string_lossy();
    let assurance_claim = assurance_claim_for_compiler_mode(compiler_mode);
    let proof_status = proof_status_for_vcs(vcs, compiler_mode);
    let mut items = Vec::with_capacity(vcs.len());
    for vc in vcs {
        let positions = match (vc.pre.span, vc.post.span) {
            (None, None) => None,
            (pre, post) => Some(json!({
                "file": file_str,
                "pre_start": pre.map(|s| s.start).unwrap_or(0),
                "pre_end": pre.map(|s| s.end).unwrap_or(0),
                "post_start": post.map(|s| s.start).unwrap_or(0),
                "post_end": post.map(|s| s.end).unwrap_or(0),
            })),
        };
        let mut obj = serde_json::Map::new();
        obj.insert("version".to_string(), json!(2));
        obj.insert("function".to_string(), json!(vc.function));
        if let Some(canonical) = canonical_name_for(&vc.function, mangled_name_origins) {
            obj.insert("canonical_function".to_string(), json!(canonical));
        }
        obj.insert("vc_id".to_string(), json!(vc.vc_id));
        obj.insert(
            "pre".to_string(),
            json!({ "ast": vc.pre.ast, "smt2": vc.pre.smt2 }),
        );
        obj.insert(
            "post".to_string(),
            json!({ "ast": vc.post.ast, "smt2": vc.post.smt2 }),
        );
        obj.insert("vc".to_string(), json!({ "smt2": vc.vc_smt2 }));
        obj.insert("status".to_string(), json!(vc.status));
        obj.insert("proof_status".to_string(), json!(proof_status));
        obj.insert(
            "assurance".to_string(),
            serde_json::to_value(assurance_for_assumptions(&vc.assumptions))?,
        );
        obj.insert(
            "assurance_claim".to_string(),
            serde_json::to_value(&assurance_claim)?,
        );
        if !vc.assumptions.is_empty() {
            obj.insert("assumptions".to_string(), assumptions_json(&vc.assumptions));
        }
        let mut diagnostics = serde_json::Map::new();
        if let Some(repair_hints) = vc_repair_hints_json(vc) {
            diagnostics.insert("repair_hints".to_string(), repair_hints);
        }
        if let Some(failure_slice) = vc_failure_slice_json(vc, file_str.as_ref()) {
            diagnostics.insert("failure_slice".to_string(), failure_slice);
        }
        if let Some(counterexample) = vc_counterexample_json(vc, file_str.as_ref()) {
            diagnostics.insert("counterexample".to_string(), counterexample);
        }
        diagnostics.insert("proof_context".to_string(), vc_proof_context_json(vc));
        if !diagnostics.is_empty() {
            obj.insert(
                "diagnostics".to_string(),
                serde_json::Value::Object(diagnostics),
            );
        }
        if let Some(pos) = positions {
            obj.insert("positions".to_string(), pos);
        }
        if !vc.refinements.is_empty() {
            let mut premises = Vec::with_capacity(vc.refinements.len());
            for (idx, premise) in vc.refinements.iter().enumerate() {
                let mut prem = serde_json::Map::new();
                prem.insert("id".to_string(), json!(format!("ref:{}", idx)));
                prem.insert("alias".to_string(), json!(premise.alias.as_str()));
                prem.insert("binder".to_string(), json!(premise.binder.as_str()));
                prem.insert(
                    "substitution".to_string(),
                    json!({
                        "ast": premise.substitution.ast.as_str(),
                        "smt2": premise.substitution.smt2.as_str(),
                    }),
                );
                prem.insert(
                    "predicate".to_string(),
                    json!({
                        "ast": premise.predicate.ast.as_str(),
                        "smt2": premise.predicate.smt2.as_str(),
                    }),
                );
                prem.insert(
                    "attachment".to_string(),
                    refinement_attachment_json(&premise.attachment),
                );
                premises.push(serde_json::Value::Object(prem));
            }
            obj.insert("refinements".to_string(), json!({ "premises": premises }));
        }
        items.push(serde_json::Value::Object(obj));
    }
    let data = serde_json::to_vec_pretty(&serde_json::Value::Array(items))?;
    fs::write(path, data).with_context(|| format!("writing {}", path.display()))?;
    Ok(())
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

fn refinement_attachment_json(att: &clg_typer::RefinementAttachment) -> serde_json::Value {
    use serde_json::json;
    match &att.detail {
        RefinementAttachmentDetail::Param { param } => json!({
            "kind": "param",
            "detail": { "param": param },
        }),
        RefinementAttachmentDetail::Return { result } => json!({
            "kind": "return",
            "detail": { "result": result },
        }),
        RefinementAttachmentDetail::Flow(flow) => {
            let mut detail = serde_json::Map::new();
            detail.insert("flow_kind".to_string(), json!(flow.flow_kind.as_str()));
            if let Some(name) = &flow.name {
                detail.insert("name".to_string(), json!(name));
            }
            if let Some(callee) = &flow.callee {
                detail.insert("callee".to_string(), json!(callee));
            }
            if let Some(arg_index) = flow.arg_index {
                detail.insert("arg_index".to_string(), json!(arg_index));
            }
            if let Some(variant) = &flow.variant {
                detail.insert("variant".to_string(), json!(variant));
            }
            if let Some(arm) = flow.arm {
                detail.insert("arm".to_string(), json!(arm));
            }
            json!({
                "kind": "flow",
                "detail": serde_json::Value::Object(detail),
            })
        }
    }
}

fn assumptions_json(assumptions: &[AssumptionBoundary]) -> serde_json::Value {
    use serde_json::json;
    let items: Vec<serde_json::Value> = assumptions
        .iter()
        .map(|assumption| {
            let mut item = serde_json::Map::new();
            item.insert("id".to_string(), json!(assumption.id));
            item.insert("category".to_string(), json!(assumption.category.as_str()));
            item.insert("status".to_string(), json!(assumption.status));
            item.insert("message".to_string(), json!(assumption.message));
            item.insert("symbols".to_string(), json!(assumption.symbols));
            if assumption.id == ASSUMPTION_CRYPTO_ID {
                let intrinsic_levels: Vec<serde_json::Value> = assumption
                    .symbols
                    .iter()
                    .map(|symbol| {
                        json!({
                            "intrinsic": symbol,
                            "tier": "L0",
                            "label": "assumed",
                        })
                    })
                    .collect();
                if !intrinsic_levels.is_empty() {
                    item.insert(
                        "intrinsic_levels".to_string(),
                        serde_json::Value::Array(intrinsic_levels),
                    );
                }
            }
            serde_json::Value::Object(item)
        })
        .collect();
    json!({ "items": items })
}

fn vc_repair_hints_json(vc: &VerificationCondition) -> Option<serde_json::Value> {
    use serde_json::json;

    fn hint(
        kind: &'static str,
        message: &'static str,
        minimal_clause: String,
    ) -> serde_json::Value {
        json!({
            "kind": kind,
            "message": message,
            "minimal_clause": minimal_clause,
        })
    }

    fn variant_from_nonneg_post(post_ast: &str) -> String {
        post_ast
            .trim()
            .strip_suffix(">= 0")
            .map(str::trim)
            .filter(|expr| !expr.is_empty())
            .unwrap_or_else(|| post_ast.trim())
            .to_string()
    }

    fn variant_from_decrease_post(post_ast: &str) -> String {
        post_ast
            .split('<')
            .nth(1)
            .map(str::trim)
            .filter(|expr| !expr.is_empty())
            .unwrap_or_else(|| post_ast.trim())
            .to_string()
    }

    let mut items = Vec::new();
    if vc.vc_id.starts_with("mut_pre:") {
        items.push(hint(
            "contract.require",
            "Add or strengthen a precondition guard for the mut collection call.",
            format!("require {{ {} }}", vc.post.ast),
        ));
    } else if vc.vc_id.starts_with("loop:") && vc.vc_id.ends_with(":invariant") {
        items.push(hint(
            "loop.invariant",
            "Add or strengthen the loop invariant needed by this VC.",
            format!("invariant {{ {} }}", vc.post.ast),
        ));
    } else if vc.vc_id.starts_with("loop:") && vc.vc_id.ends_with(":variant_nonneg") {
        let variant_expr = variant_from_nonneg_post(&vc.post.ast);
        items.push(hint(
            "loop.variant_nonneg",
            "Use a loop variant that is always non-negative.",
            format!("variant {{ {} }}", variant_expr),
        ));
    } else if vc.vc_id.starts_with("loop:") && vc.vc_id.ends_with(":variant_decrease") {
        let variant_expr = variant_from_decrease_post(&vc.post.ast);
        items.push(hint(
            "loop.variant_decrease",
            "Use a loop variant that strictly decreases each iteration.",
            format!("variant {{ {} }}", variant_expr),
        ));
    } else if vc.vc_id.starts_with("vc:") {
        items.push(hint(
            "contract.ensure",
            "Add or strengthen a postcondition that matches the failed VC goal.",
            format!("ensure {{ {} }}", vc.post.ast),
        ));
    }

    if items.is_empty() {
        None
    } else {
        Some(json!({
            "on_status": "failed",
            "items": items,
        }))
    }
}

fn span_json(
    file: &str,
    role: &'static str,
    span: Option<clg_ast::Span>,
) -> Option<serde_json::Value> {
    use serde_json::json;
    let span = span?;
    Some(json!({
        "file": file,
        "start": span.start,
        "end": span.end,
        "role": role,
    }))
}

fn vc_clause_kind(vc_id: &str) -> &'static str {
    if vc_id.starts_with("mut_pre:") {
        "require"
    } else if vc_id.starts_with("loop:") && vc_id.ends_with(":invariant") {
        "invariant"
    } else if vc_id.starts_with("loop:") && vc_id.contains(":variant_") {
        "variant"
    } else if vc_id.starts_with("vc:") {
        "ensure"
    } else {
        "vc"
    }
}

fn vc_failure_slice_json(vc: &VerificationCondition, file: &str) -> Option<serde_json::Value> {
    use serde_json::json;

    if vc.pre.span.is_none() && vc.post.span.is_none() {
        return None;
    }

    let focus = span_json(file, "post", vc.post.span)
        .or_else(|| span_json(file, "pre", vc.pre.span))
        .expect("focus span should exist when any span exists");

    let mut related_spans = Vec::new();
    if let Some(pre) = span_json(file, "pre", vc.pre.span) {
        related_spans.push(pre);
    }
    if let Some(post) = span_json(file, "post", vc.post.span) {
        related_spans.push(post);
    }

    Some(json!({
        "on_status": "failed",
        "vc_id": vc.vc_id,
        "clause_kind": vc_clause_kind(&vc.vc_id),
        "focus_span": focus,
        "related_spans": related_spans,
    }))
}

fn extract_model_symbols(ast: &str) -> Vec<String> {
    let mut current = String::new();
    let mut tokens = BTreeSet::new();

    let flush = |cur: &mut String, out: &mut BTreeSet<String>| {
        if cur.is_empty() {
            return;
        }
        while cur.ends_with(':') {
            cur.pop();
        }
        while cur.starts_with(':') {
            cur.remove(0);
        }
        if cur.is_empty() {
            return;
        }
        if cur.chars().next().is_some_and(|c| c.is_ascii_digit()) {
            cur.clear();
            return;
        }
        if matches!(cur.as_str(), "true" | "false") {
            cur.clear();
            return;
        }
        out.insert(cur.clone());
        cur.clear();
    };

    for ch in ast.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == ':' {
            current.push(ch);
        } else {
            flush(&mut current, &mut tokens);
        }
    }
    flush(&mut current, &mut tokens);
    tokens.into_iter().collect()
}

fn vc_counterexample_json(vc: &VerificationCondition, file: &str) -> Option<serde_json::Value> {
    use serde_json::json;

    if vc.pre.span.is_none() && vc.post.span.is_none() {
        return None;
    }

    let focus = span_json(file, "post", vc.post.span)
        .or_else(|| span_json(file, "pre", vc.pre.span))
        .expect("focus span should exist when any span exists");
    let symbols = extract_model_symbols(&vc.post.ast);
    let bindings: Vec<serde_json::Value> = symbols
        .into_iter()
        .map(|symbol| {
            json!({
                "symbol": symbol,
                "value": serde_json::Value::Null,
            })
        })
        .collect();

    if let Some(model) = vc.counterexample.as_ref() {
        return Some(json!({
            "on_status": "failed",
            "state": "solver_model",
            "format": "clg.counterexample.v1",
            "reason": "solver found a concrete counterexample",
            "focus_span": focus,
            "bindings": bindings,
            "model_smt2": model,
        }));
    }

    Some(json!({
        "on_status": "failed",
        "state": "solver_unavailable",
        "format": "clg.counterexample.v1",
        "reason": "external solver/model not attached",
        "focus_span": focus,
        "bindings": bindings,
    }))
}

fn vc_model_snippet_json(vc: &VerificationCondition) -> serde_json::Value {
    use serde_json::json;
    let bindings: Vec<serde_json::Value> = extract_model_symbols(&vc.post.ast)
        .into_iter()
        .map(|symbol| {
            json!({
                "symbol": symbol,
                "value": serde_json::Value::Null,
            })
        })
        .collect();
    json!({
        "state": "solver_unavailable",
        "reason": "external solver/model not attached",
        "bindings": bindings,
    })
}

fn span_offsets_json(span: Option<clg_ast::Span>) -> Option<serde_json::Value> {
    use serde_json::json;
    let span = span?;
    Some(json!({
        "start": span.start,
        "end": span.end,
    }))
}

fn vc_proof_context_json(vc: &VerificationCondition) -> serde_json::Value {
    use serde_json::json;

    let mut span_map = serde_json::Map::new();
    let focus_role = if vc.post.span.is_some() {
        "post"
    } else if vc.pre.span.is_some() {
        "pre"
    } else {
        "none"
    };
    span_map.insert("focus_role".to_string(), json!(focus_role));
    if let Some(pre) = span_offsets_json(vc.pre.span) {
        span_map.insert("pre".to_string(), pre);
    }
    if let Some(post) = span_offsets_json(vc.post.span) {
        span_map.insert("post".to_string(), post);
    }

    json!({
        "on_status": "failed",
        "format": "clg.proof_context.v1",
        "vc": {
            "function": vc.function,
            "vc_id": vc.vc_id,
            "status": vc.status,
            "pre": {
                "ast": vc.pre.ast,
                "smt2": vc.pre.smt2,
            },
            "post": {
                "ast": vc.post.ast,
                "smt2": vc.post.smt2,
            },
            "clause_kind": vc_clause_kind(&vc.vc_id),
            "vc": {
                "smt2": vc.vc_smt2,
            },
        },
        "assumptions": assumptions_json(&vc.assumptions),
        "model_snippet": vc_model_snippet_json(vc),
        "span_map": serde_json::Value::Object(span_map),
    })
}
