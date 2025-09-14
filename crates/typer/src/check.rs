use anyhow::{Context, Result};
use std::collections::HashMap;
use crate::errors::TyperError;
use crate::lower::{lower_func};
use crate::builtins::builtin_sigs;
use lumi_ast::{BinOp, Effect, Expr, Func, Param, Program, Type, Span};
use lumi_ir::Module;

type FnSig<'a> = (&'a [Param], Type);

pub fn check(ast: &Program) -> Result<Module> {
    let mut fns: HashMap<&str, FnSig> = HashMap::new();

    let builtins = builtin_sigs();
    for (name, params, ret) in &builtins {
        fns.insert(name.as_str(), (&params[..], ret.clone()));
    }

    for f in &ast.funcs {
        if fns.insert(f.name.as_str(), (&f.params, f.ret.clone())).is_some() {
            return Err(TyperError::duplicate_function(&f.name).into());
        }
    }

    for f in &ast.funcs {
        check_func(f, &fns).with_context(|| format!("in function `{}`", f.name))?;
    }

    let mut module = Module::default();
    for f in &ast.funcs {
        module.funcs.push(lower_func(f, &fns)?);
    }
    Ok(module)
}

pub fn type_check_only(ast: &Program) -> Result<()> {
    let mut fns: HashMap<&str, FnSig> = HashMap::new();

    let builtins = builtin_sigs();
    for (name, params, ret) in &builtins {
        fns.insert(name.as_str(), (&params[..], ret.clone()));
    }

    for f in &ast.funcs {
        if fns.insert(f.name.as_str(), (&f.params, f.ret.clone())).is_some() {
            return Err(TyperError::duplicate_function(&f.name).into());
        }
    }

    for f in &ast.funcs {
        check_func(f, &fns).with_context(|| format!("in function `{}`", f.name))?;
    }
    Ok(())
}

fn check_func<'a>(f: &'a Func, fns: &HashMap<&'a str, FnSig<'a>>) -> Result<()> {
    match f.effect {
        Effect::None | Effect::Pure => {}
        Effect::Mut | Effect::Io => return Err(TyperError::effect_not_supported(f.effect).into()),
    }
    let mut env: HashMap<&str, Type> = HashMap::new();
    for p in &f.params {
        if env.insert(p.name.as_str(), p.ty.clone()).is_some() {
            return Err(TyperError::duplicate_parameter(&p.name).into());
        }
    }

    let body_ty = type_of(&f.body, &env, fns, 0)?;
    if body_ty != f.ret {
        let sp = match &f.body {
            Expr::Int(_, sp) | Expr::Bool(_, sp) | Expr::String(_, sp) | Expr::Var(_, sp) => *sp,
            Expr::Bin { span, .. } | Expr::Call { span, .. } | Expr::Match { span, .. } | Expr::Return { span, .. } | Expr::If { span, .. } => *span,
        };
        return Err(TyperError::return_type_mismatch(f.ret.clone(), body_ty, sp).into());
    }
    Ok(())
}

fn type_of<'a>(
    e: &'a Expr,
    env: &HashMap<&'a str, Type>,
    fns: &HashMap<&'a str, FnSig<'a>>,
    depth: usize,
) -> Result<Type> {
    if depth > 1024 {
        return Err(TyperError::new("T011", "type-check recursion limit exceeded".to_string(), 0, 0).into());
    }
    match e {
        Expr::Int(_, _) => Ok(Type::Int),
        Expr::Bool(_, _) => Ok(Type::Bool),
        Expr::String(_, _) => Ok(Type::String),
        Expr::If { cond, then_br, else_br, span } => {
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
        Expr::Match { scrutinee, arms, span } => {
            let scrut_ty = type_of(scrutinee, env, fns, depth + 1)?;
            // Helper to get arm expr span
            let span_of = |ex: &Expr| -> Span {
                match ex {
                    Expr::Int(_, sp) | Expr::Bool(_, sp) | Expr::String(_, sp) | Expr::Var(_, sp) => *sp,
                    Expr::Bin { span, .. } | Expr::Call { span, .. } | Expr::Match { span, .. } | Expr::Return { span, .. } | Expr::If { span, .. } => *span,
                }
            };

            use lumi_ast::MatchPat;
            match scrut_ty.clone() {
                Type::Option(inner_ty) => {
                    let mut seen_some = false;
                    let mut seen_none = false;
                    let mut res_ty_opt: Option<Type> = None;
                    for arm in arms {
                        match &arm.pat {
                            MatchPat::Some(name) => {
                                if seen_some {
                                    return Err(TyperError::match_duplicate_arm("Some", *span).into());
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
                                        let sp = span_of(&arm.expr);
                                        return Err(TyperError::match_arm_type_mismatch(rt.clone(), at, sp).into());
                                    }
                                } else {
                                    res_ty_opt = Some(at);
                                }
                            }
                            MatchPat::None => {
                                if seen_none {
                                    return Err(TyperError::match_duplicate_arm("None", *span).into());
                                }
                                seen_none = true;
                                let at = type_of(&arm.expr, env, fns, depth + 1)?;
                                if let Some(rt) = &res_ty_opt {
                                    if &at != rt {
                                        let sp = span_of(&arm.expr);
                                        return Err(TyperError::match_arm_type_mismatch(rt.clone(), at, sp).into());
                                    }
                                } else {
                                    res_ty_opt = Some(at);
                                }
                            }
                            MatchPat::Ok(_) | MatchPat::Err(_) => {
                                // Wrong arm kind for Option
                                return Err(TyperError::match_invalid_scrutinee(scrut_ty.clone(), *span).into());
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
                                if seen_ok { return Err(TyperError::match_duplicate_arm("Ok", *span).into()); }
                                seen_ok = true;
                                if env.contains_key(name.as_str()) { return Err(TyperError::binder_conflict(name, *span).into()); }
                                let mut env2 = env.clone();
                                env2.insert(name.as_str(), *ok_ty.clone());
                                let at = type_of(&arm.expr, &env2, fns, depth + 1)?;
                                if let Some(rt) = &res_ty_opt {
                                    if &at != rt { let sp = span_of(&arm.expr); return Err(TyperError::match_arm_type_mismatch(rt.clone(), at, sp).into()); }
                                } else { res_ty_opt = Some(at); }
                            }
                            MatchPat::Err(name) => {
                                if seen_err { return Err(TyperError::match_duplicate_arm("Err", *span).into()); }
                                seen_err = true;
                                if env.contains_key(name.as_str()) { return Err(TyperError::binder_conflict(name, *span).into()); }
                                let mut env2 = env.clone();
                                env2.insert(name.as_str(), *err_ty.clone());
                                let at = type_of(&arm.expr, &env2, fns, depth + 1)?;
                                if let Some(rt) = &res_ty_opt {
                                    if &at != rt { let sp = span_of(&arm.expr); return Err(TyperError::match_arm_type_mismatch(rt.clone(), at, sp).into()); }
                                } else { res_ty_opt = Some(at); }
                            }
                            MatchPat::Some(_) | MatchPat::None => {
                                return Err(TyperError::match_invalid_scrutinee(scrut_ty.clone(), *span).into());
                            }
                        }
                    }
                    if !(seen_ok && seen_err) { return Err(TyperError::match_non_exhaustive(*span).into()); }
                    Ok(res_ty_opt.unwrap_or(Type::Int))
                }
                other => {
                    Err(TyperError::match_invalid_scrutinee(other, *span).into())
                }
            }
        }
        Expr::Var(name, sp) => Ok(
            env
                .get(name.as_str())
                .cloned()
                .ok_or_else(|| TyperError::unknown_variable(name, *sp))?
        ),
        Expr::Return { expr, .. } => {
            let t = type_of(expr, env, fns, depth + 1)?;
            Ok(t)
        }
        Expr::Bin { op, lhs, rhs, span } => {
            let lt = type_of(lhs, env, fns, depth + 1)?;
            let rt = type_of(rhs, env, fns, depth + 1)?;
            ensure_int(lt, "left operand", Some(*span))?;
            ensure_int(rt, "right operand", Some(*span))?;
            match op {
                BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div => Ok(Type::Int),
            }
        }
        Expr::Call { callee, args, span } => {
            // Phase 4.6 — Collections signatures (type-only)
            if let Some(t) = type_collection_call(callee, args, env, fns, depth, *span)? {
                return Ok(t);
            }
            // Phase 4.5 — ADT constructors (partial): Some(T) infers Option<T>
            if callee == "Some" {
                if args.len() != 1 { return Err(TyperError::arity_mismatch(callee, 1, args.len(), *span).into()); }
                let t0 = type_of(&args[0], env, fns, depth + 1)?;
                return Ok(Type::Option(Box::new(t0)));
            }
            let (params, ret) = fns
                .get(callee.as_str())
                .cloned()
                .ok_or_else(|| TyperError::unknown_function(callee, *span))?;
            if params.len() != args.len() {
                return Err(TyperError::arity_mismatch(callee, params.len(), args.len(), *span).into());
            }
            for (i, (p, a)) in params.iter().zip(args.iter()).enumerate() {
                let at = type_of(a, env, fns, depth + 1)?;
                let expected = p.ty.clone();
                if expected != at {
                    let sp = match a {
                        Expr::Int(_, sp)
                        | Expr::Bool(_, sp)
                        | Expr::String(_, sp)
                        | Expr::Var(_, sp)
                        | Expr::Bin { span: sp, .. }
                        | Expr::Call { span: sp, .. }
                        | Expr::Match { span: sp, .. }
                        | Expr::Return { span: sp, .. }
                        | Expr::If { span: sp, .. } => *sp,
                    };
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

    match callee {
        // List
        "std::list::len" => {
            if args.len() != 1 { return Err(TyperError::arity_mismatch(callee, 1, args.len(), span).into()); }
            let lty = arg_ty(0)?;
            if let Type::List(_) = lty { Ok(Some(Type::Int)) } else { Err(TyperError::expected_collection("List", lty, span).into()) }
        }
        "std::list::get" => {
            if args.len() != 2 { return Err(TyperError::arity_mismatch(callee, 2, args.len(), span).into()); }
            let lty = arg_ty(0)?;
            let ity = arg_ty(1)?;
            if ity != Type::Int {
                let sp = match &args[1] { Expr::Int(_, s)|Expr::Bool(_, s)|Expr::String(_, s)|Expr::Var(_, s)|Expr::Bin{ span: s, .. }|Expr::Call{ span: s, .. }|Expr::Match{ span: s, .. }|Expr::Return{ span: s, .. }|Expr::If{ span: s, .. }=>*s };
                return Err(TyperError::int_operand("index", ity, Some(sp)).into());
            }
            match lty {
                Type::List(inner) => Ok(Some(Type::Option(inner))),
                other => Err(TyperError::expected_collection("List", other, span).into()),
            }
        }
        "std::list::push" => {
            if args.len() != 2 { return Err(TyperError::arity_mismatch(callee, 2, args.len(), span).into()); }
            let lty = arg_ty(0)?;
            match lty {
                Type::List(inner) => {
                    let letxty: Type = (*inner).clone();
                    let aty = arg_ty(1)?;
                    if aty != letxty { let sp = match &args[1] { Expr::Int(_, s)|Expr::Bool(_, s)|Expr::String(_, s)|Expr::Var(_, s)|Expr::Bin{ span: s, .. }|Expr::Call{ span: s, .. }|Expr::Match{ span: s, .. }|Expr::Return{ span: s, .. }|Expr::If{ span: s, .. }=>*s}; return Err(TyperError::element_type_mismatch(letxty, aty, sp).into()); }
                    Ok(Some(Type::List(Box::new(*inner))))
                }
                other => Err(TyperError::expected_collection("List", other, span).into()),
            }
        }
        "std::list::insert" => {
            if args.len() != 3 { return Err(TyperError::arity_mismatch(callee, 3, args.len(), span).into()); }
            let lty = arg_ty(0)?;
            match lty {
                Type::List(inner) => {
                    let elem_expected: Type = (*inner).clone();
                    let aty_elem = arg_ty(1)?;
                    if aty_elem != elem_expected {
                        let sp = match &args[1] { Expr::Int(_, s)|Expr::Bool(_, s)|Expr::String(_, s)|Expr::Var(_, s)|Expr::Bin{ span: s, .. }|Expr::Call{ span: s, .. }|Expr::Match{ span: s, .. }|Expr::Return{ span: s, .. }|Expr::If{ span: s, .. }=>*s };
                        return Err(TyperError::element_type_mismatch(elem_expected, aty_elem, sp).into());
                    }
                    let ity = arg_ty(2)?;
                    if ity != Type::Int {
                        let sp = match &args[2] { Expr::Int(_, s)|Expr::Bool(_, s)|Expr::String(_, s)|Expr::Var(_, s)|Expr::Bin{ span: s, .. }|Expr::Call{ span: s, .. }|Expr::Match{ span: s, .. }|Expr::Return{ span: s, .. }|Expr::If{ span: s, .. }=>*s };
                        return Err(TyperError::int_operand("index", ity, Some(sp)).into());
                    }
                    Ok(Some(Type::List(Box::new(*inner))))
                }
                other => Err(TyperError::expected_collection("List", other, span).into()),
            }
        }
        "std::list::remove" => {
            if args.len() != 2 { return Err(TyperError::arity_mismatch(callee, 2, args.len(), span).into()); }
            let lty = arg_ty(0)?;
            let ity = arg_ty(1)?;
            if ity != Type::Int {
                let sp = match &args[1] { Expr::Int(_, s)|Expr::Bool(_, s)|Expr::String(_, s)|Expr::Var(_, s)|Expr::Bin{ span: s, .. }|Expr::Call{ span: s, .. }|Expr::Match{ span: s, .. }|Expr::Return{ span: s, .. }|Expr::If{ span: s, .. }=>*s };
                return Err(TyperError::int_operand("index", ity, Some(sp)).into());
            }
            match lty {
                Type::List(inner) => Ok(Some(Type::List(inner))),
                other => Err(TyperError::expected_collection("List", other, span).into()),
            }
        }
        "std::list::pop" => {
            if args.len() != 1 { return Err(TyperError::arity_mismatch(callee, 1, args.len(), span).into()); }
            let lty = arg_ty(0)?;
            match lty {
                Type::List(inner) => Ok(Some(Type::Option(inner))),
                other => Err(TyperError::expected_collection("List", other, span).into()),
            }
        }
        "std::list::new" => {
            return Err(TyperError::cannot_infer_collection(span, "std::list").into());
        }

        // Set
        "std::set::len" => {
            if args.len() != 1 { return Err(TyperError::arity_mismatch(callee, 1, args.len(), span).into()); }
            let sty = arg_ty(0)?;
            if let Type::Set(_) = sty { Ok(Some(Type::Int)) } else { Err(TyperError::expected_collection("Set", sty, span).into()) }
        }
        "std::set::contains" => {
            if args.len() != 2 { return Err(TyperError::arity_mismatch(callee, 2, args.len(), span).into()); }
            match arg_ty(0)? {
                Type::Set(inner) => { let aty = arg_ty(1)?; if aty != *inner { let sp = match &args[1] { Expr::Int(_, s)|Expr::Bool(_, s)|Expr::String(_, s)|Expr::Var(_, s)|Expr::Bin{ span: s, .. }|Expr::Call{ span: s, .. }|Expr::Match{ span: s, .. }|Expr::Return{ span: s, .. }|Expr::If{ span: s, .. }=>*s}; return Err(TyperError::element_type_mismatch(*inner, aty, sp).into()); } Ok(Some(Type::Bool)) }
                other => Err(TyperError::expected_collection("Set", other, span).into()),
            }
        }
        "std::set::insert" | "std::set::remove" => {
            if args.len() != 2 { return Err(TyperError::arity_mismatch(callee, 2, args.len(), span).into()); }
            match arg_ty(0)? {
                Type::Set(inner) => { let aty = arg_ty(1)?; if aty != *inner { let sp = match &args[1] { Expr::Int(_, s)|Expr::Bool(_, s)|Expr::String(_, s)|Expr::Var(_, s)|Expr::Bin{ span: s, .. }|Expr::Call{ span: s, .. }|Expr::Match{ span: s, .. }|Expr::Return{ span: s, .. }|Expr::If{ span: s, .. }=>*s}; return Err(TyperError::element_type_mismatch(*inner, aty, sp).into()); } Ok(Some(Type::Set(inner))) }
                other => Err(TyperError::expected_collection("Set", other, span).into()),
            }
        }
        "std::set::new" => { return Err(TyperError::cannot_infer_collection(span, "std::set").into()); }

        // Map
        "std::map::len" => {
            if args.len() != 1 { return Err(TyperError::arity_mismatch(callee, 1, args.len(), span).into()); }
            let mty = arg_ty(0)?;
            if let Type::Map(_, _) = mty { Ok(Some(Type::Int)) } else { Err(TyperError::expected_collection("Map", mty, span).into()) }
        }
        "std::map::contains" => {
            if args.len() != 2 { return Err(TyperError::arity_mismatch(callee, 2, args.len(), span).into()); }
            match arg_ty(0)? {
                Type::Map(k, _v) => { let aty = arg_ty(1)?; if aty != *k { let sp = match &args[1] { Expr::Int(_, s)|Expr::Bool(_, s)|Expr::String(_, s)|Expr::Var(_, s)|Expr::Bin{ span: s, .. }|Expr::Call{ span: s, .. }|Expr::Match{ span: s, .. }|Expr::Return{ span: s, .. }|Expr::If{ span: s, .. }=>*s}; return Err(TyperError::element_type_mismatch(*k, aty, sp).into()); } Ok(Some(Type::Bool)) }
                other => Err(TyperError::expected_collection("Map", other, span).into()),
            }
        }
        "std::map::get" => {
            if args.len() != 2 { return Err(TyperError::arity_mismatch(callee, 2, args.len(), span).into()); }
            match arg_ty(0)? {
                Type::Map(k, v) => { let aty = arg_ty(1)?; if aty != *k { let sp = match &args[1] { Expr::Int(_, s)|Expr::Bool(_, s)|Expr::String(_, s)|Expr::Var(_, s)|Expr::Bin{ span: s, .. }|Expr::Call{ span: s, .. }|Expr::Match{ span: s, .. }|Expr::Return{ span: s, .. }|Expr::If{ span: s, .. }=>*s}; return Err(TyperError::element_type_mismatch(*k, aty, sp).into()); } Ok(Some(Type::Option(v))) }
                other => Err(TyperError::expected_collection("Map", other, span).into()),
            }
        }
        "std::map::insert" => {
            if args.len() != 3 { return Err(TyperError::arity_mismatch(callee, 3, args.len(), span).into()); }
            match arg_ty(0)? {
                Type::Map(k, v) => {
                    let aty_k = arg_ty(1)?; if aty_k != *k { let sp = match &args[1] { Expr::Int(_, s)|Expr::Bool(_, s)|Expr::String(_, s)|Expr::Var(_, s)|Expr::Bin{ span: s, .. }|Expr::Call{ span: s, .. }|Expr::Match{ span: s, .. }|Expr::Return{ span: s, .. }|Expr::If{ span: s, .. }=>*s}; return Err(TyperError::element_type_mismatch(*k, aty_k, sp).into()); }
                    let aty_v = arg_ty(2)?; if aty_v != *v { let sp = match &args[2] { Expr::Int(_, s)|Expr::Bool(_, s)|Expr::String(_, s)|Expr::Var(_, s)|Expr::Bin{ span: s, .. }|Expr::Call{ span: s, .. }|Expr::Match{ span: s, .. }|Expr::Return{ span: s, .. }|Expr::If{ span: s, .. }=>*s}; return Err(TyperError::element_type_mismatch(*v, aty_v, sp).into()); }
                    Ok(Some(Type::Map(k, v)))
                }
                other => Err(TyperError::expected_collection("Map", other, span).into()),
            }
        }
        "std::map::remove" => {
            if args.len() != 2 { return Err(TyperError::arity_mismatch(callee, 2, args.len(), span).into()); }
            match arg_ty(0)? {
                Type::Map(k, v) => { let aty = arg_ty(1)?; if aty != *k { let sp = match &args[1] { Expr::Int(_, s)|Expr::Bool(_, s)|Expr::String(_, s)|Expr::Var(_, s)|Expr::Bin{ span: s, .. }|Expr::Call{ span: s, .. }|Expr::Match{ span: s, .. }|Expr::Return{ span: s, .. }|Expr::If{ span: s, .. }=>*s}; return Err(TyperError::element_type_mismatch(*k, aty, sp).into()); } Ok(Some(Type::Map(k, v))) }
                other => Err(TyperError::expected_collection("Map", other, span).into()),
            }
        }
        "std::map::new" => { return Err(TyperError::cannot_infer_collection(span, "std::map").into()); }

        _ => Ok(None),
    }
}

fn ensure_int(ty: Type, what: &str, span: Option<Span>) -> Result<()> {
    if ty != Type::Int {
        return Err(TyperError::int_operand(what, ty, span).into());
    }
    Ok(())
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
