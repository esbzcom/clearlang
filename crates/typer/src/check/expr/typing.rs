use anyhow::Result;
use clg_ast::{Block, Expr, MatchPat, Span, Stmt, Type};
use std::collections::{HashMap, HashSet};

use super::super::{
    base_type, is_resource_type, AliasMap, BoundsMap, FnSig, LocalBinding, TraitEnv, TypeDefs,
    RETURN_KEY,
};
use super::block::type_block;
use super::call_expr::type_call_expr;
use super::match_expr::type_match_expr;
use super::ops::{type_bin_expr, type_unary_expr};
use super::ResourceTracker;
use crate::errors::TyperError;

mod handlers;
use self::handlers::{
    type_array_lit, type_field_access, type_index_expr, type_lambda_expr, type_struct_lit,
    type_try_expr, type_tuple_lit,
};
pub(crate) fn consume_var_expr(tracker: &mut ResourceTracker, expr: &Expr) -> Result<()> {
    if let Expr::Var(name, span) = expr {
        tracker.consume_var(name, *span)?;
    }
    Ok(())
}

fn name_is_bound(name: &str, scopes: &[HashSet<String>]) -> bool {
    scopes.iter().rev().any(|scope| scope.contains(name))
}

fn record_capture(name: &str, span: Span, captures: &mut HashMap<String, Span>) {
    captures.entry(name.to_string()).or_insert(span);
}

fn collect_lambda_captures_expr(
    expr: &Expr,
    scopes: &mut Vec<HashSet<String>>,
    captures: &mut HashMap<String, Span>,
) {
    match expr {
        Expr::Int(_, _) | Expr::Bool(_, _) | Expr::String(_, _) => {}
        Expr::Var(name, span) => {
            if !name_is_bound(name, scopes) {
                record_capture(name, *span, captures);
            }
        }
        Expr::Call {
            callee,
            args,
            type_args: _,
            span,
        } => {
            if !callee.contains("::") && !name_is_bound(callee, scopes) {
                record_capture(callee, *span, captures);
            }
            for arg in args {
                collect_lambda_captures_expr(arg, scopes, captures);
            }
        }
        Expr::ArrayLit { elems, .. } | Expr::TupleLit { elems, .. } => {
            for elem in elems {
                collect_lambda_captures_expr(elem, scopes, captures);
            }
        }
        Expr::StructLit { fields, .. } => {
            for field in fields {
                collect_lambda_captures_expr(&field.expr, scopes, captures);
            }
        }
        Expr::FieldAccess { base, .. } => collect_lambda_captures_expr(base, scopes, captures),
        Expr::Index { base, index, .. } => {
            collect_lambda_captures_expr(base, scopes, captures);
            collect_lambda_captures_expr(index, scopes, captures);
        }
        Expr::Block { block } => collect_lambda_captures_block(block, scopes, captures),
        Expr::Bin { lhs, rhs, .. } => {
            collect_lambda_captures_expr(lhs, scopes, captures);
            collect_lambda_captures_expr(rhs, scopes, captures);
        }
        Expr::Return { expr, .. } | Expr::Unary { expr, .. } | Expr::Try { expr, .. } => {
            collect_lambda_captures_expr(expr, scopes, captures)
        }
        Expr::If {
            cond,
            then_br,
            else_br,
            ..
        } => {
            collect_lambda_captures_expr(cond, scopes, captures);
            collect_lambda_captures_expr(then_br, scopes, captures);
            collect_lambda_captures_expr(else_br, scopes, captures);
        }
        Expr::Match {
            scrutinee, arms, ..
        } => {
            collect_lambda_captures_expr(scrutinee, scopes, captures);
            for arm in arms {
                scopes.push(HashSet::new());
                if let Some(scope) = scopes.last_mut() {
                    match &arm.pat {
                        MatchPat::Some(name) | MatchPat::Ok(name) | MatchPat::Err(name) => {
                            scope.insert(name.clone());
                        }
                        MatchPat::EnumVariant { binders, .. } => {
                            for binder in binders {
                                scope.insert(binder.clone());
                            }
                        }
                        MatchPat::None | MatchPat::Wildcard => {}
                    }
                }
                collect_lambda_captures_expr(&arm.expr, scopes, captures);
                scopes.pop();
            }
        }
        Expr::Lambda { params, body, .. } => {
            scopes.push(HashSet::new());
            if let Some(scope) = scopes.last_mut() {
                for param in params {
                    scope.insert(param.name.clone());
                }
            }
            collect_lambda_captures_expr(body, scopes, captures);
            scopes.pop();
        }
    }
}

fn collect_lambda_captures_block(
    block: &Block,
    scopes: &mut Vec<HashSet<String>>,
    captures: &mut HashMap<String, Span>,
) {
    scopes.push(HashSet::new());
    for stmt in &block.statements {
        match stmt {
            Stmt::Let { name, expr, .. } => {
                collect_lambda_captures_expr(expr, scopes, captures);
                if let Some(scope) = scopes.last_mut() {
                    scope.insert(name.clone());
                }
            }
            Stmt::Expr { expr, .. } => collect_lambda_captures_expr(expr, scopes, captures),
            Stmt::While {
                cond,
                invariant,
                variant,
                body,
                ..
            } => {
                collect_lambda_captures_expr(cond, scopes, captures);
                collect_lambda_captures_expr(invariant, scopes, captures);
                if let Some(v) = variant {
                    collect_lambda_captures_expr(v, scopes, captures);
                }
                collect_lambda_captures_block(body, scopes, captures);
            }
        }
    }
    if let Some(tail) = &block.tail {
        collect_lambda_captures_expr(tail, scopes, captures);
    }
    scopes.pop();
}

fn collect_lambda_captures(
    params: &[clg_ast::LambdaParam],
    body: &Expr,
    env: &HashMap<&str, LocalBinding>,
) -> HashMap<String, Span> {
    let mut scopes: Vec<HashSet<String>> = Vec::with_capacity(4);
    scopes.push(
        params
            .iter()
            .map(|p| p.name.clone())
            .collect::<HashSet<_>>(),
    );
    let mut captures: HashMap<String, Span> = HashMap::new();
    collect_lambda_captures_expr(body, &mut scopes, &mut captures);
    captures.retain(|name, _| env.contains_key(name.as_str()));
    captures
}

pub(crate) fn type_of<'a>(
    e: &'a Expr,
    env: &HashMap<&'a str, LocalBinding>,
    tracker: &mut ResourceTracker,
    fns: &HashMap<&'a str, FnSig>,
    trait_env: &TraitEnv<'a>,
    aliases: &AliasMap,
    type_defs: &TypeDefs,
    type_params: &HashSet<String>,
    bounds: &BoundsMap,
    depth: usize,
    expected: Option<&Type>,
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
        Expr::ArrayLit { elems, span } => type_array_lit(
            elems,
            *span,
            expected,
            env,
            tracker,
            fns,
            trait_env,
            aliases,
            type_defs,
            type_params,
            bounds,
            depth,
        ),
        Expr::TupleLit { elems, span } => type_tuple_lit(
            elems,
            *span,
            expected,
            env,
            tracker,
            fns,
            trait_env,
            aliases,
            type_defs,
            type_params,
            bounds,
            depth,
        ),
        Expr::StructLit { name, fields, span } => type_struct_lit(
            name.as_str(),
            fields,
            *span,
            env,
            tracker,
            fns,
            trait_env,
            aliases,
            type_defs,
            type_params,
            bounds,
            depth,
        ),
        Expr::FieldAccess { base, field, span } => type_field_access(
            base,
            field.as_str(),
            *span,
            env,
            tracker,
            fns,
            trait_env,
            aliases,
            type_defs,
            type_params,
            bounds,
            depth,
        ),
        Expr::Index { base, index, span } => type_index_expr(
            base,
            index,
            *span,
            env,
            tracker,
            fns,
            trait_env,
            aliases,
            type_defs,
            type_params,
            bounds,
            depth,
        ),
        Expr::Block { block } => type_block(
            block,
            env,
            tracker,
            fns,
            trait_env,
            aliases,
            type_defs,
            type_params,
            bounds,
            depth + 1,
            expected,
        ),
        Expr::If {
            cond,
            then_br,
            else_br,
            span,
        } => {
            let cty = type_of(
                cond,
                env,
                tracker,
                fns,
                trait_env,
                aliases,
                type_defs,
                type_params,
                bounds,
                depth + 1,
                None,
            )?;
            if base_type(&cty, aliases)? != Type::Bool {
                // Reuse arg_type_mismatch with pseudo-callee `if` for stable code T003
                return Err(TyperError::arg_type_mismatch(1, "if", Type::Bool, cty, *span).into());
            }
            let baseline = tracker.clone();
            let mut then_tracker = baseline.clone();
            let tty = type_of(
                then_br,
                env,
                &mut then_tracker,
                fns,
                trait_env,
                aliases,
                type_defs,
                type_params,
                bounds,
                depth + 1,
                expected,
            )?;
            let mut else_tracker = baseline.clone();
            let ety = type_of(
                else_br,
                env,
                &mut else_tracker,
                fns,
                trait_env,
                aliases,
                type_defs,
                type_params,
                bounds,
                depth + 1,
                expected,
            )?;
            if tty != ety {
                return Err(TyperError::branch_type_mismatch(tty, ety, *span).into());
            }
            then_tracker.retain_keys_from(&baseline);
            else_tracker.retain_keys_from(&baseline);
            then_tracker.merge_branch(&else_tracker, *span)?;
            *tracker = then_tracker;
            Ok(tty)
        }

        Expr::Match {
            scrutinee,
            arms,
            span,
        } => type_match_expr(
            scrutinee,
            arms,
            *span,
            env,
            tracker,
            fns,
            trait_env,
            aliases,
            type_defs,
            type_params,
            bounds,
            depth,
            expected,
        ),
        Expr::Try { expr, span } => type_try_expr(
            expr,
            *span,
            env,
            tracker,
            fns,
            trait_env,
            aliases,
            type_defs,
            type_params,
            bounds,
            depth,
        ),
        Expr::Var(name, sp) => {
            tracker.use_var(name, *sp)?;
            match env.get(name.as_str()) {
                Some(binding) => Ok(binding.ty.clone()),
                None => {
                    if name == "result" {
                        if let Some(binding) = env.get(RETURN_KEY) {
                            return Ok(binding.ty.clone());
                        }
                    }
                    Err(TyperError::unknown_variable(name, *sp).into())
                }
            }
        }
        Expr::Return { expr, .. } => {
            let ret_expected = expected.or_else(|| env.get(RETURN_KEY).map(|binding| &binding.ty));
            let ty = type_of(
                expr,
                env,
                tracker,
                fns,
                trait_env,
                aliases,
                type_defs,
                type_params,
                bounds,
                depth + 1,
                ret_expected,
            )?;
            if is_resource_type(&ty, aliases, type_defs)? {
                consume_var_expr(tracker, expr.as_ref())?;
            }
            Ok(ty)
        }
        Expr::Unary { op, expr, span } => type_unary_expr(
            *op,
            expr,
            *span,
            env,
            tracker,
            fns,
            trait_env,
            aliases,
            type_defs,
            type_params,
            bounds,
            depth,
        ),
        Expr::Bin { op, lhs, rhs, span } => type_bin_expr(
            *op,
            lhs,
            rhs,
            *span,
            env,
            tracker,
            fns,
            trait_env,
            aliases,
            type_defs,
            type_params,
            bounds,
            depth,
        ),
        Expr::Call {
            callee,
            args,
            type_args,
            span,
        } => type_call_expr(
            callee.as_str(),
            args,
            type_args,
            env,
            tracker,
            fns,
            trait_env,
            aliases,
            type_defs,
            type_params,
            bounds,
            depth,
            expected,
            *span,
        ),
        Expr::Lambda { params, body, span } => type_lambda_expr(
            params,
            body.as_ref(),
            *span,
            expected,
            env,
            tracker,
            fns,
            trait_env,
            aliases,
            type_defs,
            type_params,
            bounds,
            depth,
        ),
    }
}

pub(crate) fn infer_expr_type<'a>(
    e: &'a Expr,
    env: &HashMap<&'a str, LocalBinding>,
    fns: &HashMap<&'a str, FnSig>,
    trait_env: &TraitEnv<'a>,
    aliases: &AliasMap,
    type_defs: &TypeDefs,
    type_params: &HashSet<String>,
    bounds: &BoundsMap,
) -> Result<Type> {
    let mut tracker = ResourceTracker::new();
    type_of(
        e,
        env,
        &mut tracker,
        fns,
        trait_env,
        aliases,
        type_defs,
        type_params,
        bounds,
        0,
        None,
    )
}
