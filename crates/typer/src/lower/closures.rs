use crate::check::{base_type, infer_expr_type, LocalBinding};
use anyhow::Result;
use clg_ast::{Block, Expr, LambdaParam, MatchPat, ParamKind, Stmt, Type};
use clg_ir::{Function as IrFunction, Instr, IrType, Value};
use std::collections::{HashMap, HashSet};

use super::layout::tuple_layout;
use super::{
    emit_alloc, emit_int_const, emit_store_i32, ir_ty, load_value_borrow, lower_expr, store_value,
    DispatcherSignature, LambdaDispatchCase, LowerCtx, CLOSURE_CODE_ID_OFFSET,
    CLOSURE_ENV_PTR_OFFSET, CLOSURE_RECORD_ALIGN, CLOSURE_RECORD_SIZE,
};

fn name_is_bound(name: &str, scopes: &[HashSet<&str>]) -> bool {
    scopes.iter().rev().any(|scope| scope.contains(name))
}

fn record_capture<'a>(name: &'a str, ordered: &mut Vec<&'a str>, seen: &mut HashSet<&'a str>) {
    if seen.insert(name) {
        ordered.push(name);
    }
}

fn collect_lambda_captures_expr<'a>(
    expr: &'a Expr,
    env: &HashMap<&'a str, Value>,
    scopes: &mut Vec<HashSet<&'a str>>,
    ordered: &mut Vec<&'a str>,
    seen: &mut HashSet<&'a str>,
) {
    match expr {
        Expr::Int(_, _) | Expr::Bool(_, _) | Expr::String(_, _) => {}
        Expr::Var(name, _) => {
            let name = name.as_str();
            if !name_is_bound(name, scopes) && env.contains_key(name) {
                record_capture(name, ordered, seen);
            }
        }
        Expr::Call { callee, args, .. } => {
            let callee = callee.as_str();
            if !callee.contains("::") && !name_is_bound(callee, scopes) && env.contains_key(callee)
            {
                record_capture(callee, ordered, seen);
            }
            for arg in args {
                collect_lambda_captures_expr(arg, env, scopes, ordered, seen);
            }
        }
        Expr::ArrayLit { elems, .. } | Expr::TupleLit { elems, .. } => {
            for elem in elems {
                collect_lambda_captures_expr(elem, env, scopes, ordered, seen);
            }
        }
        Expr::StructLit { fields, .. } => {
            for field in fields {
                collect_lambda_captures_expr(&field.expr, env, scopes, ordered, seen);
            }
        }
        Expr::FieldAccess { base, .. } => {
            collect_lambda_captures_expr(base, env, scopes, ordered, seen);
        }
        Expr::Index { base, index, .. } => {
            collect_lambda_captures_expr(base, env, scopes, ordered, seen);
            collect_lambda_captures_expr(index, env, scopes, ordered, seen);
        }
        Expr::Block { block } => {
            collect_lambda_captures_block(block, env, scopes, ordered, seen);
        }
        Expr::Bin { lhs, rhs, .. } => {
            collect_lambda_captures_expr(lhs, env, scopes, ordered, seen);
            collect_lambda_captures_expr(rhs, env, scopes, ordered, seen);
        }
        Expr::Return { expr, .. } | Expr::Unary { expr, .. } | Expr::Try { expr, .. } => {
            collect_lambda_captures_expr(expr, env, scopes, ordered, seen);
        }
        Expr::If {
            cond,
            then_br,
            else_br,
            ..
        } => {
            collect_lambda_captures_expr(cond, env, scopes, ordered, seen);
            collect_lambda_captures_expr(then_br, env, scopes, ordered, seen);
            collect_lambda_captures_expr(else_br, env, scopes, ordered, seen);
        }
        Expr::Match {
            scrutinee, arms, ..
        } => {
            collect_lambda_captures_expr(scrutinee, env, scopes, ordered, seen);
            for arm in arms {
                scopes.push(HashSet::new());
                if let Some(scope) = scopes.last_mut() {
                    match &arm.pat {
                        MatchPat::Some(name) | MatchPat::Ok(name) | MatchPat::Err(name) => {
                            scope.insert(name.as_str());
                        }
                        MatchPat::EnumVariant { binders, .. } => {
                            for binder in binders {
                                scope.insert(binder.as_str());
                            }
                        }
                        MatchPat::None | MatchPat::Wildcard => {}
                    }
                }
                collect_lambda_captures_expr(&arm.expr, env, scopes, ordered, seen);
                scopes.pop();
            }
        }
        Expr::Lambda { params, body, .. } => {
            scopes.push(HashSet::new());
            if let Some(scope) = scopes.last_mut() {
                for param in params {
                    scope.insert(param.name.as_str());
                }
            }
            collect_lambda_captures_expr(body, env, scopes, ordered, seen);
            scopes.pop();
        }
    }
}

fn collect_lambda_captures_block<'a>(
    block: &'a Block,
    env: &HashMap<&'a str, Value>,
    scopes: &mut Vec<HashSet<&'a str>>,
    ordered: &mut Vec<&'a str>,
    seen: &mut HashSet<&'a str>,
) {
    scopes.push(HashSet::new());
    for stmt in &block.statements {
        match stmt {
            Stmt::Let { name, expr, .. } => {
                collect_lambda_captures_expr(expr, env, scopes, ordered, seen);
                if let Some(scope) = scopes.last_mut() {
                    scope.insert(name.as_str());
                }
            }
            Stmt::Expr { expr, .. } => {
                collect_lambda_captures_expr(expr, env, scopes, ordered, seen);
            }
            Stmt::While {
                cond,
                invariant,
                variant,
                body,
                ..
            } => {
                collect_lambda_captures_expr(cond, env, scopes, ordered, seen);
                collect_lambda_captures_expr(invariant, env, scopes, ordered, seen);
                if let Some(v) = variant {
                    collect_lambda_captures_expr(v, env, scopes, ordered, seen);
                }
                collect_lambda_captures_block(body, env, scopes, ordered, seen);
            }
        }
    }
    if let Some(tail) = &block.tail {
        collect_lambda_captures_expr(tail, env, scopes, ordered, seen);
    }
    scopes.pop();
}

fn collect_lambda_captures<'a>(
    params: &'a [LambdaParam],
    body: &'a Expr,
    env: &HashMap<&'a str, Value>,
) -> Vec<&'a str> {
    let mut scopes = vec![params
        .iter()
        .map(|param| param.name.as_str())
        .collect::<HashSet<_>>()];
    let mut ordered: Vec<&'a str> = Vec::new();
    let mut seen: HashSet<&'a str> = HashSet::new();
    collect_lambda_captures_expr(body, env, &mut scopes, &mut ordered, &mut seen);
    ordered
}

pub(super) fn lower_lambda_expr<'a>(
    ctx: &mut LowerCtx<'a>,
    lambda_expr: &'a Expr,
    expected: Option<&Type>,
) -> Result<Value> {
    let Expr::Lambda { params, body, .. } = lambda_expr else {
        anyhow::bail!("internal error: lower_lambda_expr called for non-lambda expression");
    };
    let captures = collect_lambda_captures(params, body, &ctx.env);
    let mut capture_tys = Vec::with_capacity(captures.len());
    let mut capture_vals = Vec::with_capacity(captures.len());
    for name in &captures {
        let binding = ctx
            .type_env
            .get(name)
            .ok_or_else(|| anyhow::anyhow!("missing capture type for `{}`", name))?;
        let value = ctx
            .env
            .get(name)
            .copied()
            .ok_or_else(|| anyhow::anyhow!("missing capture value for `{}`", name))?;
        capture_tys.push(binding.ty.clone());
        capture_vals.push(value);
    }

    let capture_layout = if capture_tys.is_empty() {
        None
    } else {
        Some(tuple_layout(&capture_tys, ctx.aliases, ctx.std_types)?)
    };
    let env_ptr = if let Some(layout) = &capture_layout {
        let ptr = emit_alloc(ctx, layout.size, layout.align);
        for (i, ty) in capture_tys.iter().enumerate() {
            let offset = *layout
                .offsets
                .get(i)
                .ok_or_else(|| anyhow::anyhow!("missing capture offset {}", i))?;
            store_value(ctx, ty, ptr, offset, capture_vals[i])?;
        }
        ptr
    } else {
        emit_int_const(ctx, 0)
    };

    let code_id_num = ctx.next_lambda_code_id();
    let code_id = emit_int_const(ctx, code_id_num as i64);
    let closure_ptr = emit_alloc(ctx, CLOSURE_RECORD_SIZE, CLOSURE_RECORD_ALIGN);
    emit_store_i32(ctx, closure_ptr, CLOSURE_CODE_ID_OFFSET, code_id);
    emit_store_i32(ctx, closure_ptr, CLOSURE_ENV_PTR_OFFSET, env_ptr);

    let (sig_params, sig_ret) = if let Some(expected_ty) = expected {
        match base_type(expected_ty, ctx.aliases)? {
            Type::Fn { params, ret } => (params, (*ret).clone()),
            _ => {
                let inferred = infer_expr_type(
                    lambda_expr,
                    &ctx.type_env,
                    &ctx.fns,
                    ctx.trait_env,
                    ctx.aliases,
                    ctx.type_defs,
                    &ctx.type_params,
                    &ctx.bounds,
                )?;
                match base_type(&inferred, ctx.aliases)? {
                    Type::Fn { params, ret } => (params, (*ret).clone()),
                    _ => anyhow::bail!("lambda expression did not infer to function type"),
                }
            }
        }
    } else {
        let inferred = infer_expr_type(
            lambda_expr,
            &ctx.type_env,
            &ctx.fns,
            ctx.trait_env,
            ctx.aliases,
            ctx.type_defs,
            &ctx.type_params,
            &ctx.bounds,
        )?;
        match base_type(&inferred, ctx.aliases)? {
            Type::Fn { params, ret } => (params, (*ret).clone()),
            _ => anyhow::bail!("lambda expression did not infer to function type"),
        }
    };
    let lambda_name = format!("__clg_lambda_{code_id_num}");
    let signature = DispatcherSignature {
        params: sig_params.clone(),
        ret: sig_ret.clone(),
    };
    ctx.lambda_cases.push(LambdaDispatchCase {
        code_id: code_id_num,
        function_name: lambda_name.clone(),
        signature,
    });

    let mut lambda_env: HashMap<&str, Value> = HashMap::new();
    let mut lambda_type_env: HashMap<&str, LocalBinding> = HashMap::new();
    lambda_type_env.insert(
        "$return",
        LocalBinding {
            ty: sig_ret.clone(),
            kind: ParamKind::Borrow,
        },
    );
    for (i, param) in params.iter().enumerate() {
        let param_val = Value(1 + i as u32);
        lambda_env.insert(param.name.as_str(), param_val);
        lambda_type_env.insert(
            param.name.as_str(),
            LocalBinding {
                ty: param.ty.clone(),
                kind: ParamKind::Borrow,
            },
        );
    }

    let mut lambda_ctx = LowerCtx {
        next: (1 + params.len()) as u32,
        env: lambda_env,
        type_env: lambda_type_env,
        fns: ctx.fns.clone(),
        fn_indices: ctx.fn_indices.clone(),
        aliases: ctx.aliases,
        trait_env: ctx.trait_env,
        type_defs: ctx.type_defs,
        std_types: ctx.std_types,
        type_params: ctx.type_params.clone(),
        bounds: ctx.bounds.clone(),
        body: Vec::new(),
        ret_ty: sig_ret.clone(),
        next_closure_code_id: ctx.next_closure_code_id,
        function_name: lambda_name.clone(),
        generated_functions: Vec::new(),
        lambda_cases: Vec::new(),
        dispatcher_patches: Vec::new(),
    };

    if let Some(layout) = &capture_layout {
        for (i, capture_name) in captures.iter().enumerate() {
            let capture_ty = capture_tys
                .get(i)
                .ok_or_else(|| anyhow::anyhow!("missing capture type {}", i))?;
            let offset = *layout
                .offsets
                .get(i)
                .ok_or_else(|| anyhow::anyhow!("missing capture offset {}", i))?;
            let capture_val = load_value_borrow(&mut lambda_ctx, capture_ty, Value(0), offset)?;
            lambda_ctx.env.insert(capture_name, capture_val);
            lambda_ctx.type_env.insert(
                capture_name,
                LocalBinding {
                    ty: capture_ty.clone(),
                    kind: ParamKind::Borrow,
                },
            );
        }
    }

    let body_val = lower_expr(&mut lambda_ctx, body.as_ref(), Some(sig_ret.clone()))?;
    lambda_ctx.body.push(Instr::Ret { val: body_val });
    ctx.next_closure_code_id = lambda_ctx.next_closure_code_id;

    ctx.generated_functions.push(IrFunction {
        name: lambda_name,
        params: {
            let mut out = Vec::with_capacity(1 + params.len());
            out.push(IrType::Int); // hidden env_ptr
            out.extend(params.iter().map(|param| ir_ty(param.ty.clone())));
            out
        },
        ret: Some(ir_ty(sig_ret)),
        body: lambda_ctx.body,
    });
    ctx.generated_functions
        .extend(lambda_ctx.generated_functions);
    ctx.lambda_cases.extend(lambda_ctx.lambda_cases);
    ctx.dispatcher_patches.extend(lambda_ctx.dispatcher_patches);

    Ok(closure_ptr)
}
