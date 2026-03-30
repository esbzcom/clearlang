use crate::guards::{collect_mut_calls, guard_callee_for_kind, MutCall};
use clg_ast::{BinOp, Block, Effect, Expr, Func, Program, Span, Stmt, Type};
use std::collections::{BTreeSet, HashMap, HashSet};

use super::{
    snapshot_expr, AssumptionBoundary, AssumptionCategory, ContractExpr, LoopObligation,
    RefinementAttachment, RefinementObligation, RefinementPremise, VerificationCondition,
};
use crate::vc::linear::collect_linear_control_obligations;
use crate::vc::loops::{collect_loops, expr_span};
use crate::vc::refinements::{
    build_alias_map, build_fn_sigs, collect_refinement_obligations, fold_conjunction,
    make_refinement_obligation, merge_extras, refinement_prelude, smt_sort_for_type,
    substitute_result,
};
use crate::vc::smt::SmtEncoder;
use crate::vc::source::expr_to_source;

mod helpers;
use self::helpers::{
    append_smt_bounds, collect_function_assumptions, linear_branch_post_smt, linear_loop_post_smt,
    obligation_uses_only_symbols, premise_from_obligation, u64_bounds_smt,
};

const U64_MAX_SMT: &str = "18446744073709551615";
const ASSUMPTION_UNSIGNED_ID: &str = "unsigned.int_model";
const ASSUMPTION_BITWISE_ID: &str = "bitwise.uninterpreted";
const ASSUMPTION_CRYPTO_ID: &str = "crypto.uninterpreted";
const ASSUMPTION_PRIMITIVE_ID: &str = "primitive.unproved";
const ASSUMPTION_EXTERNAL_ID: &str = "external.dependency";
const ASSUMPTION_STATUS_ASSUMED: &str = "assumed";
const ASSUMPTION_UNSIGNED_MESSAGE: &str =
    "Unsigned values are modeled as SMT Int with bounded-domain guards where available; overflow and exact bit-level semantics are assumed.";
const ASSUMPTION_BITWISE_MESSAGE: &str =
    "Bitwise and shift operators (including bitwise-sensitive std::u64 intrinsics) are modeled as uninterpreted SMT functions.";
const ASSUMPTION_CRYPTO_MESSAGE: &str =
    "Crypto and constant-time intrinsics are modeled as uninterpreted SMT functions; cryptographic and side-channel guarantees are assumed.";
const ASSUMPTION_PRIMITIVE_MESSAGE: &str =
    "Primitive std dependencies are not formally proved and are treated as assumed boundaries.";
const ASSUMPTION_EXTERNAL_MESSAGE: &str =
    "External imported dependencies are outside the current proof kernel and are treated as assumed boundaries.";

pub fn generate_vcs(program: &Program) -> Vec<VerificationCondition> {
    generate_vcs_with_dependencies(program, &AssumptionDependencies::default())
}

#[derive(Debug, Clone, Default)]
pub struct AssumptionDependencies {
    pub non_proved_primitives: BTreeSet<String>,
    pub external_dependencies: BTreeSet<String>,
}

pub fn generate_vcs_with_dependencies(
    program: &Program,
    dependencies: &AssumptionDependencies,
) -> Vec<VerificationCondition> {
    let alias_map = build_alias_map(program);
    let fn_sigs = build_fn_sigs(program, &alias_map);
    let resource_names: HashSet<&str> = program
        .resources
        .iter()
        .map(|res| res.name.as_str())
        .collect();
    let mut out = Vec::new();
    for func in &program.funcs {
        struct EnsureItem {
            expr: Expr,
            span: Span,
            return_obligation: Option<RefinementObligation>,
        }
        let function_assumptions = collect_function_assumptions(func, dependencies);

        // VC preconditions follow runtime guard order: requires (source order),
        // then implicit alias predicates (params), then in-body obligations.
        let require_exprs: Vec<Expr> = func.requires.iter().map(|c| c.expr.clone()).collect();
        let mut pre_obligations: Vec<RefinementObligation> = Vec::new();
        if let Some(sig) = fn_sigs.get(func.name.as_str()) {
            for (param, alias_opt) in func.params.iter().zip(sig.param_aliases.iter()) {
                if let Some(alias) = alias_opt {
                    if let Some(obligation) = make_refinement_obligation(
                        alias,
                        &Expr::Var(param.name.clone(), alias.alias_span),
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
            if let Some(alias) = sig.ret_alias.as_ref() {
                if let Some(obligation) = make_refinement_obligation(
                    alias,
                    &Expr::Var("result".into(), alias.alias_span),
                    RefinementAttachment::result("result".to_string()),
                ) {
                    ensures.push(EnsureItem {
                        expr: obligation.predicate.clone(),
                        span: alias.alias_span,
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
        let allowed_symbols: HashSet<String> =
            func.params.iter().map(|param| param.name.clone()).collect();
        pre_obligations
            .retain(|obligation| obligation_uses_only_symbols(obligation, &allowed_symbols));
        let param_declarations = func
            .params
            .iter()
            .map(|param| {
                format!(
                    "(declare-const {} {})",
                    param.name,
                    smt_sort_for_type(&param.ty, &alias_map)
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

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
        let base_vc_extra = merge_extras(&[&param_declarations, &base_refinement_prelude]);
        let linear_control = collect_linear_control_obligations(func, &resource_names);

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
                let refinement_extra = if let Some(obligation) = &ensure.return_obligation {
                    let instantiated = instantiate_return_obligation(obligation, &func.body);
                    refinements.push(premise_from_obligation(obligation));
                    refinement_prelude(&pre_obligations, Some(&instantiated), &alias_map)
                } else {
                    base_refinement_prelude.clone()
                };
                let merged_extra = merge_extras(&[&param_declarations, &refinement_extra]);
                let vc_smt2 = wrap_vc_with_extra(&encoder, &vc_body, &merged_extra);

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
                    assumptions: function_assumptions.clone(),
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
                    type_args: Vec::new(),
                    args: vec![Expr::Var(arg_name.clone(), guard_span)],
                    span: guard_span,
                };
                let post_ast = expr_to_source(&guard_expr, 0);

                let mut encoder = SmtEncoder::default();
                let pre_smt = append_smt_bounds(encoder.encode(&pre_expr), &u64_param_bounds);
                let post_smt = encoder.encode(&guard_expr);
                let vc_body = format!("(=> {} {})", pre_smt, post_smt);
                let vc_smt2 = wrap_vc_with_extra(&encoder, &vc_body, &base_vc_extra);
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
                    assumptions: function_assumptions.clone(),
                });
            }
        }

        for (idx, branch_ob) in linear_control.branches.iter().enumerate() {
            let mut encoder = SmtEncoder::default();
            let pre_smt = append_smt_bounds(encoder.encode(&pre_expr), &u64_param_bounds);
            let post_ast = format!(
                "linear_state_consistent_across_branches({})",
                branch_ob.vars.join(", ")
            );
            let (linear_extra, post_smt) =
                linear_branch_post_smt("branch", idx, branch_ob.vars.len());
            let vc_body = format!("(=> {} {})", pre_smt, post_smt);
            let merged_extra = merge_extras(&[&base_vc_extra, &linear_extra]);
            let vc_smt2 = wrap_vc_with_extra(&encoder, &vc_body, &merged_extra);
            let refinements = pre_premises.clone();
            out.push(VerificationCondition {
                function: func.name.clone(),
                vc_id: format!("linear:branch:{}", idx),
                pre: ContractExpr {
                    ast: pre_ast.clone(),
                    smt2: pre_smt.clone(),
                    span: pre_span,
                },
                post: ContractExpr {
                    ast: post_ast,
                    smt2: post_smt,
                    span: Some(branch_ob.span),
                },
                vc_smt2,
                status: "generated",
                refinements,
                assumptions: function_assumptions.clone(),
            });
        }

        for (idx, loop_ob) in linear_control.loops.iter().enumerate() {
            let mut encoder = SmtEncoder::default();
            let pre_smt = append_smt_bounds(encoder.encode(&pre_expr), &u64_param_bounds);
            let post_ast = format!(
                "linear_state_preserved_across_loop({})",
                loop_ob.vars.join(", ")
            );
            let (linear_extra, post_smt) = linear_loop_post_smt("loop", idx, loop_ob.vars.len());
            let vc_body = format!("(=> {} {})", pre_smt, post_smt);
            let merged_extra = merge_extras(&[&base_vc_extra, &linear_extra]);
            let vc_smt2 = wrap_vc_with_extra(&encoder, &vc_body, &merged_extra);
            let refinements = pre_premises.clone();
            out.push(VerificationCondition {
                function: func.name.clone(),
                vc_id: format!("linear:loop:{}", idx),
                pre: ContractExpr {
                    ast: pre_ast.clone(),
                    smt2: pre_smt.clone(),
                    span: pre_span,
                },
                post: ContractExpr {
                    ast: post_ast,
                    smt2: post_smt,
                    span: Some(loop_ob.span),
                },
                vc_smt2,
                status: "generated",
                refinements,
                assumptions: function_assumptions.clone(),
            });
        }

        let mut loops: Vec<LoopObligation<'_>> = Vec::new();
        collect_loops(&func.body, &mut loops);
        for (idx, loop_ob) in loops.iter().enumerate() {
            let mut enc_inv = SmtEncoder::default();
            let pre_smt = append_smt_bounds(enc_inv.encode(&pre_expr), &u64_param_bounds);
            let inv_smt = enc_inv.encode(loop_ob.invariant);
            let vc_body = format!("(=> {} {})", pre_smt, inv_smt);
            let vc_smt2 = wrap_vc_with_extra(&enc_inv, &vc_body, &base_vc_extra);
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
                assumptions: function_assumptions.clone(),
            });

            if let Some(var_expr) = loop_ob.variant {
                let mut enc_var = SmtEncoder::default();
                let pre_smt = append_smt_bounds(enc_var.encode(&pre_expr), &u64_param_bounds);
                let var_smt = enc_var.encode(var_expr);
                let post_smt = format!("(>= {} 0)", var_smt);
                let vc_body = format!("(=> {} {})", pre_smt, post_smt);
                let vc_smt2 = wrap_vc_with_extra(&enc_var, &vc_body, &base_vc_extra);
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
                    assumptions: function_assumptions.clone(),
                });

                let mut enc_dec = SmtEncoder::default();
                let pre_smt = append_smt_bounds(enc_dec.encode(&pre_expr), &u64_param_bounds);
                let var_before = enc_dec.encode(var_expr);
                let next_sym = format!("cl.loop.variant.next.{}", idx);
                let post_smt = format!("(< {} {})", next_sym, var_before);
                let vc_body = format!("(=> {} {})", pre_smt, post_smt);
                let extra = format!("(declare-const {} Int)", next_sym);
                let merged_extra = merge_extras(&[&base_vc_extra, &extra]);
                let vc_smt2 = wrap_vc_with_extra(&enc_dec, &vc_body, &merged_extra);
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
                    assumptions: function_assumptions.clone(),
                });
            }
        }
    }
    out.sort_by(|a, b| a.function.cmp(&b.function).then(a.vc_id.cmp(&b.vc_id)));
    out
}

fn wrap_vc_with_extra(encoder: &SmtEncoder, body: &str, extra: &str) -> String {
    if extra.is_empty() {
        encoder.wrap_vc(body)
    } else {
        encoder.wrap_vc_with_extra(body, Some(extra))
    }
}

fn instantiate_return_obligation(
    obligation: &RefinementObligation,
    body: &Expr,
) -> RefinementObligation {
    RefinementObligation {
        alias: obligation.alias.clone(),
        binder: obligation.binder.clone(),
        substitution: substitute_result(&obligation.substitution, body),
        substitution_type: obligation.substitution_type.clone(),
        predicate: substitute_result(&obligation.predicate, body),
        attachment: obligation.attachment.clone(),
    }
}
