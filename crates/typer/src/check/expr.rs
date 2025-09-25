use super::{effect_label, EffectLevel, FnSig};
use crate::errors::TyperError;
use anyhow::Result;
use clg_ast::{BinOp, Expr, Span, Type, UnaryOp};
use std::collections::HashMap;

pub(super) fn type_of<'a>(
    e: &'a Expr,
    env: &HashMap<&'a str, Type>,
    fns: &HashMap<&'a str, FnSig<'a>>,
    depth: usize,
) -> Result<Type> {
    if depth > 1024 {
        return Err(TyperError::new(
            "T011",
            "type-check recursion limit exceeded".to_string(),
            0,
            0,
        )
        .into());
    }
    match e {
        Expr::Int(_, _) => Ok(Type::Int),
        Expr::Bool(_, _) => Ok(Type::Bool),
        Expr::String(_, _) => Ok(Type::String),
        Expr::If {
            cond,
            then_br,
            else_br,
            span,
        } => {
            let cty = type_of(cond, env, fns, depth + 1)?;
            if cty != Type::Bool {
                // Reuse arg_type_mismatch with pseudo-callee `if` for stable code T003
                return Err(TyperError::arg_type_mismatch(1, "if", Type::Bool, cty, *span).into());
            }
            let tty = type_of(then_br, env, fns, depth + 1)?;
            let ety = type_of(else_br, env, fns, depth + 1)?;
            if tty != ety {
                return Err(TyperError::branch_type_mismatch(tty, ety, *span).into());
            }
            Ok(tty)
        }
        Expr::Match {
            scrutinee,
            arms,
            span,
        } => {
            let scrut_ty = type_of(scrutinee, env, fns, depth + 1)?;
            use clg_ast::MatchPat;
            match scrut_ty.clone() {
                Type::Option(inner_ty) => {
                    let mut seen_some = false;
                    let mut seen_none = false;
                    let mut res_ty_opt: Option<Type> = None;
                    for arm in arms {
                        match &arm.pat {
                            MatchPat::Some(name) => {
                                if seen_some {
                                    return Err(
                                        TyperError::match_duplicate_arm("Some", *span).into()
                                    );
                                }
                                seen_some = true;
                                if env.contains_key(name.as_str()) {
                                    return Err(TyperError::binder_conflict(name, *span).into());
                                }
                                // Extend env with binder
                                let mut env2 = env.clone();
                                env2.insert(name.as_str(), *inner_ty.clone());
                                let at = type_of(&arm.expr, &env2, fns, depth + 1)?;
                                if let Some(rt) = &res_ty_opt {
                                    if &at != rt {
                                        let sp = expr_span(&arm.expr);
                                        return Err(TyperError::match_arm_type_mismatch(
                                            rt.clone(),
                                            at,
                                            sp,
                                        )
                                        .into());
                                    }
                                } else {
                                    res_ty_opt = Some(at);
                                }
                            }
                            MatchPat::None => {
                                if seen_none {
                                    return Err(
                                        TyperError::match_duplicate_arm("None", *span).into()
                                    );
                                }
                                seen_none = true;
                                let at = type_of(&arm.expr, env, fns, depth + 1)?;
                                if let Some(rt) = &res_ty_opt {
                                    if &at != rt {
                                        let sp = expr_span(&arm.expr);
                                        return Err(TyperError::match_arm_type_mismatch(
                                            rt.clone(),
                                            at,
                                            sp,
                                        )
                                        .into());
                                    }
                                } else {
                                    res_ty_opt = Some(at);
                                }
                            }
                            MatchPat::Ok(_) | MatchPat::Err(_) => {
                                // Wrong arm kind for Option
                                return Err(TyperError::match_invalid_scrutinee(
                                    scrut_ty.clone(),
                                    *span,
                                )
                                .into());
                            }
                        }
                    }
                    if !(seen_some && seen_none) {
                        return Err(TyperError::match_non_exhaustive(*span).into());
                    }
                    Ok(res_ty_opt.unwrap_or(Type::Int)) // unreachable: arms non-empty
                }
                Type::Result(ok_ty, err_ty) => {
                    let mut seen_ok = false;
                    let mut seen_err = false;
                    let mut res_ty_opt: Option<Type> = None;
                    for arm in arms {
                        match &arm.pat {
                            MatchPat::Ok(name) => {
                                if seen_ok {
                                    return Err(TyperError::match_duplicate_arm("Ok", *span).into());
                                }
                                seen_ok = true;
                                if env.contains_key(name.as_str()) {
                                    return Err(TyperError::binder_conflict(name, *span).into());
                                }
                                let mut env2 = env.clone();
                                env2.insert(name.as_str(), *ok_ty.clone());
                                let at = type_of(&arm.expr, &env2, fns, depth + 1)?;
                                if let Some(rt) = &res_ty_opt {
                                    if &at != rt {
                                        let sp = expr_span(&arm.expr);
                                        return Err(TyperError::match_arm_type_mismatch(
                                            rt.clone(),
                                            at,
                                            sp,
                                        )
                                        .into());
                                    }
                                } else {
                                    res_ty_opt = Some(at);
                                }
                            }
                            MatchPat::Err(name) => {
                                if seen_err {
                                    return Err(
                                        TyperError::match_duplicate_arm("Err", *span).into()
                                    );
                                }
                                seen_err = true;
                                if env.contains_key(name.as_str()) {
                                    return Err(TyperError::binder_conflict(name, *span).into());
                                }
                                let mut env2 = env.clone();
                                env2.insert(name.as_str(), *err_ty.clone());
                                let at = type_of(&arm.expr, &env2, fns, depth + 1)?;
                                if let Some(rt) = &res_ty_opt {
                                    if &at != rt {
                                        let sp = expr_span(&arm.expr);
                                        return Err(TyperError::match_arm_type_mismatch(
                                            rt.clone(),
                                            at,
                                            sp,
                                        )
                                        .into());
                                    }
                                } else {
                                    res_ty_opt = Some(at);
                                }
                            }
                            MatchPat::Some(_) | MatchPat::None => {
                                return Err(TyperError::match_invalid_scrutinee(
                                    scrut_ty.clone(),
                                    *span,
                                )
                                .into());
                            }
                        }
                    }
                    if !(seen_ok && seen_err) {
                        return Err(TyperError::match_non_exhaustive(*span).into());
                    }
                    Ok(res_ty_opt.unwrap_or(Type::Int))
                }
                other => Err(TyperError::match_invalid_scrutinee(other, *span).into()),
            }
        }
        Expr::Var(name, sp) => Ok(env
            .get(name.as_str())
            .cloned()
            .ok_or_else(|| TyperError::unknown_variable(name, *sp))?),
        Expr::Return { expr, .. } => {
            let t = type_of(expr, env, fns, depth + 1)?;
            Ok(t)
        }
        Expr::Unary { op, expr, span } => {
            let inner = type_of(expr, env, fns, depth + 1)?;
            match op {
                UnaryOp::Not => {
                    ensure_bool(inner, "operand", Some(*span))?;
                    Ok(Type::Bool)
                }
            }
        }
        Expr::Bin { op, lhs, rhs, span } => {
            let lt = type_of(lhs, env, fns, depth + 1)?;
            let rt = type_of(rhs, env, fns, depth + 1)?;
            match op {
                BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div => {
                    ensure_int(lt, "left operand", Some(*span))?;
                    ensure_int(rt, "right operand", Some(*span))?;
                    Ok(Type::Int)
                }
                BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
                    ensure_int(lt, "left operand", Some(*span))?;
                    ensure_int(rt, "right operand", Some(*span))?;
                    Ok(Type::Bool)
                }
                BinOp::Eq | BinOp::Neq => {
                    if lt != rt {
                        let op_str = if *op == BinOp::Eq { "==" } else { "!=" };
                        return Err(
                            TyperError::binary_operands_mismatch(op_str, lt, rt, *span).into()
                        );
                    }
                    Ok(Type::Bool)
                }
                BinOp::And | BinOp::Or => {
                    ensure_bool(lt, "left operand", Some(*span))?;
                    ensure_bool(rt, "right operand", Some(*span))?;
                    Ok(Type::Bool)
                }
            }
        }
        Expr::Call { callee, args, span } => {
            // Phase 4.6 — Collections signatures (type-only)
            if let Some(t) = type_collection_call(callee, args, env, fns, depth, *span)? {
                return Ok(t);
            }
            // Phase 4.5 — ADT constructors (partial): Some(T) infers Option<T>
            if callee == "Some" {
                if args.len() != 1 {
                    return Err(TyperError::arity_mismatch(callee, 1, args.len(), *span).into());
                }
                let t0 = type_of(&args[0], env, fns, depth + 1)?;
                return Ok(Type::Option(Box::new(t0)));
            }
            let FnSig { params, ret, .. } = fns
                .get(callee.as_str())
                .cloned()
                .ok_or_else(|| TyperError::unknown_function(callee, *span))?;
            if params.len() != args.len() {
                return Err(
                    TyperError::arity_mismatch(callee, params.len(), args.len(), *span).into(),
                );
            }
            for (i, (p, a)) in params.iter().zip(args.iter()).enumerate() {
                let at = type_of(a, env, fns, depth + 1)?;
                let expected = p.ty.clone();
                if expected != at {
                    let sp = expr_span(a);
                    return Err(TyperError::arg_type_mismatch(i, callee, expected, at, sp).into());
                }
            }
            Ok(ret)
        }
    }
}

fn type_collection_call<'a>(
    callee: &str,
    args: &'a [Expr],
    env: &HashMap<&'a str, Type>,
    fns: &HashMap<&'a str, FnSig<'a>>,
    depth: usize,
    span: Span,
) -> Result<Option<Type>> {
    // Helper to get type of an expression
    let arg_ty = |i: usize| -> Result<Type> { type_of(&args[i], env, fns, depth + 1) };

    let normalized_callee = match callee {
        "std::list::push_mut" => "std::list::push",
        "std::list::insert_mut" => "std::list::insert",
        "std::list::remove_mut" => "std::list::remove",
        "std::list::pop_mut" => "std::list::pop",
        "std::set::insert_mut" => "std::set::insert",
        "std::set::remove_mut" => "std::set::remove",
        "std::map::insert_mut" => "std::map::insert",
        "std::map::remove_mut" => "std::map::remove",
        other => other,
    };
    match normalized_callee {
        // List
        "std::list::len" => {
            if args.len() != 1 {
                return Err(TyperError::arity_mismatch(callee, 1, args.len(), span).into());
            }
            let lty = arg_ty(0)?;
            if let Type::List(_) = lty {
                Ok(Some(Type::Int))
            } else {
                Err(TyperError::expected_collection("List", lty, span).into())
            }
        }
        "std::list::can_mut" => {
            if args.len() != 1 {
                return Err(TyperError::arity_mismatch(callee, 1, args.len(), span).into());
            }
            let lty = arg_ty(0)?;
            if let Type::List(_) = lty {
                Ok(Some(Type::Bool))
            } else {
                Err(TyperError::expected_collection("List", lty, span).into())
            }
        }
        "std::list::get" => {
            if args.len() != 2 {
                return Err(TyperError::arity_mismatch(callee, 2, args.len(), span).into());
            }
            let lty = arg_ty(0)?;
            let ity = arg_ty(1)?;
            if ity != Type::Int {
                let sp = expr_span(&args[1]);
                return Err(TyperError::int_operand("index", ity, Some(sp)).into());
            }
            match lty {
                Type::List(inner) => Ok(Some(Type::Option(inner))),
                other => Err(TyperError::expected_collection("List", other, span).into()),
            }
        }
        "std::list::push" => {
            if args.len() != 2 {
                return Err(TyperError::arity_mismatch(callee, 2, args.len(), span).into());
            }
            let lty = arg_ty(0)?;
            match lty {
                Type::List(inner) => {
                    let letxty: Type = (*inner).clone();
                    let aty = arg_ty(1)?;
                    if aty != letxty {
                        let sp = expr_span(&args[1]);
                        return Err(TyperError::element_type_mismatch(letxty, aty, sp).into());
                    }
                    Ok(Some(Type::List(Box::new(*inner))))
                }
                other => Err(TyperError::expected_collection("List", other, span).into()),
            }
        }
        "std::list::insert" => {
            if args.len() != 3 {
                return Err(TyperError::arity_mismatch(callee, 3, args.len(), span).into());
            }
            let lty = arg_ty(0)?;
            match lty {
                Type::List(inner) => {
                    let elem_expected: Type = (*inner).clone();
                    let aty_elem = arg_ty(1)?;
                    if aty_elem != elem_expected {
                        let sp = expr_span(&args[1]);
                        return Err(
                            TyperError::element_type_mismatch(elem_expected, aty_elem, sp).into(),
                        );
                    }
                    let ity = arg_ty(2)?;
                    if ity != Type::Int {
                        let sp = expr_span(&args[2]);
                        return Err(TyperError::int_operand("index", ity, Some(sp)).into());
                    }
                    Ok(Some(Type::List(Box::new(*inner))))
                }
                other => Err(TyperError::expected_collection("List", other, span).into()),
            }
        }
        "std::list::remove" => {
            if args.len() != 2 {
                return Err(TyperError::arity_mismatch(callee, 2, args.len(), span).into());
            }
            let lty = arg_ty(0)?;
            let ity = arg_ty(1)?;
            if ity != Type::Int {
                let sp = expr_span(&args[1]);
                return Err(TyperError::int_operand("index", ity, Some(sp)).into());
            }
            match lty {
                Type::List(inner) => Ok(Some(Type::List(inner))),
                other => Err(TyperError::expected_collection("List", other, span).into()),
            }
        }
        "std::list::pop" => {
            if args.len() != 1 {
                return Err(TyperError::arity_mismatch(callee, 1, args.len(), span).into());
            }
            let lty = arg_ty(0)?;
            match lty {
                Type::List(inner) => Ok(Some(Type::Option(inner))),
                other => Err(TyperError::expected_collection("List", other, span).into()),
            }
        }
        "std::list::new" => Err(TyperError::cannot_infer_collection(span, "std::list").into()),

        // Set
        "std::set::len" => {
            if args.len() != 1 {
                return Err(TyperError::arity_mismatch(callee, 1, args.len(), span).into());
            }
            let sty = arg_ty(0)?;
            if let Type::Set(_) = sty {
                Ok(Some(Type::Int))
            } else {
                Err(TyperError::expected_collection("Set", sty, span).into())
            }
        }
        "std::set::can_mut" => {
            if args.len() != 1 {
                return Err(TyperError::arity_mismatch(callee, 1, args.len(), span).into());
            }
            let sty = arg_ty(0)?;
            if let Type::Set(_) = sty {
                Ok(Some(Type::Bool))
            } else {
                Err(TyperError::expected_collection("Set", sty, span).into())
            }
        }
        "std::set::contains" => {
            if args.len() != 2 {
                return Err(TyperError::arity_mismatch(callee, 2, args.len(), span).into());
            }
            match arg_ty(0)? {
                Type::Set(inner) => {
                    let aty = arg_ty(1)?;
                    if aty != *inner {
                        let sp = expr_span(&args[1]);
                        return Err(TyperError::element_type_mismatch(*inner, aty, sp).into());
                    }
                    Ok(Some(Type::Bool))
                }
                other => Err(TyperError::expected_collection("Set", other, span).into()),
            }
        }
        "std::set::insert" | "std::set::remove" => {
            if args.len() != 2 {
                return Err(TyperError::arity_mismatch(callee, 2, args.len(), span).into());
            }
            match arg_ty(0)? {
                Type::Set(inner) => {
                    let aty = arg_ty(1)?;
                    if aty != *inner {
                        let sp = expr_span(&args[1]);
                        return Err(TyperError::element_type_mismatch(*inner, aty, sp).into());
                    }
                    Ok(Some(Type::Set(inner)))
                }
                other => Err(TyperError::expected_collection("Set", other, span).into()),
            }
        }
        "std::set::new" => Err(TyperError::cannot_infer_collection(span, "std::set").into()),

        // Map
        "std::map::len" => {
            if args.len() != 1 {
                return Err(TyperError::arity_mismatch(callee, 1, args.len(), span).into());
            }
            let mty = arg_ty(0)?;
            if let Type::Map(_, _) = mty {
                Ok(Some(Type::Int))
            } else {
                Err(TyperError::expected_collection("Map", mty, span).into())
            }
        }
        "std::map::can_mut" => {
            if args.len() != 1 {
                return Err(TyperError::arity_mismatch(callee, 1, args.len(), span).into());
            }
            let mty = arg_ty(0)?;
            if let Type::Map(_, _) = mty {
                Ok(Some(Type::Bool))
            } else {
                Err(TyperError::expected_collection("Map", mty, span).into())
            }
        }
        "std::map::contains" => {
            if args.len() != 2 {
                return Err(TyperError::arity_mismatch(callee, 2, args.len(), span).into());
            }
            match arg_ty(0)? {
                Type::Map(k, _v) => {
                    let aty = arg_ty(1)?;
                    if aty != *k {
                        let sp = expr_span(&args[1]);
                        return Err(TyperError::element_type_mismatch(*k, aty, sp).into());
                    }
                    Ok(Some(Type::Bool))
                }
                other => Err(TyperError::expected_collection("Map", other, span).into()),
            }
        }
        "std::map::get" => {
            if args.len() != 2 {
                return Err(TyperError::arity_mismatch(callee, 2, args.len(), span).into());
            }
            match arg_ty(0)? {
                Type::Map(k, v) => {
                    let aty = arg_ty(1)?;
                    if aty != *k {
                        let sp = expr_span(&args[1]);
                        return Err(TyperError::element_type_mismatch(*k, aty, sp).into());
                    }
                    Ok(Some(Type::Option(v)))
                }
                other => Err(TyperError::expected_collection("Map", other, span).into()),
            }
        }
        "std::map::insert" => {
            if args.len() != 3 {
                return Err(TyperError::arity_mismatch(callee, 3, args.len(), span).into());
            }
            match arg_ty(0)? {
                Type::Map(k, v) => {
                    let aty_k = arg_ty(1)?;
                    if aty_k != *k {
                        let sp = expr_span(&args[1]);
                        return Err(TyperError::element_type_mismatch(*k, aty_k, sp).into());
                    }
                    let aty_v = arg_ty(2)?;
                    if aty_v != *v {
                        let sp = expr_span(&args[2]);
                        return Err(TyperError::element_type_mismatch(*v, aty_v, sp).into());
                    }
                    Ok(Some(Type::Map(k, v)))
                }
                other => Err(TyperError::expected_collection("Map", other, span).into()),
            }
        }
        "std::map::remove" => {
            if args.len() != 2 {
                return Err(TyperError::arity_mismatch(callee, 2, args.len(), span).into());
            }
            match arg_ty(0)? {
                Type::Map(k, v) => {
                    let aty = arg_ty(1)?;
                    if aty != *k {
                        let sp = expr_span(&args[1]);
                        return Err(TyperError::element_type_mismatch(*k, aty, sp).into());
                    }
                    Ok(Some(Type::Map(k, v)))
                }
                other => Err(TyperError::expected_collection("Map", other, span).into()),
            }
        }
        "std::map::new" => Err(TyperError::cannot_infer_collection(span, "std::map").into()),

        _ => Ok(None),
    }
}

pub(super) fn max_effect<'a>(
    e: &'a Expr,
    fns: &HashMap<&'a str, FnSig<'a>>,
    allowed: EffectLevel,
) -> Result<EffectLevel> {
    match e {
        Expr::Int(_, _) | Expr::Bool(_, _) | Expr::String(_, _) | Expr::Var(_, _) => {
            Ok(EffectLevel::Pure)
        }
        Expr::Return { expr, .. } => max_effect(expr, fns, allowed),
        Expr::Unary { expr, .. } => max_effect(expr, fns, allowed),
        Expr::Bin { lhs, rhs, .. } => {
            let left = max_effect(lhs, fns, allowed)?;
            let right = max_effect(rhs, fns, allowed)?;
            Ok(left.join(right))
        }
        Expr::If {
            cond,
            then_br,
            else_br,
            ..
        } => {
            let cond_eff = max_effect(cond, fns, allowed)?;
            let then_eff = max_effect(then_br, fns, allowed)?;
            let else_eff = max_effect(else_br, fns, allowed)?;
            Ok(cond_eff.join(then_eff).join(else_eff))
        }
        Expr::Match {
            scrutinee, arms, ..
        } => {
            let mut eff = max_effect(scrutinee, fns, allowed)?;
            for arm in arms {
                eff = eff.join(max_effect(&arm.expr, fns, allowed)?);
            }
            Ok(eff)
        }
        Expr::Call { callee, args, span } => {
            let mut eff = EffectLevel::Pure;
            for arg in args {
                eff = eff.join(max_effect(arg, fns, allowed)?);
            }
            let call_eff = call_effect(callee, fns);
            if call_eff > allowed {
                return Err(
                    TyperError::effect_required(callee, effect_label(call_eff), *span).into(),
                );
            }
            Ok(eff.join(call_eff))
        }
    }
}

fn call_effect<'a>(callee: &str, fns: &HashMap<&'a str, FnSig<'a>>) -> EffectLevel {
    if let Some(level) = builtin_effect(callee) {
        return level;
    }
    fns.get(callee)
        .map(|sig| sig.effect)
        .unwrap_or(EffectLevel::Pure)
}

fn builtin_effect(callee: &str) -> Option<EffectLevel> {
    match callee {
        "std::list::push_mut"
        | "std::list::insert_mut"
        | "std::list::remove_mut"
        | "std::list::pop_mut"
        | "std::set::insert_mut"
        | "std::set::remove_mut"
        | "std::map::insert_mut"
        | "std::map::remove_mut" => Some(EffectLevel::Mut),
        _ => None,
    }
}

fn ensure_int(ty: Type, what: &str, span: Option<Span>) -> Result<()> {
    if ty != Type::Int {
        return Err(TyperError::int_operand(what, ty, span).into());
    }
    Ok(())
}

fn ensure_bool(ty: Type, what: &str, span: Option<Span>) -> Result<()> {
    if ty != Type::Bool {
        return Err(TyperError::bool_operand(what, ty, span).into());
    }
    Ok(())
}

pub(super) fn expr_span(e: &Expr) -> Span {
    match e {
        Expr::Int(_, sp) | Expr::Bool(_, sp) | Expr::String(_, sp) | Expr::Var(_, sp) => *sp,
        Expr::Bin { span, .. }
        | Expr::Call { span, .. }
        | Expr::Match { span, .. }
        | Expr::Return { span, .. }
        | Expr::If { span, .. }
        | Expr::Unary { span, .. } => *span,
    }
}

pub(crate) fn show_ty(t: Type) -> &'static str {
    match t {
        Type::Int => "Int",
        Type::Bool => "Bool",
        Type::String => "String",
        Type::Option(_) => "Option",
        Type::Result(_, _) => "Result",
        Type::List(_) => "List",
        Type::Set(_) => "Set",
        Type::Map(_, _) => "Map",
    }
}
