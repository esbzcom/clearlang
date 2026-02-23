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
    make_refinement_obligation, merge_extras, refinement_prelude, substitute_result,
};
use crate::vc::smt::SmtEncoder;
use crate::vc::source::expr_to_source;

const U64_MAX_SMT: &str = "18446744073709551615";
const ASSUMPTION_UNSIGNED_ID: &str = "unsigned.int_model";
const ASSUMPTION_BITWISE_ID: &str = "bitwise.uninterpreted";
const ASSUMPTION_CRYPTO_ID: &str = "crypto.uninterpreted";
const ASSUMPTION_STATUS_ASSUMED: &str = "assumed";
const ASSUMPTION_UNSIGNED_MESSAGE: &str =
    "Unsigned values are modeled as SMT Int with bounded-domain guards where available; overflow and exact bit-level semantics are assumed.";
const ASSUMPTION_BITWISE_MESSAGE: &str =
    "Bitwise and shift operators are modeled as uninterpreted SMT functions.";
const ASSUMPTION_CRYPTO_MESSAGE: &str =
    "Crypto and constant-time intrinsics are modeled as uninterpreted SMT functions; cryptographic and side-channel guarantees are assumed.";

pub fn generate_vcs(program: &Program) -> Vec<VerificationCondition> {
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
        let function_assumptions = collect_function_assumptions(func);

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
            let merged_extra = merge_extras(&[&base_refinement_prelude, &linear_extra]);
            let vc_smt2 = if merged_extra.is_empty() {
                encoder.wrap_vc(&vc_body)
            } else {
                encoder.wrap_vc_with_extra(&vc_body, Some(&merged_extra))
            };
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
            let merged_extra = merge_extras(&[&base_refinement_prelude, &linear_extra]);
            let vc_smt2 = if merged_extra.is_empty() {
                encoder.wrap_vc(&vc_body)
            } else {
                encoder.wrap_vc_with_extra(&vc_body, Some(&merged_extra))
            };
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
                assumptions: function_assumptions.clone(),
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
                    assumptions: function_assumptions.clone(),
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
                    assumptions: function_assumptions.clone(),
                });
            }
        }
    }
    out.sort_by(|a, b| a.function.cmp(&b.function).then(a.vc_id.cmp(&b.vc_id)));
    out
}

#[derive(Default)]
struct AssumptionUsage {
    unsigned_types: BTreeSet<String>,
    bitwise_ops: BTreeSet<String>,
    crypto_intrinsics: BTreeSet<String>,
}

fn collect_function_assumptions(func: &Func) -> Vec<AssumptionBoundary> {
    let mut usage = AssumptionUsage::default();
    for param in &func.params {
        collect_unsigned_from_type(&param.ty, &mut usage.unsigned_types);
    }
    collect_unsigned_from_type(&func.ret, &mut usage.unsigned_types);
    for req in &func.requires {
        collect_assumption_usage_expr(&req.expr, &mut usage);
    }
    for ensure in &func.ensures {
        collect_assumption_usage_expr(&ensure.expr, &mut usage);
    }
    collect_assumption_usage_expr(&func.body, &mut usage);

    let mut assumptions = Vec::new();
    if !usage.unsigned_types.is_empty() {
        assumptions.push(AssumptionBoundary {
            id: ASSUMPTION_UNSIGNED_ID,
            category: AssumptionCategory::Unsigned,
            status: ASSUMPTION_STATUS_ASSUMED,
            message: ASSUMPTION_UNSIGNED_MESSAGE,
            symbols: usage.unsigned_types.iter().cloned().collect(),
        });
    }
    if !usage.bitwise_ops.is_empty() {
        assumptions.push(AssumptionBoundary {
            id: ASSUMPTION_BITWISE_ID,
            category: AssumptionCategory::Bitwise,
            status: ASSUMPTION_STATUS_ASSUMED,
            message: ASSUMPTION_BITWISE_MESSAGE,
            symbols: usage.bitwise_ops.iter().cloned().collect(),
        });
    }
    if !usage.crypto_intrinsics.is_empty() {
        assumptions.push(AssumptionBoundary {
            id: ASSUMPTION_CRYPTO_ID,
            category: AssumptionCategory::Crypto,
            status: ASSUMPTION_STATUS_ASSUMED,
            message: ASSUMPTION_CRYPTO_MESSAGE,
            symbols: usage.crypto_intrinsics.iter().cloned().collect(),
        });
    }
    assumptions
}

fn collect_unsigned_from_type(ty: &Type, out: &mut BTreeSet<String>) {
    match ty {
        Type::U8 => {
            out.insert("U8".to_string());
        }
        Type::U64 => {
            out.insert("U64".to_string());
        }
        Type::U128 => {
            out.insert("U128".to_string());
        }
        Type::U256 => {
            out.insert("U256".to_string());
        }
        Type::Option(inner)
        | Type::List(inner)
        | Type::Set(inner)
        | Type::Array(inner, _)
        | Type::Slice(inner) => collect_unsigned_from_type(inner, out),
        Type::Result(ok, err) | Type::Map(ok, err) => {
            collect_unsigned_from_type(ok, out);
            collect_unsigned_from_type(err, out);
        }
        Type::Tuple(items) | Type::Named { args: items, .. } => {
            for item in items {
                collect_unsigned_from_type(item, out);
            }
        }
        Type::Fn { params, ret } => {
            for param in params {
                collect_unsigned_from_type(param, out);
            }
            collect_unsigned_from_type(ret, out);
        }
        Type::Int | Type::Bool | Type::String | Type::Bytes => {}
    }
}

fn collect_assumption_usage_block(block: &Block, out: &mut AssumptionUsage) {
    for stmt in &block.statements {
        match stmt {
            Stmt::Let { expr, .. } | Stmt::Expr { expr, .. } => {
                collect_assumption_usage_expr(expr, out);
            }
            Stmt::While {
                cond,
                invariant,
                variant,
                body,
                ..
            } => {
                collect_assumption_usage_expr(cond, out);
                collect_assumption_usage_expr(invariant, out);
                if let Some(variant_expr) = variant {
                    collect_assumption_usage_expr(variant_expr, out);
                }
                collect_assumption_usage_block(body, out);
            }
        }
    }
    if let Some(tail) = &block.tail {
        collect_assumption_usage_expr(tail, out);
    }
}

fn collect_assumption_usage_expr(expr: &Expr, out: &mut AssumptionUsage) {
    match expr {
        Expr::Int(..) | Expr::Bool(..) | Expr::String(..) | Expr::Var(..) => {}
        Expr::ArrayLit { elems, .. } | Expr::TupleLit { elems, .. } => {
            for elem in elems {
                collect_assumption_usage_expr(elem, out);
            }
        }
        Expr::StructLit { fields, .. } => {
            for field in fields {
                collect_assumption_usage_expr(&field.expr, out);
            }
        }
        Expr::FieldAccess { base, .. } => collect_assumption_usage_expr(base, out),
        Expr::Block { block } => collect_assumption_usage_block(block, out),
        Expr::Bin { op, lhs, rhs, .. } => {
            if let Some(symbol) = bitwise_symbol(*op) {
                out.bitwise_ops.insert(symbol.to_string());
            }
            collect_assumption_usage_expr(lhs, out);
            collect_assumption_usage_expr(rhs, out);
        }
        Expr::Call {
            callee,
            type_args,
            args,
            ..
        } => {
            for ty in type_args {
                collect_unsigned_from_type(ty, &mut out.unsigned_types);
            }
            for arg in args {
                collect_assumption_usage_expr(arg, out);
            }
            if matches!(callee.as_str(), "U8" | "U64" | "U128" | "U256") {
                out.unsigned_types.insert(callee.clone());
            }
            if callee.starts_with("std::u64::") {
                out.unsigned_types.insert("U64".to_string());
            }
            if callee.starts_with("std::u128::") {
                out.unsigned_types.insert("U128".to_string());
            }
            if callee.starts_with("std::u256::") {
                out.unsigned_types.insert("U256".to_string());
            }
            if is_crypto_assumption_intrinsic(callee) {
                out.crypto_intrinsics.insert(callee.clone());
            }
        }
        Expr::Return { expr, .. } | Expr::Unary { expr, .. } | Expr::Try { expr, .. } => {
            collect_assumption_usage_expr(expr, out);
        }
        Expr::Match {
            scrutinee, arms, ..
        } => {
            collect_assumption_usage_expr(scrutinee, out);
            for arm in arms {
                collect_assumption_usage_expr(&arm.expr, out);
            }
        }
        Expr::If {
            cond,
            then_br,
            else_br,
            ..
        } => {
            collect_assumption_usage_expr(cond, out);
            collect_assumption_usage_expr(then_br, out);
            collect_assumption_usage_expr(else_br, out);
        }
        Expr::Index { base, index, .. } => {
            collect_assumption_usage_expr(base, out);
            collect_assumption_usage_expr(index, out);
        }
        Expr::Lambda { params, body, .. } => {
            for param in params {
                collect_unsigned_from_type(&param.ty, &mut out.unsigned_types);
            }
            collect_assumption_usage_expr(body, out);
        }
    }
}

fn bitwise_symbol(op: BinOp) -> Option<&'static str> {
    match op {
        BinOp::Shl => Some("<<"),
        BinOp::Shr => Some(">>"),
        BinOp::BitAnd => Some("&"),
        BinOp::BitXor => Some("^"),
        BinOp::BitOr => Some("|"),
        _ => None,
    }
}

fn is_crypto_assumption_intrinsic(callee: &str) -> bool {
    matches!(
        callee,
        "std::crypto::hash" | "std::crypto::hmac" | "std::crypto::verify" | "std::bytes::eq_ct"
    )
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

fn linear_branch_post_smt(scope: &str, idx: usize, vars: usize) -> (String, String) {
    linear_state_post_smt(scope, idx, vars, "then", "else")
}

fn linear_loop_post_smt(scope: &str, idx: usize, vars: usize) -> (String, String) {
    linear_state_post_smt(scope, idx, vars, "before", "after")
}

fn linear_state_post_smt(
    scope: &str,
    idx: usize,
    vars: usize,
    left_label: &str,
    right_label: &str,
) -> (String, String) {
    if vars == 0 {
        return (String::new(), "true".to_string());
    }

    let mut declarations = Vec::new();
    let mut checks = Vec::new();
    for var_idx in 0..vars {
        let left = format!("cl.linear.{}.{}.{}.{}", scope, idx, var_idx, left_label);
        let right = format!("cl.linear.{}.{}.{}.{}", scope, idx, var_idx, right_label);
        declarations.push(format!("(declare-const {} Int)", left));
        declarations.push(format!("(declare-const {} Int)", right));
        checks.push(format!("(= {} {})", left, right));
        checks.push(format!("(>= {} 0)", left));
        checks.push(format!("(>= {} 0)", right));
    }
    let post = if checks.len() == 1 {
        checks[0].clone()
    } else {
        format!("(and {})", checks.join(" "))
    };
    (declarations.join("\n"), post)
}
