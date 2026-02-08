use crate::guards::{collect_mut_calls, guard_callee_for_kind, MutCall};
use clg_ast::{Effect, Expr, Program, Span, Type};
use std::collections::HashMap;

use super::{
    snapshot_expr, ContractExpr, LoopObligation, RefinementAttachment, RefinementObligation,
    RefinementPremise, VerificationCondition,
};
use crate::vc::loops::{collect_loops, expr_span};
use crate::vc::refinements::{
    build_alias_map, build_fn_sigs, collect_refinement_obligations, fold_conjunction,
    make_refinement_obligation, merge_extras, refinement_prelude, substitute_result,
};
use crate::vc::smt::SmtEncoder;
use crate::vc::source::expr_to_source;

const U64_MAX_SMT: &str = "18446744073709551615";

pub fn generate_vcs(program: &Program) -> Vec<VerificationCondition> {
    let alias_map = build_alias_map(program);
    let fn_sigs = build_fn_sigs(program, &alias_map);
    let mut out = Vec::new();
    for func in &program.funcs {
        struct EnsureItem {
            expr: Expr,
            span: Span,
            return_obligation: Option<RefinementObligation>,
        }

        // VC preconditions follow runtime guard order: requires (source order),
        // then implicit alias predicates (params), then in-body obligations.
        let require_exprs: Vec<Expr> = func.requires.iter().map(|c| c.expr.clone()).collect();
        let mut pre_obligations: Vec<RefinementObligation> = Vec::new();
        if let Some(sig) = fn_sigs.get(func.name.as_str()) {
            for (param, alias_opt) in func.params.iter().zip(sig.param_aliases.iter()) {
                if let Some(alias) = alias_opt {
                    if let Some(obligation) = make_refinement_obligation(
                        alias,
                        &Expr::Var(param.name.clone(), alias.span),
                        RefinementAttachment::param(param.name.clone()),
                    ) {
                        pre_obligations.push(obligation);
                    }
                }
            }
        }
        let u64_param_bounds: Vec<String> = func
            .params
            .iter()
            .filter(|p| matches!(p.ty, Type::U64))
            .map(|p| u64_bounds_smt(p.name.as_str()))
            .collect();

        // Ensure VCs follow source order, with an implicit refined-return predicate appended.
        let mut ensures: Vec<EnsureItem> = func
            .ensures
            .iter()
            .map(|e| EnsureItem {
                expr: e.expr.clone(),
                span: e.span,
                return_obligation: None,
            })
            .collect();
        if let Some(sig) = fn_sigs.get(func.name.as_str()) {
            if let Some(alias) = sig.ret_alias {
                if let Some(obligation) = make_refinement_obligation(
                    alias,
                    &Expr::Var("result".into(), alias.span),
                    RefinementAttachment::result("result".to_string()),
                ) {
                    ensures.push(EnsureItem {
                        expr: obligation.predicate.clone(),
                        span: alias.span,
                        return_obligation: Some(obligation),
                    });
                }
            }
        }

        // Obligations arising from alias flow inside the body (lets, calls, matches, etc.).
        let mut body_obligations: Vec<RefinementObligation> = Vec::new();
        let mut env: HashMap<String, Type> = func
            .params
            .iter()
            .map(|p| (p.name.clone(), p.ty.clone()))
            .collect();
        collect_refinement_obligations(
            &func.body,
            &alias_map,
            &fn_sigs,
            &mut env,
            &mut body_obligations,
        );
        pre_obligations.extend(body_obligations.into_iter());

        let mut all_pre = require_exprs.clone();
        all_pre.extend(pre_obligations.iter().map(|ob| ob.predicate.clone()));
        let pre_expr = if all_pre.is_empty() {
            Expr::Bool(true, Span { start: 0, end: 0 })
        } else {
            fold_conjunction(all_pre)
        };
        let pre_span = if func.requires.is_empty() {
            None
        } else {
            let start = func.requires.first().unwrap().span.start;
            let end = func.requires.last().unwrap().span.end;
            Some(Span { start, end })
        };
        let pre_ast = expr_to_source(&pre_expr, 0);
        let pre_premises: Vec<RefinementPremise> = pre_obligations
            .iter()
            .map(premise_from_obligation)
            .collect();
        let base_refinement_prelude = refinement_prelude(&pre_obligations, None, &alias_map);

        if matches!(func.effect, Effect::None | Effect::Pure) && !ensures.is_empty() {
            let has_u64_ret = matches!(func.ret, Type::U64);
            for (idx, ensure) in ensures.iter().enumerate() {
                let post_ast = expr_to_source(&ensure.expr, 0);
                let substituted = substitute_result(&ensure.expr, &func.body);

                let mut encoder = SmtEncoder::default();
                let pre_smt = append_smt_bounds(encoder.encode(&pre_expr), &u64_param_bounds);
                let post_smt = encoder.encode(&ensure.expr);
                let substituted_smt = encoder.encode(&substituted);
                let substituted_smt = if has_u64_ret {
                    let bounds = u64_bounds_smt(&substituted_smt);
                    format!("(and {} {})", substituted_smt, bounds)
                } else {
                    substituted_smt
                };
                let vc_body = format!("(=> {} {})", pre_smt, substituted_smt);
                let mut refinements = pre_premises.clone();
                let refinement_prelude = if let Some(obligation) = &ensure.return_obligation {
                    refinements.push(premise_from_obligation(obligation));
                    refinement_prelude(&pre_obligations, Some(obligation), &alias_map)
                } else {
                    base_refinement_prelude.clone()
                };
                let vc_smt2 = if refinement_prelude.is_empty() {
                    encoder.wrap_vc(&vc_body)
                } else {
                    encoder.wrap_vc_with_extra(&vc_body, Some(&refinement_prelude))
                };

                out.push(VerificationCondition {
                    function: func.name.clone(),
                    vc_id: format!("vc:{}", idx),
                    pre: ContractExpr {
                        ast: pre_ast.clone(),
                        smt2: pre_smt.clone(),
                        span: pre_span,
                    },
                    post: ContractExpr {
                        ast: post_ast,
                        smt2: post_smt,
                        span: Some(ensure.span),
                    },
                    vc_smt2,
                    status: "generated",
                    refinements,
                });
            }
        }

        let mut mut_calls: Vec<MutCall> = Vec::new();
        collect_mut_calls(&func.body, &mut mut_calls);
        if !mut_calls.is_empty() {
            for (idx, call) in mut_calls.into_iter().enumerate() {
                let Some(arg_name) = call.target else {
                    continue;
                };
                let guard_span = call.span;
                let guard_expr = Expr::Call {
                    callee: guard_callee_for_kind(call.kind).to_string(),
                    args: vec![Expr::Var(arg_name.clone(), guard_span)],
                    span: guard_span,
                };
                let post_ast = expr_to_source(&guard_expr, 0);

                let mut encoder = SmtEncoder::default();
                let pre_smt = append_smt_bounds(encoder.encode(&pre_expr), &u64_param_bounds);
                let post_smt = encoder.encode(&guard_expr);
                let vc_body = format!("(=> {} {})", pre_smt, post_smt);
                let vc_smt2 = if base_refinement_prelude.is_empty() {
                    encoder.wrap_vc(&vc_body)
                } else {
                    encoder.wrap_vc_with_extra(&vc_body, Some(&base_refinement_prelude))
                };
                let refinements = pre_premises.clone();

                out.push(VerificationCondition {
                    function: func.name.clone(),
                    vc_id: format!("mut_pre:{}:{}", call.callee, idx),
                    pre: ContractExpr {
                        ast: pre_ast.clone(),
                        smt2: pre_smt.clone(),
                        span: pre_span,
                    },
                    post: ContractExpr {
                        ast: post_ast,
                        smt2: post_smt,
                        span: Some(guard_span),
                    },
                    vc_smt2,
                    status: "generated",
                    refinements,
                });
            }
        }

        let mut loops: Vec<LoopObligation<'_>> = Vec::new();
        collect_loops(&func.body, &mut loops);
        for (idx, loop_ob) in loops.iter().enumerate() {
            let mut enc_inv = SmtEncoder::default();
            let pre_smt = append_smt_bounds(enc_inv.encode(&pre_expr), &u64_param_bounds);
            let inv_smt = enc_inv.encode(loop_ob.invariant);
            let vc_body = format!("(=> {} {})", pre_smt, inv_smt);
            let vc_smt2 = if base_refinement_prelude.is_empty() {
                enc_inv.wrap_vc(&vc_body)
            } else {
                enc_inv.wrap_vc_with_extra(&vc_body, Some(&base_refinement_prelude))
            };
            let refinements = pre_premises.clone();

            out.push(VerificationCondition {
                function: func.name.clone(),
                vc_id: format!("loop:{}:invariant", idx),
                pre: ContractExpr {
                    ast: pre_ast.clone(),
                    smt2: pre_smt.clone(),
                    span: pre_span,
                },
                post: ContractExpr {
                    ast: expr_to_source(loop_ob.invariant, 0),
                    smt2: inv_smt,
                    span: Some(expr_span(loop_ob.invariant)),
                },
                vc_smt2,
                status: "generated",
                refinements,
            });

            if let Some(var_expr) = loop_ob.variant {
                let mut enc_var = SmtEncoder::default();
                let pre_smt = append_smt_bounds(enc_var.encode(&pre_expr), &u64_param_bounds);
                let var_smt = enc_var.encode(var_expr);
                let post_smt = format!("(>= {} 0)", var_smt);
                let vc_body = format!("(=> {} {})", pre_smt, post_smt);
                let vc_smt2 = if base_refinement_prelude.is_empty() {
                    enc_var.wrap_vc(&vc_body)
                } else {
                    enc_var.wrap_vc_with_extra(&vc_body, Some(&base_refinement_prelude))
                };
                let refinements = pre_premises.clone();
                out.push(VerificationCondition {
                    function: func.name.clone(),
                    vc_id: format!("loop:{}:variant_nonneg", idx),
                    pre: ContractExpr {
                        ast: pre_ast.clone(),
                        smt2: pre_smt.clone(),
                        span: pre_span,
                    },
                    post: ContractExpr {
                        ast: format!("{} >= 0", expr_to_source(var_expr, 0)),
                        smt2: post_smt,
                        span: Some(expr_span(var_expr)),
                    },
                    vc_smt2,
                    status: "generated",
                    refinements,
                });

                let mut enc_dec = SmtEncoder::default();
                let pre_smt = append_smt_bounds(enc_dec.encode(&pre_expr), &u64_param_bounds);
                let var_before = enc_dec.encode(var_expr);
                let next_sym = format!("cl.loop.variant.next.{}", idx);
                let post_smt = format!("(< {} {})", next_sym, var_before);
                let vc_body = format!("(=> {} {})", pre_smt, post_smt);
                let extra = format!("(declare-const {} Int)", next_sym);
                let merged_extra = merge_extras(&[&base_refinement_prelude, &extra]);
                let vc_smt2 = if merged_extra.is_empty() {
                    enc_dec.wrap_vc(&vc_body)
                } else {
                    enc_dec.wrap_vc_with_extra(&vc_body, Some(&merged_extra))
                };
                let refinements = pre_premises.clone();
                out.push(VerificationCondition {
                    function: func.name.clone(),
                    vc_id: format!("loop:{}:variant_decrease", idx),
                    pre: ContractExpr {
                        ast: pre_ast.clone(),
                        smt2: pre_smt.clone(),
                        span: pre_span,
                    },
                    post: ContractExpr {
                        ast: format!(
                            "{}' < {}",
                            expr_to_source(var_expr, 0),
                            expr_to_source(var_expr, 0)
                        ),
                        smt2: post_smt,
                        span: Some(expr_span(var_expr)),
                    },
                    vc_smt2,
                    status: "generated",
                    refinements,
                });
            }
        }
    }
    out.sort_by(|a, b| a.function.cmp(&b.function).then(a.vc_id.cmp(&b.vc_id)));
    out
}


fn u64_bounds_smt(term: &str) -> String {
    format!("(and (<= 0 {term}) (<= {term} {U64_MAX_SMT}))")
}

fn append_smt_bounds(base: String, bounds: &[String]) -> String {
    if bounds.is_empty() {
        base
    } else {
        format!("(and {} {})", base, bounds.join(" "))
    }
}

fn premise_from_obligation(obligation: &RefinementObligation) -> RefinementPremise {
    RefinementPremise {
        alias: obligation.alias.clone(),
        binder: obligation.binder.clone(),
        substitution: snapshot_expr(&obligation.substitution),
        predicate: snapshot_expr(&obligation.predicate),
        attachment: obligation.attachment.clone(),
    }
}
