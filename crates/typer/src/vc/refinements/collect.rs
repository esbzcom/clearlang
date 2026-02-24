use std::collections::HashMap;

use clg_ast::{BinOp, Expr, MatchPat, Stmt, Type};

use super::alias::{top_level_alias, AliasView, FnSigView};
use super::obligations::make_refinement_obligation;
use crate::vc::{
    RefinementAttachment, RefinementFlowDetail, RefinementFlowKind, RefinementObligation,
};

pub(crate) fn collect_refinement_obligations<'a>(
    expr: &'a Expr,
    aliases: &HashMap<&'a str, AliasView<'a>>,
    fn_sigs: &HashMap<String, FnSigView>,
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
                                    &alias,
                                    &Expr::Var(name.clone(), alias.alias_span),
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
                                    &alias,
                                    &Expr::Var(name.clone(), alias.alias_span),
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
                                    &alias,
                                    &Expr::Var(name.clone(), alias.alias_span),
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
                        if let Some(obligation) =
                            make_refinement_obligation(&alias, arg, attachment)
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
        Expr::Lambda { body, .. } => {
            collect_refinement_obligations(body.as_ref(), aliases, fn_sigs, &mut env.clone(), out);
            None
        }
    }
}

pub(super) fn collect_block_refinements<'a>(
    block: &'a clg_ast::Block,
    aliases: &HashMap<&'a str, AliasView<'a>>,
    fn_sigs: &HashMap<String, FnSigView>,
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
                            &alias,
                            &Expr::Var(name.clone(), alias.alias_span),
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
