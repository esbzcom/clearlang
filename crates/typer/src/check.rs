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
            Expr::Bin { span, .. } | Expr::Call { span, .. } | Expr::Match { span, .. } | Expr::Return { span, .. } => *span,
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
        Expr::Match { scrutinee, arms, span } => {
            let scrut_ty = type_of(scrutinee, env, fns, depth + 1)?;
            // Helper to get arm expr span
            let span_of = |ex: &Expr| -> Span {
                match ex {
                    Expr::Int(_, sp) | Expr::Bool(_, sp) | Expr::String(_, sp) | Expr::Var(_, sp) => *sp,
                    Expr::Bin { span, .. } | Expr::Call { span, .. } | Expr::Match { span, .. } | Expr::Return { span, .. } => *span,
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
            // Phase 4.3A — Collections (strict errors): emit a single friendly error
            if callee.starts_with("std::list::") || callee.starts_with("std::set::") || callee.starts_with("std::map::") {
                return Err(TyperError::collections_unavailable(callee, *span).into());
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
                        | Expr::Return { span: sp, .. } => *sp,
                    };
                    return Err(TyperError::arg_type_mismatch(i, callee, expected, at, sp).into());
                }
            }
            Ok(ret)
        }
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
    }
}
