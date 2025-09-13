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
            Expr::Bin { span, .. } | Expr::Call { span, .. } | Expr::Match { span, .. } => *span,
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
        Expr::Match { span, .. } => {
            return Err(TyperError::new("T012", "match not supported in typer yet".to_string(), span.start, span.end).into());
        }
        Expr::Var(name, sp) => Ok(
            env
                .get(name.as_str())
                .cloned()
                .ok_or_else(|| TyperError::unknown_variable(name, *sp))?
        ),
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
                        | Expr::Match { span: sp, .. } => *sp,
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
