use anyhow::{bail, Context, Result};
use lumi_ast::{BinOp, Effect, Expr, Func, Param, Program, Type};
use lumi_ir::PlaceHolder;
use std::collections::HashMap;

type FnSig<'a> = (&'a [Param], Type);

/// Type-check the Lumi AST and return a placeholder IR on success.
/// Phase 3.1: checks Int/Bool, variables, calls, binops, arity, and returns.
pub fn check(ast: &Program) -> Result<PlaceHolder> {
    let mut fns: HashMap<&str, FnSig> = HashMap::new();
    for f in &ast.funcs {
        if fns.insert(f.name.as_str(), (&f.params, f.ret)).is_some() {
            bail!("duplicate function `{}`", f.name);
        }
    }

    for f in &ast.funcs {
        check_func(f, &fns).with_context(|| format!("in function `{}`", f.name))?;
    }

    Ok(PlaceHolder)
}

fn check_func<'a>(f: &'a Func, fns: &HashMap<&'a str, FnSig<'a>>) -> Result<()> {
    // Effects stub (Phase 3.2): accept None|Pure; reject Mut/Io for now
    match f.effect {
        Effect::None | Effect::Pure => {}
        Effect::Mut | Effect::Io => bail!(
            "effect `{}` not supported yet; use `pure` or omit",
            match f.effect { Effect::Mut => "mut", Effect::Io => "io", _ => unreachable!() }
        ),
    }
    let mut env: HashMap<&str, Type> = HashMap::new();
    for p in &f.params {
        if env.insert(p.name.as_str(), p.ty).is_some() {
            bail!("duplicate parameter `{}`", p.name);
        }
    }

    let body_ty = type_of(&f.body, &env, fns, 0)?;
    if body_ty != f.ret {
        bail!(
            "return type mismatch: declared `{}`, found `{}`",
            show_ty(f.ret),
            show_ty(body_ty)
        );
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
        bail!("type-check recursion limit exceeded");
    }
    match e {
        Expr::Int(_) => Ok(Type::Int),
        Expr::Bool(_) => Ok(Type::Bool),
        Expr::Var(name) => env
            .get(name.as_str())
            .copied()
            .with_context(|| format!("unknown variable `{}`", name)),
        Expr::Bin { op, lhs, rhs } => {
            let lt = type_of(lhs, env, fns, depth + 1)?;
            let rt = type_of(rhs, env, fns, depth + 1)?;
            ensure_int(lt, "left operand")?;
            ensure_int(rt, "right operand")?;
            match op {
                BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div => Ok(Type::Int),
            }
        }
        Expr::Call { callee, args } => {
            let (params, ret) = fns
                .get(callee.as_str())
                .copied()
                .with_context(|| format!("unknown function `{}`", callee))?;
            if params.len() != args.len() {
                bail!(
                    "arity mismatch calling `{}`: expected {}, found {}",
                    callee,
                    params.len(),
                    args.len()
                );
            }
            for (i, (p, a)) in params.iter().zip(args.iter()).enumerate() {
                let at = type_of(a, env, fns, depth + 1)?;
                if p.ty != at {
                    bail!(
                        "arg {} type mismatch calling `{}`: expected `{}`, found `{}`",
                        i,
                        callee,
                        show_ty(p.ty),
                        show_ty(at)
                    );
                }
            }
            Ok(ret)
        }
    }
}

fn ensure_int(ty: Type, what: &str) -> Result<()> {
    if ty != Type::Int {
        bail!("{} must be Int, found `{}`", what, show_ty(ty));
    }
    Ok(())
}

fn show_ty(t: Type) -> &'static str {
    match t {
        Type::Int => "Int",
        Type::Bool => "Bool",
    }
}
