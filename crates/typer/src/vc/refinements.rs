use crate::builtins::builtin_sigs;
use clg_ast::{BinOp, Expr, MatchPat, Program, Span, Stmt, Type};
use std::collections::HashMap;

use super::{
    snapshot_expr, RefinementAttachment, RefinementFlowDetail, RefinementFlowKind,
    RefinementObligation,
};

pub(super) fn smt_sort_for_type(ty: &Type, aliases: &HashMap<&str, AliasView<'_>>) -> &'static str {
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
        | Type::Slice(_)
        | Type::Tuple(_)
        | Type::Named { .. } => {
            if let Type::Named { name, args } = ty {
                if args.is_empty() {
                    if let Some(alias) = aliases.get(name.as_str()) {
                        return smt_sort_for_type(alias.base, aliases);
                    }
                }
            }
            "Int"
        }
    }
}

pub(super) fn refinement_prelude(
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

pub(super) fn merge_extras(parts: &[&str]) -> String {
    let mut out = Vec::new();
    for part in parts {
        let trimmed = part.trim();
        if !trimmed.is_empty() {
            out.push(trimmed);
        }
    }
    out.join("\n")
}

pub(super) fn make_refinement_obligation(
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
    if let Type::Named { name, args } = ty {
        if args.is_empty() {
            return aliases.get(name.as_str());
        }
    }
    None
}

#[derive(Clone)]
pub(super) struct AliasView<'a> {
    pub(super) name: &'a str,
    pub(super) base: &'a Type,
    pub(super) binder: Option<&'a str>,
    pub(super) predicate: &'a Expr,
    pub(super) span: Span,
}

pub(super) fn build_alias_map<'a>(program: &'a Program) -> HashMap<&'a str, AliasView<'a>> {
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
pub(super) struct FnSigView<'a> {
    pub(super) ret: Type,
    pub(super) param_aliases: Vec<Option<&'a AliasView<'a>>>,
    pub(super) ret_alias: Option<&'a AliasView<'a>>,
}

pub(super) fn build_fn_sigs<'a>(
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

pub(super) fn collect_refinement_obligations<'a>(
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
                Some(Type::Array(Box::new(first), Some(elems.len() as u32)))
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
                collect_refinement_obligations(
                    &field.expr,
                    aliases,
                    fn_sigs,
                    &mut env.clone(),
                    out,
                );
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
                Some(Type::Slice(inner)) => Some(*inner),
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

pub(super) fn collect_block_refinements<'a>(
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

pub(super) fn fold_conjunction(mut exprs: Vec<Expr>) -> Expr {
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

pub(super) fn substitute_result(expr: &Expr, replacement: &Expr) -> Expr {
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
