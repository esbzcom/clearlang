use anyhow::Result;
use clg_ast::{Block, Expr, LambdaParam, MatchPat, Stmt};
use std::collections::{HashMap, HashSet};

use super::layout::tuple_layout;
use super::{
    emit_alloc, emit_int_const, emit_store_i32, store_value, LowerCtx, CLOSURE_CODE_ID_OFFSET,
    CLOSURE_ENV_PTR_OFFSET, CLOSURE_RECORD_ALIGN, CLOSURE_RECORD_SIZE,
};
use clg_ir::Value;

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
    params: &'a [LambdaParam],
    body: &'a Expr,
) -> Result<Value> {
    let captures = collect_lambda_captures(params, body, &ctx.env);
    let env_ptr = if captures.is_empty() {
        emit_int_const(ctx, 0)
    } else {
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
        let layout = tuple_layout(&capture_tys, ctx.aliases, ctx.std_types)?;
        let ptr = emit_alloc(ctx, layout.size, layout.align);
        for (i, ty) in capture_tys.iter().enumerate() {
            let offset = *layout
                .offsets
                .get(i)
                .ok_or_else(|| anyhow::anyhow!("missing capture offset {}", i))?;
            store_value(ctx, ty, ptr, offset, capture_vals[i])?;
        }
        ptr
    };

    let code_id_num = ctx.next_lambda_code_id() as i64;
    let code_id = emit_int_const(ctx, code_id_num);
    let closure_ptr = emit_alloc(ctx, CLOSURE_RECORD_SIZE, CLOSURE_RECORD_ALIGN);
    emit_store_i32(ctx, closure_ptr, CLOSURE_CODE_ID_OFFSET, code_id);
    emit_store_i32(ctx, closure_ptr, CLOSURE_ENV_PTR_OFFSET, env_ptr);

    // Closure invocation dispatch is handled in Phase 17.7.4.3.
    Ok(closure_ptr)
}
