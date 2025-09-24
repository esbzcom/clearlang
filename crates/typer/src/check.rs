use crate::builtins::builtin_sigs;
use crate::errors::TyperError;
use crate::lower::lower_func;
use crate::vc::{generate_vcs, VerificationCondition};
use anyhow::{Context, Result};
use clg_ast::{BinOp, Effect, Expr, Func, Param, Program, Span, Type, UnaryOp};
use clg_ir::Module;
use std::collections::{HashMap, HashSet};

type FnSig<'a> = (&'a [Param], Type);

pub struct TypecheckOutput {
    pub ir: Module,
    pub vcs: Vec<VerificationCondition>,
}

pub fn check_with_vcs(ast: &Program) -> Result<TypecheckOutput> {
    let mut fns: HashMap<&str, FnSig> = HashMap::new();

    let builtins = builtin_sigs();
    for (name, params, ret) in &builtins {
        fns.insert(name.as_str(), (&params[..], ret.clone()));
    }

    for f in &ast.funcs {
        if fns
            .insert(f.name.as_str(), (&f.params, f.ret.clone()))
            .is_some()
        {
            return Err(TyperError::duplicate_function(&f.name).into());
        }
    }

    for f in &ast.funcs {
        check_func(f, &fns).with_context(|| format!("in function `{}`", f.name))?;
    }

    // Collect used intrinsics
    let used_intrinsics = collect_used_intrinsics(ast);

    // Order of function indices: all user-defined first, then intrinsics used (stable order)
    let mut fn_indices: HashMap<&str, u32> = HashMap::new();
    for (i, f) in ast.funcs.iter().enumerate() {
        fn_indices.insert(f.name.as_str(), i as u32);
    }
    // Stable intrinsic order
    let intrinsic_order = ["std::str::len", "std::str::eq", "std::str::concat"];
    let mut intrinsic_defs: Vec<clg_ir::Function> = Vec::new();
    for name in intrinsic_order.iter() {
        if used_intrinsics.contains(*name) {
            let idx = (ast.funcs.len() + intrinsic_defs.len()) as u32;
            fn_indices.insert(name, idx);
            // Define IR function signature for the intrinsic
            let (params, ret) = match *name {
                "std::str::len" => (vec![clg_ir::IrType::Int], Some(clg_ir::IrType::Int)),
                "std::str::eq" => (
                    vec![clg_ir::IrType::Int, clg_ir::IrType::Int],
                    Some(clg_ir::IrType::Bool),
                ),
                "std::str::concat" => (
                    vec![clg_ir::IrType::Int, clg_ir::IrType::Int],
                    Some(clg_ir::IrType::Int),
                ),
                _ => (vec![], None),
            };
            intrinsic_defs.push(clg_ir::Function {
                name: (*name).to_string(),
                params,
                ret,
                body: vec![],
            });
        }
    }

    let mut module = Module::default();
    for f in &ast.funcs {
        module.funcs.push(lower_func(f, &fns, &fn_indices)?);
    }
    // Append intrinsic function declarations at the end
    module.funcs.extend(intrinsic_defs.into_iter());
    let vcs = generate_vcs(ast);
    Ok(TypecheckOutput { ir: module, vcs })
}

pub fn check(ast: &Program) -> Result<Module> {
    Ok(check_with_vcs(ast)?.ir)
}

pub fn type_check_only(ast: &Program) -> Result<()> {
    let mut fns: HashMap<&str, FnSig> = HashMap::new();

    let builtins = builtin_sigs();
    for (name, params, ret) in &builtins {
        fns.insert(name.as_str(), (&params[..], ret.clone()));
    }

    for f in &ast.funcs {
        if fns
            .insert(f.name.as_str(), (&f.params, f.ret.clone()))
            .is_some()
        {
            return Err(TyperError::duplicate_function(&f.name).into());
        }
    }

    for f in &ast.funcs {
        check_func(f, &fns).with_context(|| format!("in function `{}`", f.name))?;
    }
    Ok(())
}

fn check_func<'a>(f: &'a Func, fns: &HashMap<&'a str, FnSig<'a>>) -> Result<()> {
    let mut env: HashMap<&str, Type> = HashMap::new();
    for p in &f.params {
        if env.insert(p.name.as_str(), p.ty.clone()).is_some() {
            return Err(TyperError::duplicate_parameter(&p.name).into());
        }
    }

    match f.effect {
        Effect::None | Effect::Pure => {}
        Effect::Mut | Effect::Io => return Err(TyperError::effect_not_supported(f.effect).into()),
    }

    for req in &f.requires {
        let ty = type_of(&req.expr, &env, fns, 0)?;
        if ty != Type::Bool {
            return Err(TyperError::contract_not_bool("require", ty, req.span).into());
        }
    }

    let mut ensure_env = env.clone();
    if !ensure_env.contains_key("result") {
        ensure_env.insert("result", f.ret.clone());
    }
    for ens in &f.ensures {
        let ty = type_of(&ens.expr, &ensure_env, fns, 0)?;
        if ty != Type::Bool {
            return Err(TyperError::contract_not_bool("ensure", ty, ens.span).into());
        }
    }

    let body_ty = type_of(&f.body, &env, fns, 0)?;
    if body_ty != f.ret {
        let sp = expr_span(&f.body);
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
            let (params, ret) = fns
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

fn collect_used_intrinsics(ast: &Program) -> HashSet<&'static str> {
    let mut set: HashSet<&'static str> = HashSet::new();
    fn walk_expr<'a>(e: &'a Expr, set: &mut HashSet<&'static str>) {
        match e {
            Expr::Int(_, _) | Expr::Bool(_, _) | Expr::String(_, _) | Expr::Var(_, _) => {}
            Expr::Return { expr, .. } => walk_expr(expr, set),
            Expr::Unary { expr, .. } => walk_expr(expr, set),
            Expr::Bin { lhs, rhs, .. } => {
                walk_expr(lhs, set);
                walk_expr(rhs, set);
            }
            Expr::If {
                cond,
                then_br,
                else_br,
                ..
            } => {
                walk_expr(cond, set);
                walk_expr(then_br, set);
                walk_expr(else_br, set);
            }
            Expr::Match {
                scrutinee, arms, ..
            } => {
                walk_expr(scrutinee, set);
                for arm in arms {
                    walk_expr(&arm.expr, set);
                }
            }
            Expr::Call { callee, args, .. } => {
                match callee.as_str() {
                    "std::str::len" => {
                        set.insert("std::str::len");
                    }
                    "std::str::eq" => {
                        set.insert("std::str::eq");
                    }
                    "std::str::concat" => {
                        set.insert("std::str::concat");
                    }
                    _ => {}
                }
                for a in args {
                    walk_expr(a, set);
                }
            }
        }
    }
    for f in &ast.funcs {
        walk_expr(&f.body, &mut set);
    }
    set
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
        "std::list::new" => {
            return Err(TyperError::cannot_infer_collection(span, "std::list").into());
        }

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
        "std::set::new" => {
            return Err(TyperError::cannot_infer_collection(span, "std::set").into());
        }

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
        "std::map::new" => {
            return Err(TyperError::cannot_infer_collection(span, "std::map").into());
        }

        _ => Ok(None),
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

fn expr_span(e: &Expr) -> Span {
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
