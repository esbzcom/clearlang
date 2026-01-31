use crate::builtins::builtin_sigs;
use crate::guards::{collect_mut_calls, guard_callee_for_kind, MutCall};
use clg_ast::{BinOp, Effect, Expr, MatchArm, MatchPat, Program, Span, Stmt, Type, UnaryOp};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct ContractExpr {
    pub ast: String,
    pub smt2: String,
    pub span: Option<Span>,
}

#[derive(Debug, Clone)]
pub struct ExprSnapshot {
    pub ast: String,
    pub smt2: String,
}

#[derive(Debug, Clone)]
pub enum RefinementAttachmentKind {
    Param,
    Return,
    Flow,
}

#[derive(Debug, Clone)]
pub enum RefinementAttachmentDetail {
    Param { param: String },
    Return { result: String },
    Flow(RefinementFlowDetail),
}

#[derive(Debug, Clone)]
pub struct RefinementAttachment {
    pub kind: RefinementAttachmentKind,
    pub detail: RefinementAttachmentDetail,
}

#[derive(Debug, Clone)]
pub enum RefinementFlowKind {
    Let,
    CallArg,
    MatchBinder,
}

impl RefinementFlowKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            RefinementFlowKind::Let => "let",
            RefinementFlowKind::CallArg => "call_arg",
            RefinementFlowKind::MatchBinder => "match_binder",
        }
    }
}

#[derive(Debug, Clone)]
pub struct RefinementFlowDetail {
    pub flow_kind: RefinementFlowKind,
    pub name: Option<String>,
    pub callee: Option<String>,
    pub arg_index: Option<usize>,
    pub variant: Option<String>,
    pub arm: Option<usize>,
}

impl RefinementFlowDetail {
    fn new(flow_kind: RefinementFlowKind) -> Self {
        Self {
            flow_kind,
            name: None,
            callee: None,
            arg_index: None,
            variant: None,
            arm: None,
        }
    }
}

impl RefinementAttachment {
    fn param(name: String) -> Self {
        Self {
            kind: RefinementAttachmentKind::Param,
            detail: RefinementAttachmentDetail::Param { param: name },
        }
    }

    fn result(name: String) -> Self {
        Self {
            kind: RefinementAttachmentKind::Return,
            detail: RefinementAttachmentDetail::Return { result: name },
        }
    }

    fn flow(detail: RefinementFlowDetail) -> Self {
        Self {
            kind: RefinementAttachmentKind::Flow,
            detail: RefinementAttachmentDetail::Flow(detail),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RefinementPremise {
    pub alias: String,
    pub binder: String,
    pub substitution: ExprSnapshot,
    pub predicate: ExprSnapshot,
    pub attachment: RefinementAttachment,
}

#[derive(Debug, Clone)]
pub struct VerificationCondition {
    pub function: String,
    pub vc_id: String,
    pub pre: ContractExpr,
    pub post: ContractExpr,
    pub vc_smt2: String,
    pub status: &'static str,
    pub refinements: Vec<RefinementPremise>,
}

const U64_MAX_SMT: &str = "18446744073709551615";

#[derive(Debug)]
struct LoopObligation<'a> {
    invariant: &'a Expr,
    variant: Option<&'a Expr>,
}

#[derive(Clone)]
struct RefinementObligation {
    alias: String,
    binder: String,
    substitution: Expr,
    substitution_type: Type,
    predicate: Expr,
    attachment: RefinementAttachment,
}

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

fn snapshot_expr(expr: &Expr) -> ExprSnapshot {
    let ast = expr_to_source(expr, 0);
    let mut encoder = SmtEncoder::default();
    let smt2 = encoder.encode(expr);
    ExprSnapshot { ast, smt2 }
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

fn smt_sort_for_type(ty: &Type, aliases: &HashMap<&str, AliasView<'_>>) -> &'static str {
    match ty {
        Type::Int => "Int",
        Type::U8 => "Int",
        Type::U64 | Type::U128 | Type::U256 => "Int",
        Type::Bool => "Bool",
        Type::String => "String",
        Type::Bytes => "String",
        Type::Option(_)
        | Type::Result(_, _)
        | Type::List(_)
        | Type::Set(_)
        | Type::Map(_, _)
        | Type::Array(_, _)
        | Type::Tuple(_)
        | Type::Resource(_) => {
            if let Type::Resource(name) = ty {
                if let Some(alias) = aliases.get(name.as_str()) {
                    return smt_sort_for_type(alias.base, aliases);
                }
            }
            "Int"
        }
    }
}

fn refinement_prelude(
    obligations: &[RefinementObligation],
    extra: Option<&RefinementObligation>,
    aliases: &HashMap<&str, AliasView<'_>>,
) -> String {
    let total = obligations.len() + extra.map(|_| 1).unwrap_or(0);
    if total == 0 {
        return String::new();
    }
    let mut lines = Vec::new();
    let iter = obligations.iter().chain(extra);
    for (idx, obligation) in iter.enumerate() {
        let sort = smt_sort_for_type(&obligation.substitution_type, aliases);
        let substitution = snapshot_expr(&obligation.substitution);
        let predicate = snapshot_expr(&obligation.predicate);
        lines.push(format!(
            "; refinement ref:{} alias {} binder {}",
            idx, obligation.alias, obligation.binder
        ));
        lines.push(format!(
            "(define-fun cl.ref.premise.{}.sub () {} {})",
            idx, sort, substitution.smt2
        ));
        lines.push(format!(
            "(define-fun cl.ref.premise.{}.pred () Bool {})",
            idx, predicate.smt2
        ));
    }
    lines.join("\n")
}

fn merge_extras(parts: &[&str]) -> String {
    let mut out = Vec::new();
    for part in parts {
        let trimmed = part.trim();
        if !trimmed.is_empty() {
            out.push(trimmed);
        }
    }
    out.join("\n")
}

fn make_refinement_obligation(
    alias: &AliasView<'_>,
    replacement: &Expr,
    attachment: RefinementAttachment,
) -> Option<RefinementObligation> {
    let binder = alias.binder?;
    let predicate = substitute_binder(alias.predicate, binder, replacement);
    Some(RefinementObligation {
        alias: alias.name.to_string(),
        binder: binder.to_string(),
        substitution: replacement.clone(),
        substitution_type: alias.base.clone(),
        predicate,
        attachment,
    })
}

fn top_level_alias<'a>(
    ty: &Type,
    aliases: &'a HashMap<&'a str, AliasView<'a>>,
) -> Option<&'a AliasView<'a>> {
    if let Type::Resource(name) = ty {
        aliases.get(name.as_str())
    } else {
        None
    }
}

#[derive(Clone)]
struct AliasView<'a> {
    name: &'a str,
    base: &'a Type,
    binder: Option<&'a str>,
    predicate: &'a Expr,
    span: Span,
}

fn build_alias_map<'a>(program: &'a Program) -> HashMap<&'a str, AliasView<'a>> {
    program
        .refined_aliases
        .iter()
        .map(|a| {
            (
                a.name.as_str(),
                AliasView {
                    name: a.name.as_str(),
                    base: &a.base,
                    binder: a.binder.as_deref(),
                    predicate: &a.predicate,
                    span: a.span,
                },
            )
        })
        .collect()
}

#[derive(Clone)]
struct FnSigView<'a> {
    ret: Type,
    param_aliases: Vec<Option<&'a AliasView<'a>>>,
    ret_alias: Option<&'a AliasView<'a>>,
}

fn build_fn_sigs<'a>(
    program: &'a Program,
    aliases: &'a HashMap<&'a str, AliasView<'a>>,
) -> HashMap<String, FnSigView<'a>> {
    let mut map: HashMap<String, FnSigView<'a>> = HashMap::new();
    for func in &program.funcs {
        let param_aliases = func
            .params
            .iter()
            .map(|p| top_level_alias(&p.ty, aliases))
            .collect();
        let ret_alias = top_level_alias(&func.ret, aliases);
        map.insert(
            func.name.clone(),
            FnSigView {
                ret: func.ret.clone(),
                param_aliases,
                ret_alias,
            },
        );
    }

    for (name, params, ret, _) in builtin_sigs() {
        map.entry(name.clone()).or_insert_with(|| FnSigView {
            param_aliases: params
                .iter()
                .map(|p| top_level_alias(&p.ty, aliases))
                .collect(),
            ret: ret.clone(),
            ret_alias: top_level_alias(&ret, aliases),
        });
    }
    map
}

fn collect_refinement_obligations<'a>(
    expr: &'a Expr,
    aliases: &HashMap<&'a str, AliasView<'a>>,
    fn_sigs: &HashMap<String, FnSigView<'a>>,
    env: &mut HashMap<String, Type>,
    out: &mut Vec<RefinementObligation>,
) -> Option<Type> {
    match expr {
        Expr::Int(_, _) => Some(Type::Int),
        Expr::Bool(_, _) => Some(Type::Bool),
        Expr::String(_, _) => Some(Type::String),
        Expr::Var(name, _) => env.get(name).cloned(),
        Expr::ArrayLit { elems, .. } => {
            let mut elem_tys: Vec<Option<Type>> = Vec::with_capacity(elems.len());
            for elem in elems {
                elem_tys.push(collect_refinement_obligations(
                    elem,
                    aliases,
                    fn_sigs,
                    &mut env.clone(),
                    out,
                ));
            }
            let Some(first) = elem_tys.first().cloned().flatten() else {
                return None;
            };
            if elem_tys
                .iter()
                .skip(1)
                .all(|ty| ty.as_ref() == Some(&first))
            {
                Some(Type::Array(Box::new(first), elems.len() as u32))
            } else {
                None
            }
        }
        Expr::TupleLit { elems, .. } => {
            let mut tys: Vec<Type> = Vec::with_capacity(elems.len());
            for elem in elems {
                let Some(ty) =
                    collect_refinement_obligations(elem, aliases, fn_sigs, &mut env.clone(), out)
                else {
                    return None;
                };
                tys.push(ty);
            }
            Some(Type::Tuple(tys))
        }
        Expr::StructLit { fields, .. } => {
            for field in fields {
                collect_refinement_obligations(&field.expr, aliases, fn_sigs, &mut env.clone(), out);
            }
            None
        }
        Expr::FieldAccess { base, .. } => {
            collect_refinement_obligations(base.as_ref(), aliases, fn_sigs, &mut env.clone(), out);
            None
        }
        Expr::Index { base, index, .. } => {
            let base_ty = collect_refinement_obligations(
                base.as_ref(),
                aliases,
                fn_sigs,
                &mut env.clone(),
                out,
            );
            collect_refinement_obligations(index.as_ref(), aliases, fn_sigs, &mut env.clone(), out);
            match base_ty {
                Some(Type::Array(inner, _)) => Some(*inner),
                Some(Type::Tuple(elems)) => {
                    if let Expr::Int(idx, _) = index.as_ref() {
                        let idx = *idx as usize;
                        elems.get(idx).cloned()
                    } else {
                        None
                    }
                }
                _ => None,
            }
        }
        Expr::Unary { expr, .. } => {
            collect_refinement_obligations(expr.as_ref(), aliases, fn_sigs, &mut env.clone(), out);
            Some(Type::Bool)
        }
        Expr::Bin { op, lhs, rhs, .. } => {
            let lt = collect_refinement_obligations(
                lhs.as_ref(),
                aliases,
                fn_sigs,
                &mut env.clone(),
                out,
            );
            let rt = collect_refinement_obligations(
                rhs.as_ref(),
                aliases,
                fn_sigs,
                &mut env.clone(),
                out,
            );
            let has_u64 = matches!(lt, Some(Type::U64)) || matches!(rt, Some(Type::U64));
            let res_ty = match op {
                BinOp::Add
                | BinOp::Sub
                | BinOp::Mul
                | BinOp::Div
                | BinOp::Shl
                | BinOp::Shr
                | BinOp::BitAnd
                | BinOp::BitOr
                | BinOp::BitXor => {
                    if has_u64 {
                        Type::U64
                    } else {
                        Type::Int
                    }
                }
                BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge | BinOp::Eq | BinOp::Neq => {
                    Type::Bool
                }
                BinOp::And | BinOp::Or => Type::Bool,
            };
            Some(res_ty)
        }
        Expr::Block { block } => {
            collect_block_refinements(block.as_ref(), aliases, fn_sigs, env, out)
        }
        Expr::If {
            cond,
            then_br,
            else_br,
            ..
        } => {
            collect_refinement_obligations(cond.as_ref(), aliases, fn_sigs, &mut env.clone(), out);
            let mut then_env = env.clone();
            let then_ty = collect_refinement_obligations(
                then_br.as_ref(),
                aliases,
                fn_sigs,
                &mut then_env,
                out,
            );
            let mut else_env = env.clone();
            let else_ty = collect_refinement_obligations(
                else_br.as_ref(),
                aliases,
                fn_sigs,
                &mut else_env,
                out,
            );
            merge_types(then_ty, else_ty)
        }
        Expr::Match {
            scrutinee, arms, ..
        } => {
            let scrut_ty = collect_refinement_obligations(
                scrutinee.as_ref(),
                aliases,
                fn_sigs,
                &mut env.clone(),
                out,
            );
            let mut arm_tys: Vec<Option<Type>> = Vec::new();
            if let Some(Type::Option(inner_ty)) = scrut_ty.clone() {
                for (arm_idx, arm) in arms.iter().enumerate() {
                    match &arm.pat {
                        MatchPat::Some(name) => {
                            let mut arm_env = env.clone();
                            arm_env.insert(name.clone(), (*inner_ty).clone());
                            if let Some(alias) = top_level_alias(inner_ty.as_ref(), aliases) {
                                let mut detail =
                                    RefinementFlowDetail::new(RefinementFlowKind::MatchBinder);
                                detail.name = Some(name.clone());
                                detail.variant = Some("Some".to_string());
                                detail.arm = Some(arm_idx);
                                let attachment = RefinementAttachment::flow(detail);
                                if let Some(obligation) = make_refinement_obligation(
                                    alias,
                                    &Expr::Var(name.clone(), alias.span),
                                    attachment,
                                ) {
                                    out.push(obligation);
                                }
                            }
                            let arm_ty = collect_refinement_obligations(
                                &arm.expr,
                                aliases,
                                fn_sigs,
                                &mut arm_env,
                                out,
                            );
                            arm_tys.push(arm_ty);
                        }
                        MatchPat::None => {
                            let arm_ty = collect_refinement_obligations(
                                &arm.expr,
                                aliases,
                                fn_sigs,
                                &mut env.clone(),
                                out,
                            );
                            arm_tys.push(arm_ty);
                        }
                        _ => {
                            let arm_ty = collect_refinement_obligations(
                                &arm.expr,
                                aliases,
                                fn_sigs,
                                &mut env.clone(),
                                out,
                            );
                            arm_tys.push(arm_ty);
                        }
                    }
                }
            } else if let Some(Type::Result(ok_ty, err_ty)) = scrut_ty.clone() {
                for (arm_idx, arm) in arms.iter().enumerate() {
                    match &arm.pat {
                        MatchPat::Ok(name) => {
                            let mut arm_env = env.clone();
                            arm_env.insert(name.clone(), (*ok_ty).clone());
                            if let Some(alias) = top_level_alias(ok_ty.as_ref(), aliases) {
                                let mut detail =
                                    RefinementFlowDetail::new(RefinementFlowKind::MatchBinder);
                                detail.name = Some(name.clone());
                                detail.variant = Some("Ok".to_string());
                                detail.arm = Some(arm_idx);
                                let attachment = RefinementAttachment::flow(detail);
                                if let Some(obligation) = make_refinement_obligation(
                                    alias,
                                    &Expr::Var(name.clone(), alias.span),
                                    attachment,
                                ) {
                                    out.push(obligation);
                                }
                            }
                            let arm_ty = collect_refinement_obligations(
                                &arm.expr,
                                aliases,
                                fn_sigs,
                                &mut arm_env,
                                out,
                            );
                            arm_tys.push(arm_ty);
                        }
                        MatchPat::Err(name) => {
                            let mut arm_env = env.clone();
                            arm_env.insert(name.clone(), (*err_ty).clone());
                            if let Some(alias) = top_level_alias(err_ty.as_ref(), aliases) {
                                let mut detail =
                                    RefinementFlowDetail::new(RefinementFlowKind::MatchBinder);
                                detail.name = Some(name.clone());
                                detail.variant = Some("Err".to_string());
                                detail.arm = Some(arm_idx);
                                let attachment = RefinementAttachment::flow(detail);
                                if let Some(obligation) = make_refinement_obligation(
                                    alias,
                                    &Expr::Var(name.clone(), alias.span),
                                    attachment,
                                ) {
                                    out.push(obligation);
                                }
                            }
                            let arm_ty = collect_refinement_obligations(
                                &arm.expr,
                                aliases,
                                fn_sigs,
                                &mut arm_env,
                                out,
                            );
                            arm_tys.push(arm_ty);
                        }
                        _ => {
                            let arm_ty = collect_refinement_obligations(
                                &arm.expr,
                                aliases,
                                fn_sigs,
                                &mut env.clone(),
                                out,
                            );
                            arm_tys.push(arm_ty);
                        }
                    }
                }
            } else {
                for arm in arms {
                    let arm_ty = collect_refinement_obligations(
                        &arm.expr,
                        aliases,
                        fn_sigs,
                        &mut env.clone(),
                        out,
                    );
                    arm_tys.push(arm_ty);
                }
            }
            merge_type_list(&arm_tys)
        }
        Expr::Call { callee, args, .. } => {
            if callee.as_str() == "U64" && args.len() == 1 {
                collect_refinement_obligations(&args[0], aliases, fn_sigs, &mut env.clone(), out);
                return Some(Type::U64);
            }
            if let Some(sig) = fn_sigs.get(callee.as_str()) {
                for (idx, (arg, alias_opt)) in args.iter().zip(sig.param_aliases.iter()).enumerate()
                {
                    collect_refinement_obligations(arg, aliases, fn_sigs, &mut env.clone(), out);
                    if let Some(alias) = alias_opt {
                        let mut detail = RefinementFlowDetail::new(RefinementFlowKind::CallArg);
                        detail.callee = Some(callee.clone());
                        detail.arg_index = Some(idx);
                        if let Expr::Var(name, _) = arg {
                            detail.name = Some(name.clone());
                        }
                        let attachment = RefinementAttachment::flow(detail);
                        if let Some(obligation) = make_refinement_obligation(alias, arg, attachment)
                        {
                            out.push(obligation);
                        }
                    }
                }
                Some(sig.ret.clone())
            } else {
                match callee.as_str() {
                    "Some" if args.len() == 1 => {
                        let inner_ty = collect_refinement_obligations(
                            &args[0],
                            aliases,
                            fn_sigs,
                            &mut env.clone(),
                            out,
                        );
                        inner_ty.map(|ty| Type::Option(Box::new(ty)))
                    }
                    "Ok" if args.len() == 1 => {
                        let ok_ty = collect_refinement_obligations(
                            &args[0],
                            aliases,
                            fn_sigs,
                            &mut env.clone(),
                            out,
                        );
                        ok_ty.map(|ty| Type::Result(Box::new(ty), Box::new(Type::Int)))
                    }
                    "Err" if args.len() == 1 => {
                        let err_ty = collect_refinement_obligations(
                            &args[0],
                            aliases,
                            fn_sigs,
                            &mut env.clone(),
                            out,
                        );
                        err_ty.map(|ty| Type::Result(Box::new(Type::Int), Box::new(ty)))
                    }
                    _ => {
                        for arg in args {
                            collect_refinement_obligations(
                                arg,
                                aliases,
                                fn_sigs,
                                &mut env.clone(),
                                out,
                            );
                        }
                        None
                    }
                }
            }
        }
        Expr::Return { expr, .. } => {
            collect_refinement_obligations(expr.as_ref(), aliases, fn_sigs, &mut env.clone(), out)
        }
        Expr::Try { expr, .. } => {
            let inner = collect_refinement_obligations(expr.as_ref(), aliases, fn_sigs, env, out)?;
            match inner {
                Type::Option(t) => Some(*t),
                Type::Result(ok, _) => Some(*ok),
                other => Some(other),
            }
        }
    }
}

fn collect_block_refinements<'a>(
    block: &'a clg_ast::Block,
    aliases: &HashMap<&'a str, AliasView<'a>>,
    fn_sigs: &HashMap<String, FnSigView<'a>>,
    outer_env: &mut HashMap<String, Type>,
    out: &mut Vec<RefinementObligation>,
) -> Option<Type> {
    let mut env = outer_env.clone();
    for stmt in &block.statements {
        match stmt {
            Stmt::Let { name, expr, .. } => {
                let expr_ty = collect_refinement_obligations(
                    expr.as_ref(),
                    aliases,
                    fn_sigs,
                    &mut env.clone(),
                    out,
                );
                if let Some(ty) = expr_ty {
                    if let Some(alias) = top_level_alias(&ty, aliases) {
                        let mut detail = RefinementFlowDetail::new(RefinementFlowKind::Let);
                        detail.name = Some(name.clone());
                        let attachment = RefinementAttachment::flow(detail);
                        if let Some(obligation) = make_refinement_obligation(
                            alias,
                            &Expr::Var(name.clone(), alias.span),
                            attachment,
                        ) {
                            out.push(obligation);
                        }
                    }
                    env.insert(name.clone(), ty);
                }
            }
            Stmt::Expr { expr, .. } => {
                collect_refinement_obligations(
                    expr.as_ref(),
                    aliases,
                    fn_sigs,
                    &mut env.clone(),
                    out,
                );
            }
            Stmt::While {
                cond,
                invariant,
                variant,
                body,
                ..
            } => {
                collect_refinement_obligations(
                    cond.as_ref(),
                    aliases,
                    fn_sigs,
                    &mut env.clone(),
                    out,
                );
                collect_refinement_obligations(
                    invariant.as_ref(),
                    aliases,
                    fn_sigs,
                    &mut env.clone(),
                    out,
                );
                if let Some(v) = variant {
                    collect_refinement_obligations(
                        v.as_ref(),
                        aliases,
                        fn_sigs,
                        &mut env.clone(),
                        out,
                    );
                }
                collect_block_refinements(body.as_ref(), aliases, fn_sigs, &mut env.clone(), out);
            }
        }
    }
    if let Some(tail) = &block.tail {
        collect_refinement_obligations(tail.as_ref(), aliases, fn_sigs, &mut env, out)
    } else {
        None
    }
}

fn merge_types(lhs: Option<Type>, rhs: Option<Type>) -> Option<Type> {
    match (lhs, rhs) {
        (Some(l), Some(r)) if l == r => Some(l),
        (Some(l), None) => Some(l),
        (None, Some(r)) => Some(r),
        _ => None,
    }
}

fn merge_type_list(types: &[Option<Type>]) -> Option<Type> {
    let mut iter = types.iter().filter_map(|t| t.clone());
    let first = iter.next()?;
    if iter.all(|t| t == first) {
        Some(first)
    } else {
        None
    }
}

fn substitute_binder(expr: &Expr, binder: &str, replacement: &Expr) -> Expr {
    match expr {
        Expr::Var(name, _) if name == binder => replacement.clone(),
        Expr::Var(_, _) | Expr::Int(_, _) | Expr::Bool(_, _) | Expr::String(_, _) => expr.clone(),
        Expr::ArrayLit { elems, span } => Expr::ArrayLit {
            elems: elems
                .iter()
                .map(|e| substitute_binder(e, binder, replacement))
                .collect(),
            span: *span,
        },
        Expr::TupleLit { elems, span } => Expr::TupleLit {
            elems: elems
                .iter()
                .map(|e| substitute_binder(e, binder, replacement))
                .collect(),
            span: *span,
        },
        Expr::StructLit { name, fields, span } => Expr::StructLit {
            name: name.clone(),
            fields: fields
                .iter()
                .map(|field| clg_ast::StructFieldInit {
                    name: field.name.clone(),
                    expr: substitute_binder(&field.expr, binder, replacement),
                    span: field.span,
                })
                .collect(),
            span: *span,
        },
        Expr::FieldAccess { base, field, span } => Expr::FieldAccess {
            base: Box::new(substitute_binder(base, binder, replacement)),
            field: field.clone(),
            span: *span,
        },
        Expr::Index { base, index, span } => Expr::Index {
            base: Box::new(substitute_binder(base, binder, replacement)),
            index: Box::new(substitute_binder(index, binder, replacement)),
            span: *span,
        },
        Expr::Bin { op, lhs, rhs, span } => Expr::Bin {
            op: *op,
            lhs: Box::new(substitute_binder(lhs, binder, replacement)),
            rhs: Box::new(substitute_binder(rhs, binder, replacement)),
            span: *span,
        },
        Expr::Unary { op, expr, span } => Expr::Unary {
            op: *op,
            expr: Box::new(substitute_binder(expr, binder, replacement)),
            span: *span,
        },
        Expr::Call { callee, args, span } => Expr::Call {
            callee: callee.clone(),
            args: args
                .iter()
                .map(|a| substitute_binder(a, binder, replacement))
                .collect(),
            span: *span,
        },
        Expr::Return { expr, span } => Expr::Return {
            expr: Box::new(substitute_binder(expr, binder, replacement)),
            span: *span,
        },
        Expr::Try { expr, span } => Expr::Try {
            expr: Box::new(substitute_binder(expr, binder, replacement)),
            span: *span,
        },
        Expr::Block { block } => Expr::Block {
            block: Box::new(substitute_block_binder(block, binder, replacement)),
        },
        Expr::Match { .. } | Expr::If { .. } => expr.clone(),
    }
}

fn substitute_block_binder(
    block: &clg_ast::Block,
    binder: &str,
    replacement: &Expr,
) -> clg_ast::Block {
    let mut new_block = block.clone();
    new_block.statements = block
        .statements
        .iter()
        .map(|stmt| match stmt {
            Stmt::Let { name, expr, span } => Stmt::Let {
                name: name.clone(),
                expr: Box::new(substitute_binder(expr, binder, replacement)),
                span: *span,
            },
            Stmt::Expr { expr, span } => Stmt::Expr {
                expr: Box::new(substitute_binder(expr, binder, replacement)),
                span: *span,
            },
            Stmt::While {
                cond,
                invariant,
                variant,
                body,
                span,
            } => Stmt::While {
                cond: Box::new(substitute_binder(cond, binder, replacement)),
                invariant: Box::new(substitute_binder(invariant, binder, replacement)),
                variant: variant
                    .as_ref()
                    .map(|v| Box::new(substitute_binder(v, binder, replacement))),
                body: Box::new(substitute_block_binder(body, binder, replacement)),
                span: *span,
            },
        })
        .collect();
    new_block.tail = block
        .tail
        .as_ref()
        .map(|expr| Box::new(substitute_binder(expr, binder, replacement)));
    new_block
}

fn fold_conjunction(mut exprs: Vec<Expr>) -> Expr {
    let mut iter = exprs.drain(..);
    let first = iter.next().expect("exprs not empty");
    iter.fold(first, |lhs, rhs| {
        let span = merge_span(&lhs, &rhs);
        Expr::Bin {
            op: BinOp::And,
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
            span,
        }
    })
}

fn substitute_result(expr: &Expr, replacement: &Expr) -> Expr {
    match expr {
        Expr::Var(name, _) if name == "result" => replacement.clone(),
        Expr::Var(_, _) | Expr::Int(_, _) | Expr::Bool(_, _) | Expr::String(_, _) => expr.clone(),
        Expr::ArrayLit { elems, span } => Expr::ArrayLit {
            elems: elems
                .iter()
                .map(|e| substitute_result(e, replacement))
                .collect(),
            span: *span,
        },
        Expr::TupleLit { elems, span } => Expr::TupleLit {
            elems: elems
                .iter()
                .map(|e| substitute_result(e, replacement))
                .collect(),
            span: *span,
        },
        Expr::StructLit { name, fields, span } => Expr::StructLit {
            name: name.clone(),
            fields: fields
                .iter()
                .map(|field| clg_ast::StructFieldInit {
                    name: field.name.clone(),
                    expr: substitute_result(&field.expr, replacement),
                    span: field.span,
                })
                .collect(),
            span: *span,
        },
        Expr::FieldAccess { base, field, span } => Expr::FieldAccess {
            base: Box::new(substitute_result(base, replacement)),
            field: field.clone(),
            span: *span,
        },
        Expr::Index { base, index, span } => Expr::Index {
            base: Box::new(substitute_result(base, replacement)),
            index: Box::new(substitute_result(index, replacement)),
            span: *span,
        },
        Expr::Bin { op, lhs, rhs, span } => Expr::Bin {
            op: *op,
            lhs: Box::new(substitute_result(lhs, replacement)),
            rhs: Box::new(substitute_result(rhs, replacement)),
            span: *span,
        },
        Expr::Unary { op, expr, span } => Expr::Unary {
            op: *op,
            expr: Box::new(substitute_result(expr, replacement)),
            span: *span,
        },
        Expr::Call { callee, args, span } => Expr::Call {
            callee: callee.clone(),
            args: args
                .iter()
                .map(|a| substitute_result(a, replacement))
                .collect(),
            span: *span,
        },
        Expr::Return { expr, span } => Expr::Return {
            expr: Box::new(substitute_result(expr, replacement)),
            span: *span,
        },
        Expr::Try { expr, span } => Expr::Try {
            expr: Box::new(substitute_result(expr, replacement)),
            span: *span,
        },
        Expr::Block { block } => Expr::Block {
            block: Box::new(substitute_block(block, replacement)),
        },
        Expr::Match { .. } | Expr::If { .. } => expr.clone(),
    }
}

fn substitute_block(block: &clg_ast::Block, replacement: &Expr) -> clg_ast::Block {
    let mut new_block = block.clone();
    new_block.statements = block
        .statements
        .iter()
        .map(|stmt| match stmt {
            Stmt::Let { name, expr, span } => Stmt::Let {
                name: name.clone(),
                expr: Box::new(substitute_result(expr, replacement)),
                span: *span,
            },
            Stmt::Expr { expr, span } => Stmt::Expr {
                expr: Box::new(substitute_result(expr, replacement)),
                span: *span,
            },
            Stmt::While {
                cond,
                invariant,
                variant,
                body,
                span,
            } => Stmt::While {
                cond: Box::new(substitute_result(cond, replacement)),
                invariant: Box::new(substitute_result(invariant, replacement)),
                variant: variant
                    .as_ref()
                    .map(|v| Box::new(substitute_result(v, replacement))),
                body: Box::new(substitute_block(body, replacement)),
                span: *span,
            },
        })
        .collect();
    new_block.tail = block
        .tail
        .as_ref()
        .map(|expr| Box::new(substitute_result(expr, replacement)));
    new_block
}
fn expr_to_source(expr: &Expr, parent_prec: u8) -> String {
    match expr {
        Expr::Int(n, _) => n.to_string(),
        Expr::Bool(b, _) => b.to_string(),
        Expr::String(s, _) => format!("\"{}\"", s),
        Expr::Var(name, _) => name.clone(),
        Expr::ArrayLit { elems, .. } => {
            let rendered = elems
                .iter()
                .map(|e| expr_to_source(e, 0))
                .collect::<Vec<_>>()
                .join(", ");
            format!("[{}]", rendered)
        }
        Expr::TupleLit { elems, .. } => {
            let rendered = elems
                .iter()
                .map(|e| expr_to_source(e, 0))
                .collect::<Vec<_>>()
                .join(", ");
            format!("({})", rendered)
        }
        Expr::StructLit { name, fields, .. } => {
            let rendered = fields
                .iter()
                .map(|f| format!("{}: {}", f.name, expr_to_source(&f.expr, 0)))
                .collect::<Vec<_>>()
                .join(", ");
            format!("{} {{ {} }}", name, rendered)
        }
        Expr::FieldAccess { base, field, .. } => {
            let base_str = expr_to_source(base, precedence_unary());
            format!("{}.{}", base_str, field)
        }
        Expr::Index { base, index, .. } => {
            let base_str = expr_to_source(base, precedence_unary());
            let index_str = expr_to_source(index, 0);
            format!("{}[{}]", base_str, index_str)
        }
        Expr::Unary { op, expr, .. } => match op {
            UnaryOp::Not => {
                let inner = expr_to_source(expr, precedence_unary());
                format!("!{}", inner)
            }
        },
        Expr::Bin { op, lhs, rhs, .. } => {
            let prec = precedence_bin(*op);
            let left = expr_to_source(lhs, prec);
            let right = expr_to_source(rhs, prec + 1);
            let infix = match op {
                BinOp::Add => "+",
                BinOp::Sub => "-",
                BinOp::Mul => "*",
                BinOp::Div => "/",
                BinOp::Shl => "<<",
                BinOp::Shr => ">>",
                BinOp::BitAnd => "&",
                BinOp::BitXor => "^",
                BinOp::BitOr => "|",
                BinOp::Lt => "<",
                BinOp::Le => "<=",
                BinOp::Gt => ">",
                BinOp::Ge => ">=",
                BinOp::Eq => "==",
                BinOp::Neq => "!=",
                BinOp::And => "&&",
                BinOp::Or => "||",
            };
            let s = format!("{} {} {}", left, infix, right);
            if prec < parent_prec {
                format!("({})", s)
            } else {
                s
            }
        }
        Expr::Call { callee, args, .. } => {
            let params: Vec<String> = args.iter().map(|a| expr_to_source(a, 0)).collect();
            format!("{}({})", callee, params.join(", "))
        }
        Expr::Return { expr, .. } => format!("return {}", expr_to_source(expr, 0)),
        Expr::Try { expr, .. } => {
            let inner = expr_to_source(expr, precedence_unary());
            let rendered = format!("{}?", inner);
            if precedence_unary() < parent_prec {
                format!("({})", rendered)
            } else {
                rendered
            }
        }
        Expr::Match {
            scrutinee, arms, ..
        } => {
            if arms.len() == 2 {
                let scrutinee_src = expr_to_source(scrutinee, 0);
                let first = match_arm_to_source(&arms[0]);
                let second = match_arm_to_source(&arms[1]);
                format!(
                    "match {} {{ {} => {}, {} => {} }}",
                    scrutinee_src, first.pat, first.body, second.pat, second.body
                )
            } else {
                format!("{:?}", expr)
            }
        }
        Expr::If {
            cond,
            then_br,
            else_br,
            ..
        } => {
            let cond_src = expr_to_source(cond, 0);
            let then_src = expr_to_source(then_br, 0);
            let else_src = expr_to_source(else_br, 0);
            format!("if {} {{ {} }} else {{ {} }}", cond_src, then_src, else_src)
        }
        Expr::Block { block } => {
            let mut parts: Vec<String> = Vec::new();
            for stmt in &block.statements {
                let rendered = match stmt {
                    Stmt::Let { name, expr, .. } => {
                        format!("let {} = {};", name, expr_to_source(expr.as_ref(), 0))
                    }
                    Stmt::Expr { expr, .. } => {
                        format!("{};", expr_to_source(expr.as_ref(), 0))
                    }
                    Stmt::While {
                        cond,
                        invariant,
                        variant,
                        body,
                        ..
                    } => {
                        let cond_src = expr_to_source(cond.as_ref(), 0);
                        let inv_src = expr_to_source(invariant.as_ref(), 0);
                        let var_src = variant
                            .as_ref()
                            .map(|v| format!(" variant {{ {} }}", expr_to_source(v.as_ref(), 0)))
                            .unwrap_or_default();
                        let body_src = expr_to_source(
                            &Expr::Block {
                                block: body.clone(),
                            },
                            0,
                        );
                        format!(
                            "while {} invariant {{ {} }}{} {}",
                            cond_src, inv_src, var_src, body_src
                        )
                    }
                };
                parts.push(rendered);
            }
            if let Some(tail) = &block.tail {
                parts.push(expr_to_source(tail.as_ref(), 0));
            }
            format!("{{ {} }}", parts.join(" "))
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum SmtHelper {
    VariantAccessors,
    OptionCtor,
    ResultCtor,
    BitwiseOps,
}

#[derive(Default)]
struct SmtEncoder {
    helpers: HashSet<SmtHelper>,
    fresh: usize,
    builtin_calls: HashSet<String>,
}

impl SmtEncoder {
    fn encode(&mut self, expr: &Expr) -> String {
        self.encode_inner(expr)
    }

    fn encode_inner(&mut self, expr: &Expr) -> String {
        match expr {
            Expr::Int(n, _) => n.to_string(),
            Expr::Bool(b, _) => b.to_string(),
            Expr::String(s, _) => format!("\"{}\"", s),
            Expr::Var(name, _) => name.clone(),
            Expr::ArrayLit { .. }
            | Expr::TupleLit { .. }
            | Expr::StructLit { .. }
            | Expr::FieldAccess { .. }
            | Expr::Index { .. } => "0".to_string(),
            Expr::Unary { op, expr, .. } => match op {
                UnaryOp::Not => format!("(not {})", self.encode_inner(expr)),
            },
            Expr::Bin { op, lhs, rhs, .. } => match op {
                BinOp::Add => format!("(+ {} {})", self.encode_inner(lhs), self.encode_inner(rhs)),
                BinOp::Sub => format!("(- {} {})", self.encode_inner(lhs), self.encode_inner(rhs)),
                BinOp::Mul => format!("(* {} {})", self.encode_inner(lhs), self.encode_inner(rhs)),
                BinOp::Div => format!(
                    "(div {} {})",
                    self.encode_inner(lhs),
                    self.encode_inner(rhs)
                ),
                BinOp::Shl => {
                    self.helpers.insert(SmtHelper::BitwiseOps);
                    format!(
                        "(clg.shl {} {})",
                        self.encode_inner(lhs),
                        self.encode_inner(rhs)
                    )
                }
                BinOp::Shr => {
                    self.helpers.insert(SmtHelper::BitwiseOps);
                    format!(
                        "(clg.shr {} {})",
                        self.encode_inner(lhs),
                        self.encode_inner(rhs)
                    )
                }
                BinOp::BitAnd => {
                    self.helpers.insert(SmtHelper::BitwiseOps);
                    format!(
                        "(clg.bit_and {} {})",
                        self.encode_inner(lhs),
                        self.encode_inner(rhs)
                    )
                }
                BinOp::BitXor => {
                    self.helpers.insert(SmtHelper::BitwiseOps);
                    format!(
                        "(clg.bit_xor {} {})",
                        self.encode_inner(lhs),
                        self.encode_inner(rhs)
                    )
                }
                BinOp::BitOr => {
                    self.helpers.insert(SmtHelper::BitwiseOps);
                    format!(
                        "(clg.bit_or {} {})",
                        self.encode_inner(lhs),
                        self.encode_inner(rhs)
                    )
                }
                BinOp::Lt => format!("(< {} {})", self.encode_inner(lhs), self.encode_inner(rhs)),
                BinOp::Le => format!("(<= {} {})", self.encode_inner(lhs), self.encode_inner(rhs)),
                BinOp::Gt => format!("(> {} {})", self.encode_inner(lhs), self.encode_inner(rhs)),
                BinOp::Ge => format!("(>= {} {})", self.encode_inner(lhs), self.encode_inner(rhs)),
                BinOp::Eq => format!("(= {} {})", self.encode_inner(lhs), self.encode_inner(rhs)),
                BinOp::Neq => format!(
                    "(not (= {} {}))",
                    self.encode_inner(lhs),
                    self.encode_inner(rhs)
                ),
                BinOp::And => format!(
                    "(and {} {})",
                    self.encode_inner(lhs),
                    self.encode_inner(rhs)
                ),
                BinOp::Or => format!("(or {} {})", self.encode_inner(lhs), self.encode_inner(rhs)),
            },
            Expr::Call { callee, args, .. } => self.encode_call(callee.as_str(), args),
            Expr::Return { expr, .. } => self.encode_inner(expr),
            Expr::Try { expr, .. } => self.encode_try(expr),
            Expr::Match {
                scrutinee, arms, ..
            } => self.encode_match(scrutinee, arms),
            Expr::If {
                cond,
                then_br,
                else_br,
                ..
            } => format!(
                "(ite {} {} {})",
                self.encode_inner(cond),
                self.encode_inner(then_br),
                self.encode_inner(else_br)
            ),
            Expr::Block { block } => {
                if let Some(tail) = &block.tail {
                    self.encode_inner(tail.as_ref())
                } else {
                    "0".to_string()
                }
            }
        }
    }

    fn encode_call(&mut self, callee: &str, args: &[Expr]) -> String {
        match callee {
            "Some" => {
                self.helpers.insert(SmtHelper::VariantAccessors);
                self.helpers.insert(SmtHelper::OptionCtor);
                let val = args
                    .first()
                    .map(|a| self.encode_inner(a))
                    .unwrap_or_else(|| "0".to_string());
                format!("(cl.option.mk 1 {} 0)", val)
            }
            "None" => {
                self.helpers.insert(SmtHelper::VariantAccessors);
                self.helpers.insert(SmtHelper::OptionCtor);
                "(cl.option.mk 0 0 0)".to_string()
            }
            "Ok" => {
                self.helpers.insert(SmtHelper::VariantAccessors);
                self.helpers.insert(SmtHelper::ResultCtor);
                let val = args
                    .first()
                    .map(|a| self.encode_inner(a))
                    .unwrap_or_else(|| "0".to_string());
                format!("(cl.result.mk 1 {} 0)", val)
            }
            "Err" => {
                self.helpers.insert(SmtHelper::VariantAccessors);
                self.helpers.insert(SmtHelper::ResultCtor);
                let val = args
                    .first()
                    .map(|a| self.encode_inner(a))
                    .unwrap_or_else(|| "0".to_string());
                format!("(cl.result.mk 0 {} 0)", val)
            }
            _ => {
                self.record_builtin_call(callee);
                let parts: Vec<String> = args.iter().map(|a| self.encode_inner(a)).collect();
                let smt_callee = smt_symbol(callee);
                if parts.is_empty() {
                    format!("({})", smt_callee)
                } else {
                    format!("({} {})", smt_callee, parts.join(" "))
                }
            }
        }
    }

    fn encode_try(&mut self, expr: &Expr) -> String {
        self.helpers.insert(SmtHelper::VariantAccessors);
        let inner = self.encode_inner(expr);
        let tmp = self.fresh_sym("cl_try");
        format!(
            "(let (({} {})) (cl.variant.payload_lo {}))",
            tmp, inner, tmp
        )
    }

    fn encode_match(&mut self, scrutinee: &Expr, arms: &[MatchArm]) -> String {
        if arms.len() != 2 {
            return "0".to_string();
        }
        if !arms.iter().all(|arm| {
            matches!(
                arm.pat,
                MatchPat::Some(_)
                    | MatchPat::None
                    | MatchPat::Ok(_)
                    | MatchPat::Err(_)
            )
        }) {
            return "0".to_string();
        }
        self.helpers.insert(SmtHelper::VariantAccessors);
        let scrutinee_term = self.encode_inner(scrutinee);
        let tmp = self.fresh_sym("cl_match");

        let cond = self.match_condition(&tmp, &arms[0].pat);
        let then_branch = self.match_branch(&tmp, &arms[0]);
        let else_branch = self.match_branch(&tmp, &arms[1]);

        format!(
            "(let (({} {})) (ite {} {} {}))",
            tmp, scrutinee_term, cond, then_branch, else_branch
        )
    }

    fn match_condition(&mut self, scrutinee_sym: &str, pat: &MatchPat) -> String {
        let tag_value = match pat {
            MatchPat::Some(_) | MatchPat::Ok(_) => 1,
            MatchPat::None | MatchPat::Err(_) => 0,
            MatchPat::Wildcard | MatchPat::EnumVariant { .. } => 0,
        };
        format!("(= (cl.variant.tag {}) {})", scrutinee_sym, tag_value)
    }

    fn match_branch(&mut self, scrutinee_sym: &str, arm: &MatchArm) -> String {
        let body = self.encode_inner(&arm.expr);
        match &arm.pat {
            MatchPat::Some(name) | MatchPat::Ok(name) | MatchPat::Err(name) => {
                let payload = format!("(cl.variant.payload_lo {})", scrutinee_sym);
                format!("(let (({} {})) {})", name, payload, body)
            }
            MatchPat::None | MatchPat::Wildcard | MatchPat::EnumVariant { .. } => body,
        }
    }

    fn fresh_sym(&mut self, prefix: &str) -> String {
        let sym = format!("{}${}", prefix, self.fresh);
        self.fresh += 1;
        sym
    }

    fn wrap_vc(&self, body: &str) -> String {
        self.wrap_vc_with_extra(body, None)
    }

    fn wrap_vc_with_extra(&self, body: &str, extra: Option<&str>) -> String {
        let prelude = self.helpers_prelude();
        let mut parts: Vec<String> = Vec::new();
        if !prelude.is_empty() {
            parts.push(prelude);
        }
        if let Some(extra_block) = extra {
            if !extra_block.is_empty() {
                parts.push(extra_block.to_string());
            }
        }
        parts.push(body.to_string());
        parts.join("\n")
    }

    fn helpers_prelude(&self) -> String {
        if self.helpers.is_empty() && self.builtin_calls.is_empty() {
            return String::new();
        }
        let mut lines: Vec<String> = Vec::new();
        if self.helpers.contains(&SmtHelper::VariantAccessors) {
            lines.push("; Option/Result variants use (tag, payload_lo, payload_hi)".to_string());
            lines.push("(declare-fun cl.variant.tag (Int) Int)".to_string());
            lines.push("(declare-fun cl.variant.payload_lo (Int) Int)".to_string());
            lines.push("(declare-fun cl.variant.payload_hi (Int) Int)".to_string());
        }
        if self.helpers.contains(&SmtHelper::OptionCtor) {
            lines.push("(declare-fun cl.option.mk (Int Int Int) Int)".to_string());
        }
        if self.helpers.contains(&SmtHelper::ResultCtor) {
            lines.push("(declare-fun cl.result.mk (Int Int Int) Int)".to_string());
        }
        if self.helpers.contains(&SmtHelper::BitwiseOps) {
            lines.push("; Bitwise ops are modeled as uninterpreted functions.".to_string());
            lines.push("(declare-fun clg.bit_and (Int Int) Int)".to_string());
            lines.push("(declare-fun clg.bit_or (Int Int) Int)".to_string());
            lines.push("(declare-fun clg.bit_xor (Int Int) Int)".to_string());
            lines.push("(declare-fun clg.shl (Int Int) Int)".to_string());
            lines.push("(declare-fun clg.shr (Int Int) Int)".to_string());
        }
        let builtin_lines = self.builtin_prelude();
        if !builtin_lines.is_empty() {
            lines.push("; Builtin intrinsics are modeled as uninterpreted functions.".to_string());
            lines.extend(builtin_lines);
        }
        lines.join(
            "
",
        )
    }

    fn record_builtin_call(&mut self, callee: &str) {
        if is_builtin_name(callee) {
            self.builtin_calls.insert(callee.to_string());
        }
    }

    fn builtin_prelude(&self) -> Vec<String> {
        if self.builtin_calls.is_empty() {
            return Vec::new();
        }
        let mut lines = Vec::new();
        for (name, params, ret, _) in builtin_sigs() {
            if !self.builtin_calls.contains(&name) {
                continue;
            }
            let args: Vec<&str> = params
                .iter()
                .map(|param| smt_sort_for_builtin(&param.ty))
                .collect();
            let ret_sort = smt_sort_for_builtin(&ret);
            let smt_name = smt_symbol(&name);
            lines.push(format!(
                "(declare-fun {} ({}) {})",
                smt_name,
                args.join(" "),
                ret_sort
            ));
        }
        lines
    }
}

fn smt_symbol(name: &str) -> String {
    if is_simple_smt_symbol(name) {
        name.to_string()
    } else {
        format!("|{}|", name)
    }
}

fn is_simple_smt_symbol(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if first.is_ascii_digit() || !is_smt_symbol_char(first) {
        return false;
    }
    chars.all(is_smt_symbol_char)
}

fn is_smt_symbol_char(c: char) -> bool {
    c.is_ascii_alphanumeric()
        || matches!(
            c,
            '_' | '-' | '+' | '*' | '/' | '=' | '%' | '?' | '!' | '.' | '$' | '<' | '>' | '~'
                | '@' | '^' | '&'
        )
}

fn is_builtin_name(callee: &str) -> bool {
    builtin_sigs()
        .iter()
        .any(|(name, _params, _ret, _)| name == callee)
}

fn smt_sort_for_builtin(ty: &Type) -> &'static str {
    match ty {
        Type::Int | Type::U8 | Type::U64 | Type::U128 | Type::U256 => "Int",
        Type::Bool => "Bool",
        Type::String | Type::Bytes => "String",
        Type::Option(_)
        | Type::Result(_, _)
        | Type::List(_)
        | Type::Set(_)
        | Type::Map(_, _)
        | Type::Array(_, _)
        | Type::Tuple(_)
        | Type::Resource(_) => "Int",
    }
}

struct ArmSource {
    pat: String,
    body: String,
}

fn match_arm_to_source(arm: &MatchArm) -> ArmSource {
    let pat = match &arm.pat {
        MatchPat::Some(name) => format!("Some({})", name),
        MatchPat::None => "None".to_string(),
        MatchPat::Ok(name) => format!("Ok({})", name),
        MatchPat::Err(name) => format!("Err({})", name),
        MatchPat::Wildcard => "_".to_string(),
        MatchPat::EnumVariant {
            enum_name,
            variant,
            binders,
        } => {
            if binders.is_empty() {
                format!("{}::{}", enum_name, variant)
            } else {
                format!("{}::{}({})", enum_name, variant, binders.join(", "))
            }
        }
    };
    let body = expr_to_source(&arm.expr, 0);
    ArmSource { pat, body }
}

fn collect_loops<'a>(expr: &'a Expr, out: &mut Vec<LoopObligation<'a>>) {
    match expr {
        Expr::Block { block } => collect_loops_block(block, out),
        Expr::If {
            cond,
            then_br,
            else_br,
            ..
        } => {
            collect_loops(cond, out);
            collect_loops(then_br, out);
            collect_loops(else_br, out);
        }
        Expr::Match {
            scrutinee, arms, ..
        } => {
            collect_loops(scrutinee, out);
            for arm in arms {
                collect_loops(&arm.expr, out);
            }
        }
        Expr::Bin { lhs, rhs, .. } => {
            collect_loops(lhs, out);
            collect_loops(rhs, out);
        }
        Expr::ArrayLit { elems, .. } | Expr::TupleLit { elems, .. } => {
            for elem in elems {
                collect_loops(elem, out);
            }
        }
        Expr::StructLit { fields, .. } => {
            for field in fields {
                collect_loops(&field.expr, out);
            }
        }
        Expr::Index { base, index, .. } => {
            collect_loops(base, out);
            collect_loops(index, out);
        }
        Expr::FieldAccess { base, .. } => {
            collect_loops(base, out);
        }
        Expr::Call { args, .. } => {
            for arg in args {
                collect_loops(arg, out);
            }
        }
        Expr::Unary { expr, .. } | Expr::Return { expr, .. } | Expr::Try { expr, .. } => {
            collect_loops(expr, out);
        }
        Expr::Int(_, _) | Expr::Bool(_, _) | Expr::String(_, _) | Expr::Var(_, _) => {}
    }
}

fn collect_loops_block<'a>(block: &'a clg_ast::Block, out: &mut Vec<LoopObligation<'a>>) {
    for stmt in &block.statements {
        match stmt {
            Stmt::Let { expr, .. } | Stmt::Expr { expr, .. } => collect_loops(expr, out),
            Stmt::While {
                cond,
                invariant,
                variant,
                body,
                ..
            } => {
                out.push(LoopObligation {
                    invariant,
                    variant: variant.as_deref(),
                });
                collect_loops(cond, out);
                collect_loops(invariant, out);
                if let Some(v) = variant {
                    collect_loops(v, out);
                }
                collect_loops_block(body, out);
            }
        }
    }
    if let Some(tail) = &block.tail {
        collect_loops(tail, out);
    }
}

fn expr_span(e: &Expr) -> Span {
    match e {
        Expr::Int(_, sp) | Expr::Bool(_, sp) | Expr::String(_, sp) | Expr::Var(_, sp) => *sp,
        Expr::ArrayLit { span, .. }
        | Expr::TupleLit { span, .. }
        | Expr::StructLit { span, .. }
        | Expr::FieldAccess { span, .. }
        | Expr::Index { span, .. } => *span,
        Expr::Bin { span, .. }
        | Expr::Call { span, .. }
        | Expr::Match { span, .. }
        | Expr::Return { span, .. }
        | Expr::If { span, .. }
        | Expr::Unary { span, .. }
        | Expr::Try { span, .. } => *span,
        Expr::Block { block } => block.span,
    }
}

fn precedence_bin(op: BinOp) -> u8 {
    match op {
        BinOp::Mul | BinOp::Div => 60,
        BinOp::Add | BinOp::Sub => 50,
        BinOp::Shl | BinOp::Shr => 45,
        BinOp::BitAnd => 40,
        BinOp::BitXor => 39,
        BinOp::BitOr => 38,
        BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge | BinOp::Eq | BinOp::Neq => 30,
        BinOp::And => 10,
        BinOp::Or => 5,
    }
}

fn precedence_unary() -> u8 {
    50
}

fn merge_span(lhs: &Expr, rhs: &Expr) -> Span {
    let ls = span_of(lhs);
    let rs = span_of(rhs);
    Span {
        start: ls.start,
        end: rs.end,
    }
}

fn span_of(expr: &Expr) -> Span {
    match expr {
        Expr::Int(_, sp) | Expr::Bool(_, sp) | Expr::String(_, sp) | Expr::Var(_, sp) => *sp,
        Expr::ArrayLit { span, .. }
        | Expr::TupleLit { span, .. }
        | Expr::StructLit { span, .. }
        | Expr::FieldAccess { span, .. }
        | Expr::Index { span, .. } => *span,
        Expr::Bin { span, .. }
        | Expr::Call { span, .. }
        | Expr::Match { span, .. }
        | Expr::Return { span, .. }
        | Expr::If { span, .. }
        | Expr::Unary { span, .. }
        | Expr::Try { span, .. } => *span,
        Expr::Block { block } => block.span,
    }
}
