use crate::guards::{collect_mut_calls, guard_callee_for_kind, MutCall};
use clg_ast::{BinOp, Block, ContractDecl, Effect, Expr, Func, Program, Span, Stmt, Type};
use sha2::{Digest, Sha256};
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
    obligation_uses_only_symbols, premise_from_obligation, u64_bitvector_bindings_smt,
    u64_bounds_smt,
};

const U64_MAX_SMT: &str = "18446744073709551615";
const ASSUMPTION_UNSIGNED_ID: &str = "unsigned.int_model";
const ASSUMPTION_BITWISE_ID: &str = "bitwise.uninterpreted";
const ASSUMPTION_CRYPTO_ID: &str = "crypto.uninterpreted";
const ASSUMPTION_PRIMITIVE_ID: &str = "primitive.unproved";
const ASSUMPTION_EXTERNAL_ID: &str = "external.dependency";
const ASSUMPTION_STATUS_ASSUMED: &str = "assumed";
const ASSUMPTION_UNSIGNED_MESSAGE: &str =
    "Uncovered unsigned values (currently non-U64 paths) are modeled as SMT Int with bounded-domain guards where available; overflow and exact bit-level semantics are assumed.";
const ASSUMPTION_BITWISE_MESSAGE: &str =
    "Uncovered bitwise/shift surfaces (outside the U64-covered operator/intrinsic set) are modeled as uninterpreted SMT functions.";
const ASSUMPTION_CRYPTO_MESSAGE: &str =
    "Crypto and constant-time intrinsics are modeled as uninterpreted SMT functions; cryptographic and side-channel guarantees are assumed.";
const ASSUMPTION_PRIMITIVE_MESSAGE: &str =
    "Primitive std dependencies are not formally proved and are treated as assumed boundaries.";
const ASSUMPTION_EXTERNAL_MESSAGE: &str =
    "External imported dependencies are outside the current proof kernel and are treated as assumed boundaries.";
const ASSUMPTION_STATE_TRANSITION_ID: &str = "contract.state.transition.adapter";
const ASSUMPTION_STATE_TRANSITION_MESSAGE: &str =
    "Contract state transition symbols are emitted, but their target adapter and solver semantics are not yet proved.";

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
        let mut function_assumptions = collect_function_assumptions(func, dependencies);
        let contract_state = contract_for_function(program, func);
        let contract_model = contract_state.map(|contract| state_transition_model(func, contract));
        if let Some(model) = contract_model.as_ref() {
            function_assumptions.extend(state_model_assumptions(model, contract_state.unwrap()));
        }

        // VC preconditions follow runtime guard order: requires (source order),
        // then implicit alias predicates (params), then in-body obligations.
        let require_exprs: Vec<Expr> = func
            .requires
            .iter()
            .map(|c| rewrite_contract_state_expr(&c.expr, contract_state, StatePhase::Pre))
            .collect();
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
        let u64_param_names: Vec<String> = func
            .params
            .iter()
            .filter(|p| matches!(p.ty, Type::U64))
            .map(|p| p.name.clone())
            .collect();
        let u64_param_bounds: Vec<String> = u64_param_names
            .iter()
            .map(|name| u64_bounds_smt(name.as_str()))
            .collect();
        let u64_bitvector_bindings = u64_bitvector_bindings_smt(&u64_param_names);

        // Ensure VCs follow source order, with an implicit refined-return predicate appended.
        let mut ensures: Vec<EnsureItem> = func
            .ensures
            .iter()
            .map(|e| EnsureItem {
                expr: rewrite_contract_state_expr(&e.expr, contract_state, StatePhase::Post),
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
        let state_symbol_declarations = contract_model
            .as_ref()
            .map(|model| model.declarations.clone())
            .unwrap_or_default();
        let base_vc_extra = merge_extras(&[
            &param_declarations,
            &base_refinement_prelude,
            &u64_bitvector_bindings,
            &state_symbol_declarations,
        ]);
        let linear_control = collect_linear_control_obligations(func, &resource_names);

        if (matches!(func.effect, Effect::None | Effect::Pure)
            || (matches!(func.effect, Effect::Mut) && contract_state.is_some()))
            && !ensures.is_empty()
        {
            let has_u64_ret = matches!(func.ret, Type::U64);
            for (idx, ensure) in ensures.iter().enumerate() {
                let post_ast = expr_to_source(&ensure.expr, 0);
                let rewritten_body =
                    rewrite_contract_state_expr(&func.body, contract_state, StatePhase::Post);
                let substituted = substitute_result(&ensure.expr, &rewritten_body);

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
                let premise = transition_premise(&pre_smt, contract_model.as_ref());
                let vc_body = format!("(=> {} {})", premise, substituted_smt);
                let mut refinements = pre_premises.clone();
                let refinement_extra = if let Some(obligation) = &ensure.return_obligation {
                    let instantiated = instantiate_return_obligation(obligation, &func.body);
                    refinements.push(premise_from_obligation(obligation));
                    refinement_prelude(&pre_obligations, Some(&instantiated), &alias_map)
                } else {
                    base_refinement_prelude.clone()
                };
                let merged_extra = merge_extras(&[
                    &param_declarations,
                    &refinement_extra,
                    &u64_bitvector_bindings,
                    &state_symbol_declarations,
                ]);
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
                    counterexample: None,
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
                let premise = transition_premise(&pre_smt, contract_model.as_ref());
                let vc_body = format!("(=> {} {})", premise, post_smt);
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
                    counterexample: None,
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
                counterexample: None,
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
                counterexample: None,
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
                counterexample: None,
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
                    counterexample: None,
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
                    counterexample: None,
                    refinements,
                    assumptions: function_assumptions.clone(),
                });
            }
        }

        if matches!(func.effect, Effect::Mut) {
            if let (Some(contract), Some(model)) = (contract_state, contract_model.as_ref()) {
                out.push(state_transition_vc(func, contract, model));
                out.extend(state_invariant_vcs(func, contract, model));
            }
        }
    }
    for contract in &program.contracts {
        if let Some(init) = contract.init.as_ref() {
            let model = state_transition_model_for_expr(&init.body, contract, None);
            out.extend(state_invariant_vcs_for(
                &format!("{}::init", contract.name),
                "init-invariant",
                contract,
                &model,
            ));
        }
        if let Some(migration) = contract.migration.as_ref() {
            let model = state_transition_model_for_expr(&migration.body, contract, None);
            out.extend(state_invariant_vcs_for(
                &format!("{}::migrate", contract.name),
                "migration-invariant",
                contract,
                &model,
            ));
        }
    }
    out.sort_by(|a, b| a.function.cmp(&b.function).then(a.vc_id.cmp(&b.vc_id)));
    out
}

fn contract_for_function<'a>(program: &'a Program, func: &Func) -> Option<&'a ContractDecl> {
    program.contracts.iter().find(|contract| {
        contract
            .functions
            .iter()
            .any(|member| member.name == func.name)
    })
}

fn state_field_id(contract: &ContractDecl, field_name: &str) -> String {
    let input = format!(
        "clg.contract-state-field.v1\0{}\0{field_name}",
        contract.name
    );
    format!("sha256:{:x}", Sha256::digest(input.as_bytes()))
}

fn smt_state_symbol(phase: &str, field_id: &str) -> String {
    format!("|clg.state.{phase}.{field_id}|")
}

#[derive(Clone, Copy)]
enum StatePhase {
    Pre,
    Post,
}

impl StatePhase {
    fn smt_name(self) -> &'static str {
        match self {
            Self::Pre => "pre",
            Self::Post => "post",
        }
    }
}

fn state_symbol_declarations(contract: &ContractDecl) -> String {
    contract
        .fields
        .iter()
        .flat_map(|field| {
            let field_id = state_field_id(contract, &field.name);
            let sort = smt_sort_for_type(&field.ty, &HashMap::new());
            [
                format!(
                    "(declare-const {} {sort})",
                    smt_state_symbol("pre", &field_id)
                ),
                format!(
                    "(declare-const {} {sort})",
                    smt_state_symbol("post", &field_id)
                ),
            ]
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn rewrite_contract_state_expr(
    expr: &Expr,
    contract: Option<&ContractDecl>,
    phase: StatePhase,
) -> Expr {
    match expr {
        Expr::FieldAccess { base, field, span } if matches!(base.as_ref(), Expr::Var(name, _) if name == "state") =>
        {
            if let Some(contract) = contract.filter(|contract| {
                contract
                    .fields
                    .iter()
                    .any(|state_field| state_field.name == *field)
            }) {
                return Expr::Var(
                    smt_state_symbol(phase.smt_name(), &state_field_id(contract, field.as_str())),
                    *span,
                );
            }
            expr.clone()
        }
        Expr::Call { callee, args, .. } if callee == "__clg_old" && args.len() == 1 => {
            rewrite_contract_state_expr(&args[0], contract, StatePhase::Pre)
        }
        Expr::ArrayLit { elems, span } => Expr::ArrayLit {
            elems: elems
                .iter()
                .map(|elem| rewrite_contract_state_expr(elem, contract, phase))
                .collect(),
            span: *span,
        },
        Expr::TupleLit { elems, span } => Expr::TupleLit {
            elems: elems
                .iter()
                .map(|elem| rewrite_contract_state_expr(elem, contract, phase))
                .collect(),
            span: *span,
        },
        Expr::StructLit { name, fields, span } => Expr::StructLit {
            name: name.clone(),
            fields: fields
                .iter()
                .map(|field| clg_ast::StructFieldInit {
                    name: field.name.clone(),
                    expr: rewrite_contract_state_expr(&field.expr, contract, phase),
                    span: field.span,
                })
                .collect(),
            span: *span,
        },
        Expr::FieldAccess { base, field, span } => Expr::FieldAccess {
            base: Box::new(rewrite_contract_state_expr(base, contract, phase)),
            field: field.clone(),
            span: *span,
        },
        Expr::Block { block } => Expr::Block {
            block: Box::new(rewrite_contract_state_block(block, contract, phase)),
        },
        Expr::Bin { op, lhs, rhs, span } => Expr::Bin {
            op: *op,
            lhs: Box::new(rewrite_contract_state_expr(lhs, contract, phase)),
            rhs: Box::new(rewrite_contract_state_expr(rhs, contract, phase)),
            span: *span,
        },
        Expr::Call {
            callee,
            type_args,
            args,
            span,
        } => Expr::Call {
            callee: callee.clone(),
            type_args: type_args.clone(),
            args: args
                .iter()
                .map(|arg| rewrite_contract_state_expr(arg, contract, phase))
                .collect(),
            span: *span,
        },
        Expr::Return { expr, span } => Expr::Return {
            expr: Box::new(rewrite_contract_state_expr(expr, contract, phase)),
            span: *span,
        },
        Expr::Unary { op, expr, span } => Expr::Unary {
            op: *op,
            expr: Box::new(rewrite_contract_state_expr(expr, contract, phase)),
            span: *span,
        },
        Expr::Match {
            scrutinee,
            arms,
            span,
        } => Expr::Match {
            scrutinee: Box::new(rewrite_contract_state_expr(scrutinee, contract, phase)),
            arms: arms
                .iter()
                .map(|arm| clg_ast::MatchArm {
                    pat: arm.pat.clone(),
                    expr: rewrite_contract_state_expr(&arm.expr, contract, phase),
                })
                .collect(),
            span: *span,
        },
        Expr::If {
            cond,
            then_br,
            else_br,
            span,
        } => Expr::If {
            cond: Box::new(rewrite_contract_state_expr(cond, contract, phase)),
            then_br: Box::new(rewrite_contract_state_expr(then_br, contract, phase)),
            else_br: Box::new(rewrite_contract_state_expr(else_br, contract, phase)),
            span: *span,
        },
        Expr::Index { base, index, span } => Expr::Index {
            base: Box::new(rewrite_contract_state_expr(base, contract, phase)),
            index: Box::new(rewrite_contract_state_expr(index, contract, phase)),
            span: *span,
        },
        Expr::Try { expr, span } => Expr::Try {
            expr: Box::new(rewrite_contract_state_expr(expr, contract, phase)),
            span: *span,
        },
        Expr::Lambda { params, body, span } => Expr::Lambda {
            params: params.clone(),
            body: Box::new(rewrite_contract_state_expr(body, contract, phase)),
            span: *span,
        },
        Expr::Int(_, _) | Expr::Bool(_, _) | Expr::String(_, _) | Expr::Var(_, _) => expr.clone(),
    }
}

fn rewrite_contract_state_block(
    block: &Block,
    contract: Option<&ContractDecl>,
    phase: StatePhase,
) -> Block {
    Block {
        statements: block
            .statements
            .iter()
            .map(|stmt| match stmt {
                Stmt::Let { name, expr, span } => Stmt::Let {
                    name: name.clone(),
                    expr: Box::new(rewrite_contract_state_expr(expr, contract, phase)),
                    span: *span,
                },
                Stmt::Expr { expr, span } => Stmt::Expr {
                    expr: Box::new(rewrite_contract_state_expr(expr, contract, phase)),
                    span: *span,
                },
                Stmt::While {
                    cond,
                    invariant,
                    variant,
                    body,
                    span,
                } => Stmt::While {
                    cond: Box::new(rewrite_contract_state_expr(cond, contract, phase)),
                    invariant: Box::new(rewrite_contract_state_expr(invariant, contract, phase)),
                    variant: variant.as_ref().map(|variant| {
                        Box::new(rewrite_contract_state_expr(variant, contract, phase))
                    }),
                    body: Box::new(rewrite_contract_state_block(body, contract, phase)),
                    span: *span,
                },
            })
            .collect(),
        tail: block
            .tail
            .as_ref()
            .map(|tail| Box::new(rewrite_contract_state_expr(tail, contract, phase))),
        span: block.span,
    }
}

#[allow(dead_code)]
fn collect_state_writes(expr: &Expr, writes: &mut HashSet<String>) {
    match expr {
        Expr::Call { callee, args, .. } => {
            if let Some(field) = callee.strip_prefix("__clg_state_write$") {
                writes.insert(field.to_string());
            }
            for arg in args {
                collect_state_writes(arg, writes);
            }
        }
        Expr::ArrayLit { elems, .. } | Expr::TupleLit { elems, .. } => {
            for elem in elems {
                collect_state_writes(elem, writes);
            }
        }
        Expr::StructLit { fields, .. } => {
            for field in fields {
                collect_state_writes(&field.expr, writes);
            }
        }
        Expr::FieldAccess { base, .. }
        | Expr::Unary { expr: base, .. }
        | Expr::Return { expr: base, .. }
        | Expr::Try { expr: base, .. } => collect_state_writes(base, writes),
        Expr::Index { base, index, .. }
        | Expr::Bin {
            lhs: base,
            rhs: index,
            ..
        } => {
            collect_state_writes(base, writes);
            collect_state_writes(index, writes);
        }
        Expr::Block { block } => {
            for stmt in &block.statements {
                match stmt {
                    Stmt::Let { expr, .. } | Stmt::Expr { expr, .. } => {
                        collect_state_writes(expr, writes)
                    }
                    Stmt::While {
                        cond,
                        invariant,
                        variant,
                        body,
                        ..
                    } => {
                        collect_state_writes(cond, writes);
                        collect_state_writes(invariant, writes);
                        if let Some(variant) = variant {
                            collect_state_writes(variant, writes);
                        }
                        collect_state_writes(
                            &Expr::Block {
                                block: body.clone(),
                            },
                            writes,
                        );
                    }
                }
            }
            if let Some(tail) = &block.tail {
                collect_state_writes(tail, writes);
            }
        }
        Expr::If {
            cond,
            then_br,
            else_br,
            ..
        } => {
            collect_state_writes(cond, writes);
            collect_state_writes(then_br, writes);
            collect_state_writes(else_br, writes);
        }
        Expr::Match {
            scrutinee, arms, ..
        } => {
            collect_state_writes(scrutinee, writes);
            for arm in arms {
                collect_state_writes(&arm.expr, writes);
            }
        }
        Expr::Lambda { body, .. } => collect_state_writes(body, writes),
        Expr::Int(_, _) | Expr::Bool(_, _) | Expr::String(_, _) | Expr::Var(_, _) => {}
    }
}

fn state_transition_model(func: &Func, contract: &ContractDecl) -> StateTransitionModel {
    state_transition_model_for_expr(&func.body, contract, Some(&func.ret))
}

fn state_transition_model_for_expr(
    body: &Expr,
    contract: &ContractDecl,
    result_type: Option<&Type>,
) -> StateTransitionModel {
    let mut states: HashMap<String, Expr> = contract
        .fields
        .iter()
        .map(|field| {
            let field_id = state_field_id(contract, &field.name);
            (
                field.name.clone(),
                Expr::Var(smt_state_symbol("pre", &field_id), field.span),
            )
        })
        .collect();
    let mut locals = HashMap::new();
    let mut writes = BTreeSet::new();
    let mut fully_modelled = contract
        .fields
        .iter()
        .all(|field| state_solver_type_supported(&field.ty));
    let result_expr = model_transition_expr(
        body,
        &mut states,
        &mut locals,
        &mut writes,
        &mut fully_modelled,
    );

    let declarations = state_symbol_declarations(contract);
    let mut relations = Vec::new();
    let mut rendered_fields = Vec::new();
    let mut encoder = SmtEncoder::default();
    for field in &contract.fields {
        let field_id = state_field_id(contract, &field.name);
        let pre = smt_state_symbol("pre", &field_id);
        let post = smt_state_symbol("post", &field_id);
        let value = states
            .get(&field.name)
            .expect("state model initializes every declared field");
        let value_smt = encoder.encode(value);
        relations.push(format!("(= {post} {value_smt})"));
        rendered_fields.push(format!("{}: pre={}, post={}", field.name, pre, post));
    }
    if let Some(result_type) = result_type {
        if state_solver_type_supported(result_type) {
            let sort = smt_sort_for_type(result_type, &HashMap::new());
            relations.push(format!("(= result {})", encoder.encode(&result_expr)));
            let declarations = if declarations.is_empty() {
                format!("(declare-const result {sort})")
            } else {
                format!("{declarations}\n(declare-const result {sort})")
            };
            return StateTransitionModel {
                declarations,
                relation_smt: conjoin_smt(relations),
                rendered_fields,
                writes,
                fully_modelled,
            };
        }
        fully_modelled = false;
    }
    StateTransitionModel {
        declarations,
        relation_smt: conjoin_smt(relations),
        rendered_fields,
        writes,
        fully_modelled,
    }
}

fn state_solver_type_supported(ty: &Type) -> bool {
    matches!(
        ty,
        Type::Bool
            | Type::Int
            | Type::U8
            | Type::U64
            | Type::U128
            | Type::U256
            | Type::String
            | Type::Bytes
    )
}

fn conjoin_smt(parts: Vec<String>) -> String {
    match parts.len() {
        0 => "true".to_string(),
        1 => parts.into_iter().next().expect("one part"),
        _ => format!("(and {})", parts.join(" ")),
    }
}

fn transition_premise(pre_smt: &str, model: Option<&StateTransitionModel>) -> String {
    match model {
        Some(model) => format!("(and {pre_smt} {})", model.relation_smt),
        None => pre_smt.to_string(),
    }
}

fn state_model_assumptions(
    model: &StateTransitionModel,
    contract: &ContractDecl,
) -> Vec<AssumptionBoundary> {
    if model.fully_modelled {
        Vec::new()
    } else {
        vec![AssumptionBoundary {
            id: ASSUMPTION_STATE_TRANSITION_ID,
            category: AssumptionCategory::Primitive,
            status: ASSUMPTION_STATUS_ASSUMED,
            message: ASSUMPTION_STATE_TRANSITION_MESSAGE,
            symbols: contract
                .fields
                .iter()
                .map(|field| state_field_id(contract, &field.name))
                .collect(),
        }]
    }
}

fn model_transition_expr(
    expr: &Expr,
    states: &mut HashMap<String, Expr>,
    locals: &mut HashMap<String, Expr>,
    writes: &mut BTreeSet<String>,
    fully_modelled: &mut bool,
) -> Expr {
    match expr {
        Expr::Var(name, _) => locals.get(name).cloned().unwrap_or_else(|| expr.clone()),
        Expr::FieldAccess { base, field, .. } if matches!(base.as_ref(), Expr::Var(name, _) if name == "state") => {
            states.get(field).cloned().unwrap_or_else(|| {
                *fully_modelled = false;
                expr.clone()
            })
        }
        Expr::Call {
            callee,
            type_args,
            args,
            span,
        } if callee.starts_with("__clg_state_write$") => {
            let values = args
                .iter()
                .map(|arg| model_transition_expr(arg, states, locals, writes, fully_modelled))
                .collect::<Vec<_>>();
            let field = callee.trim_start_matches("__clg_state_write$");
            let Some(value) = values.first() else {
                *fully_modelled = false;
                return Expr::Call {
                    callee: callee.clone(),
                    type_args: type_args.clone(),
                    args: values,
                    span: *span,
                };
            };
            if states.contains_key(field) {
                states.insert(field.to_string(), value.clone());
                writes.insert(field.to_string());
                value.clone()
            } else {
                *fully_modelled = false;
                expr.clone()
            }
        }
        Expr::Call {
            callee,
            type_args: _,
            args,
            span,
        } if callee.starts_with("__clg_event_emit$") => {
            for arg in args {
                let _ = model_transition_expr(arg, states, locals, writes, fully_modelled);
            }
            Expr::Bool(true, *span)
        }
        Expr::Call { callee, .. } if callee.starts_with("__clg_external_call$") => {
            *fully_modelled = false;
            expr.clone()
        }
        Expr::Call {
            callee,
            type_args,
            args,
            span,
        } => Expr::Call {
            callee: callee.clone(),
            type_args: type_args.clone(),
            args: args
                .iter()
                .map(|arg| model_transition_expr(arg, states, locals, writes, fully_modelled))
                .collect(),
            span: *span,
        },
        Expr::Block { block } => {
            let saved_locals = locals.clone();
            for stmt in &block.statements {
                match stmt {
                    Stmt::Let { name, expr, .. } => {
                        let value =
                            model_transition_expr(expr, states, locals, writes, fully_modelled);
                        locals.insert(name.clone(), value);
                    }
                    Stmt::Expr { expr, .. } => {
                        let _ = model_transition_expr(expr, states, locals, writes, fully_modelled);
                    }
                    Stmt::While { .. } => *fully_modelled = false,
                }
            }
            let value = block
                .tail
                .as_ref()
                .map(|tail| model_transition_expr(tail, states, locals, writes, fully_modelled))
                .unwrap_or_else(|| Expr::Int(0, block.span));
            *locals = saved_locals;
            value
        }
        Expr::ArrayLit { elems, span } => Expr::ArrayLit {
            elems: elems
                .iter()
                .map(|item| model_transition_expr(item, states, locals, writes, fully_modelled))
                .collect(),
            span: *span,
        },
        Expr::TupleLit { elems, span } => Expr::TupleLit {
            elems: elems
                .iter()
                .map(|item| model_transition_expr(item, states, locals, writes, fully_modelled))
                .collect(),
            span: *span,
        },
        Expr::StructLit { name, fields, span } => Expr::StructLit {
            name: name.clone(),
            fields: fields
                .iter()
                .map(|field| clg_ast::StructFieldInit {
                    name: field.name.clone(),
                    expr: model_transition_expr(
                        &field.expr,
                        states,
                        locals,
                        writes,
                        fully_modelled,
                    ),
                    span: field.span,
                })
                .collect(),
            span: *span,
        },
        Expr::FieldAccess { base, field, span } => Expr::FieldAccess {
            base: Box::new(model_transition_expr(
                base,
                states,
                locals,
                writes,
                fully_modelled,
            )),
            field: field.clone(),
            span: *span,
        },
        Expr::Bin { op, lhs, rhs, span } => Expr::Bin {
            op: *op,
            lhs: Box::new(model_transition_expr(
                lhs,
                states,
                locals,
                writes,
                fully_modelled,
            )),
            rhs: Box::new(model_transition_expr(
                rhs,
                states,
                locals,
                writes,
                fully_modelled,
            )),
            span: *span,
        },
        Expr::Return { expr, span } => Expr::Return {
            expr: Box::new(model_transition_expr(
                expr,
                states,
                locals,
                writes,
                fully_modelled,
            )),
            span: *span,
        },
        Expr::Unary { op, expr, span } => Expr::Unary {
            op: *op,
            expr: Box::new(model_transition_expr(
                expr,
                states,
                locals,
                writes,
                fully_modelled,
            )),
            span: *span,
        },
        Expr::Index { base, index, span } => Expr::Index {
            base: Box::new(model_transition_expr(
                base,
                states,
                locals,
                writes,
                fully_modelled,
            )),
            index: Box::new(model_transition_expr(
                index,
                states,
                locals,
                writes,
                fully_modelled,
            )),
            span: *span,
        },
        Expr::Try { expr, span } => Expr::Try {
            expr: Box::new(model_transition_expr(
                expr,
                states,
                locals,
                writes,
                fully_modelled,
            )),
            span: *span,
        },
        Expr::If { .. } | Expr::Match { .. } | Expr::Lambda { .. } => {
            *fully_modelled = false;
            expr.clone()
        }
        Expr::Int(_, _) | Expr::Bool(_, _) | Expr::String(_, _) => expr.clone(),
    }
}

#[derive(Clone)]
struct StateTransitionModel {
    declarations: String,
    relation_smt: String,
    rendered_fields: Vec<String>,
    writes: BTreeSet<String>,
    fully_modelled: bool,
}

fn state_transition_vc(
    func: &Func,
    contract: &ContractDecl,
    model: &StateTransitionModel,
) -> VerificationCondition {
    let encoder = SmtEncoder::default();
    let vc_smt2 = wrap_vc_with_extra(
        &encoder,
        &format!("(=> {} {})", model.relation_smt, model.relation_smt),
        &model.declarations,
    );
    VerificationCondition {
        function: func.name.clone(),
        vc_id: "state-transition:0".to_string(),
        pre: ContractExpr {
            ast: format!("state entry [{}]", model.rendered_fields.join(", ")),
            smt2: model.relation_smt.clone(),
            span: None,
        },
        post: ContractExpr {
            ast: format!(
                "state exit [{}]; writes [{}]",
                model.rendered_fields.join(", "),
                model.writes.iter().cloned().collect::<Vec<_>>().join(", ")
            ),
            smt2: model.relation_smt.clone(),
            span: None,
        },
        vc_smt2,
        status: "generated",
        counterexample: None,
        refinements: Vec::new(),
        assumptions: state_model_assumptions(model, contract),
    }
}

fn state_invariant_vcs(
    func: &Func,
    contract: &ContractDecl,
    model: &StateTransitionModel,
) -> Vec<VerificationCondition> {
    state_invariant_vcs_for(&func.name, "state-invariant", contract, model)
}

fn state_invariant_vcs_for(
    function: &str,
    vc_prefix: &str,
    contract: &ContractDecl,
    model: &StateTransitionModel,
) -> Vec<VerificationCondition> {
    contract
        .invariants
        .iter()
        .enumerate()
        .map(|(idx, invariant)| {
            let entry =
                rewrite_contract_state_expr(&invariant.expr, Some(contract), StatePhase::Pre);
            let exit =
                rewrite_contract_state_expr(&invariant.expr, Some(contract), StatePhase::Post);
            let mut encoder = SmtEncoder::default();
            let pre_smt = encoder.encode(&entry);
            let post_smt = encoder.encode(&exit);
            let vc_body = format!("(=> (and {pre_smt} {}) {post_smt})", model.relation_smt);
            let vc_smt2 = wrap_vc_with_extra(&encoder, &vc_body, &model.declarations);
            VerificationCondition {
                function: function.to_string(),
                vc_id: format!("{vc_prefix}:{idx}"),
                pre: ContractExpr {
                    ast: format!("entry invariant: {}", expr_to_source(&invariant.expr, 0)),
                    smt2: pre_smt,
                    span: Some(invariant.span),
                },
                post: ContractExpr {
                    ast: format!("exit invariant: {}", expr_to_source(&invariant.expr, 0)),
                    smt2: post_smt,
                    span: Some(invariant.span),
                },
                vc_smt2,
                status: "generated",
                counterexample: None,
                refinements: Vec::new(),
                assumptions: state_model_assumptions(model, contract),
            }
        })
        .collect()
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
