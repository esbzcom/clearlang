use anyhow::Result;
use clg_ast::{Block, Expr, MatchPat, ParamKind, Span, Stmt, Type};
use std::collections::{HashMap, HashSet};

use super::super::{
    base_types_match, is_resource_type, refinement_loss, AliasMap, BoundsMap, FnSig, LocalBinding,
    TraitEnv, TypeDefs,
};
use super::literals::{ensure_bool, ensure_int};
use super::{consume_var_expr, expr_span, type_of, ResourceTracker};
use crate::errors::TyperError;

fn name_is_bound(name: &str, scopes: &[HashSet<&str>]) -> bool {
    scopes.iter().rev().any(|scope| scope.contains(name))
}

fn collect_closure_calls_expr<'a>(
    expr: &'a Expr,
    closure_names: &HashSet<&'a str>,
    scopes: &mut Vec<HashSet<&'a str>>,
    out: &mut Vec<(&'a str, Span)>,
) {
    match expr {
        Expr::Int(_, _) | Expr::Bool(_, _) | Expr::String(_, _) | Expr::Var(_, _) => {}
        Expr::Call { callee, args, span } => {
            let callee_name = callee.as_str();
            if !callee_name.contains("::")
                && closure_names.contains(callee_name)
                && !name_is_bound(callee_name, scopes)
            {
                out.push((callee_name, *span));
            }
            for arg in args {
                collect_closure_calls_expr(arg, closure_names, scopes, out);
            }
        }
        Expr::ArrayLit { elems, .. } | Expr::TupleLit { elems, .. } => {
            for elem in elems {
                collect_closure_calls_expr(elem, closure_names, scopes, out);
            }
        }
        Expr::StructLit { fields, .. } => {
            for field in fields {
                collect_closure_calls_expr(&field.expr, closure_names, scopes, out);
            }
        }
        Expr::FieldAccess { base, .. } => {
            collect_closure_calls_expr(base, closure_names, scopes, out);
        }
        Expr::Index { base, index, .. } => {
            collect_closure_calls_expr(base, closure_names, scopes, out);
            collect_closure_calls_expr(index, closure_names, scopes, out);
        }
        Expr::Block { block } => collect_closure_calls_block(block, closure_names, scopes, out),
        Expr::Bin { lhs, rhs, .. } => {
            collect_closure_calls_expr(lhs, closure_names, scopes, out);
            collect_closure_calls_expr(rhs, closure_names, scopes, out);
        }
        Expr::Return { expr, .. } | Expr::Unary { expr, .. } | Expr::Try { expr, .. } => {
            collect_closure_calls_expr(expr, closure_names, scopes, out);
        }
        Expr::If {
            cond,
            then_br,
            else_br,
            ..
        } => {
            collect_closure_calls_expr(cond, closure_names, scopes, out);
            collect_closure_calls_expr(then_br, closure_names, scopes, out);
            collect_closure_calls_expr(else_br, closure_names, scopes, out);
        }
        Expr::Match {
            scrutinee, arms, ..
        } => {
            collect_closure_calls_expr(scrutinee, closure_names, scopes, out);
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
                collect_closure_calls_expr(&arm.expr, closure_names, scopes, out);
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
            collect_closure_calls_expr(body, closure_names, scopes, out);
            scopes.pop();
        }
    }
}

fn collect_closure_calls_block<'a>(
    block: &'a Block,
    closure_names: &HashSet<&'a str>,
    scopes: &mut Vec<HashSet<&'a str>>,
    out: &mut Vec<(&'a str, Span)>,
) {
    scopes.push(HashSet::new());
    for stmt in &block.statements {
        match stmt {
            Stmt::Let { name, expr, .. } => {
                collect_closure_calls_expr(expr, closure_names, scopes, out);
                if let Some(scope) = scopes.last_mut() {
                    scope.insert(name.as_str());
                }
            }
            Stmt::Expr { expr, .. } => {
                collect_closure_calls_expr(expr, closure_names, scopes, out);
            }
            Stmt::While {
                cond,
                invariant,
                variant,
                body,
                ..
            } => {
                collect_closure_calls_expr(cond, closure_names, scopes, out);
                collect_closure_calls_expr(invariant, closure_names, scopes, out);
                if let Some(v) = variant {
                    collect_closure_calls_expr(v, closure_names, scopes, out);
                }
                collect_closure_calls_block(body, closure_names, scopes, out);
            }
        }
    }
    if let Some(tail) = &block.tail {
        collect_closure_calls_expr(tail, closure_names, scopes, out);
    }
    scopes.pop();
}

fn has_path(
    start: usize,
    target: usize,
    edges: &[Vec<(usize, Span)>],
    visited: &mut [bool],
) -> bool {
    if start == target {
        return true;
    }
    if visited[start] {
        return false;
    }
    visited[start] = true;
    for (next, _) in &edges[start] {
        if has_path(*next, target, edges, visited) {
            return true;
        }
    }
    false
}

fn reject_recursive_closure_values(block: &Block) -> Result<()> {
    let mut closure_bindings: Vec<(&str, Span, &Expr)> = Vec::new();
    let mut closure_index: HashMap<&str, usize> = HashMap::new();
    for stmt in &block.statements {
        if let Stmt::Let { name, expr, span } = stmt {
            if matches!(expr.as_ref(), Expr::Lambda { .. }) {
                let idx = closure_bindings.len();
                closure_bindings.push((name.as_str(), *span, expr.as_ref()));
                closure_index.insert(name.as_str(), idx);
            }
        }
    }
    if closure_bindings.is_empty() {
        return Ok(());
    }

    let closure_names: HashSet<&str> = closure_bindings.iter().map(|(name, _, _)| *name).collect();
    let mut edges: Vec<Vec<(usize, Span)>> = vec![Vec::new(); closure_bindings.len()];

    for (from_idx, (_, _, expr)) in closure_bindings.iter().enumerate() {
        let Expr::Lambda { params, body, .. } = expr else {
            continue;
        };
        let mut scopes: Vec<HashSet<&str>> = Vec::with_capacity(2);
        scopes.push(params.iter().map(|param| param.name.as_str()).collect());
        let mut calls: Vec<(&str, Span)> = Vec::new();
        collect_closure_calls_expr(body, &closure_names, &mut scopes, &mut calls);
        for (callee, call_span) in calls {
            if let Some(to_idx) = closure_index.get(callee).copied() {
                edges[from_idx].push((to_idx, call_span));
            }
        }
    }

    for (idx, outgoing) in edges.iter().enumerate() {
        for (next, call_span) in outgoing {
            if *next == idx {
                let (name, _, _) = closure_bindings[idx];
                return Err(TyperError::feature_not_supported(
                    &format!("self-referential closure value `{name}`"),
                    *call_span,
                )
                .into());
            }
        }
    }

    for (idx, outgoing) in edges.iter().enumerate() {
        for (next, call_span) in outgoing {
            let mut visited = vec![false; edges.len()];
            if has_path(*next, idx, &edges, &mut visited) {
                let (from_name, _, _) = closure_bindings[idx];
                let (to_name, _, _) = closure_bindings[*next];
                return Err(TyperError::feature_not_supported(
                    &format!("mutually recursive closure values `{from_name}` and `{to_name}`"),
                    *call_span,
                )
                .into());
            }
        }
    }

    Ok(())
}

pub(super) fn type_block_stmt<'a>(
    block: &'a Block,
    env: &HashMap<&'a str, LocalBinding>,
    tracker: &mut ResourceTracker,
    fns: &HashMap<&'a str, FnSig>,
    trait_env: &TraitEnv<'a>,
    aliases: &AliasMap,
    type_defs: &TypeDefs,
    type_params: &HashSet<String>,
    bounds: &BoundsMap,
    depth: usize,
) -> Result<()> {
    reject_recursive_closure_values(block)?;
    let mut inner_env = env.clone();
    let mut inner_tracker = tracker.clone();
    for stmt in &block.statements {
        match stmt {
            Stmt::Let { name, expr, .. } => {
                let ty = type_of(
                    expr.as_ref(),
                    &inner_env,
                    &mut inner_tracker,
                    fns,
                    trait_env,
                    aliases,
                    type_defs,
                    type_params,
                    bounds,
                    depth + 1,
                    None,
                )?;
                if let Some(existing) = inner_env.get(name.as_str()) {
                    if base_types_match(&existing.ty, &ty, aliases)?
                        && refinement_loss(&ty, &existing.ty, aliases)
                    {
                        let sp = expr_span(expr.as_ref());
                        return Err(TyperError::refinement_loss(existing.ty.clone(), ty, sp).into());
                    }
                }
                let is_resource = is_resource_type(&ty, aliases, type_defs)?;
                if is_resource {
                    consume_var_expr(&mut inner_tracker, expr.as_ref())?;
                }
                inner_tracker.register_local(name.as_str(), is_resource);
                inner_env.insert(
                    name.as_str(),
                    LocalBinding {
                        ty,
                        kind: ParamKind::Borrow,
                    },
                );
            }
            Stmt::While {
                cond,
                invariant,
                variant,
                body,
                span,
            } => {
                type_while_stmt(
                    cond.as_ref(),
                    invariant.as_ref(),
                    variant.as_ref().map(|v| v.as_ref()),
                    body.as_ref(),
                    &inner_env,
                    &mut inner_tracker,
                    fns,
                    trait_env,
                    aliases,
                    type_defs,
                    type_params,
                    bounds,
                    depth + 1,
                    *span,
                )?;
            }
            Stmt::Expr { expr, .. } => {
                type_of(
                    expr.as_ref(),
                    &inner_env,
                    &mut inner_tracker,
                    fns,
                    trait_env,
                    aliases,
                    type_defs,
                    type_params,
                    bounds,
                    depth + 1,
                    None,
                )?;
            }
        }
    }
    if let Some(tail) = &block.tail {
        let ty = type_of(
            tail.as_ref(),
            &inner_env,
            &mut inner_tracker,
            fns,
            trait_env,
            aliases,
            type_defs,
            type_params,
            bounds,
            depth + 1,
            None,
        )?;
        if is_resource_type(&ty, aliases, type_defs)? {
            consume_var_expr(&mut inner_tracker, tail.as_ref())?;
        }
    }
    *tracker = inner_tracker;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn type_while_stmt<'a>(
    cond: &'a Expr,
    invariant: &'a Expr,
    variant: Option<&'a Expr>,
    body: &'a Block,
    env: &HashMap<&'a str, LocalBinding>,
    tracker: &mut ResourceTracker,
    fns: &HashMap<&'a str, FnSig>,
    trait_env: &TraitEnv<'a>,
    aliases: &AliasMap,
    type_defs: &TypeDefs,
    type_params: &HashSet<String>,
    bounds: &BoundsMap,
    depth: usize,
    span: Span,
) -> Result<()> {
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
        depth,
        None,
    )?;
    ensure_bool(cty, aliases, "condition", Some(expr_span(cond)))?;
    let inv_ty = type_of(
        invariant,
        env,
        tracker,
        fns,
        trait_env,
        aliases,
        type_defs,
        type_params,
        bounds,
        depth,
        None,
    )?;
    ensure_bool(
        inv_ty,
        aliases,
        "loop invariant",
        Some(expr_span(invariant)),
    )?;
    if let Some(var_expr) = variant {
        let vty = type_of(
            var_expr,
            env,
            tracker,
            fns,
            trait_env,
            aliases,
            type_defs,
            type_params,
            bounds,
            depth,
            None,
        )?;
        ensure_int(vty, aliases, "loop variant", Some(expr_span(var_expr)))?;
    }
    let baseline = tracker.clone();
    let mut body_tracker = baseline.clone();
    type_block_stmt(
        body,
        env,
        &mut body_tracker,
        fns,
        trait_env,
        aliases,
        type_defs,
        type_params,
        bounds,
        depth + 1,
    )?;
    body_tracker.retain_keys_from(&baseline);
    body_tracker.merge_branch(&baseline, span)?;
    *tracker = baseline;
    Ok(())
}

pub(super) fn type_block<'a>(
    block: &'a Block,
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
    reject_recursive_closure_values(block)?;
    let mut inner_env = env.clone();
    let mut inner_tracker = tracker.clone();
    for stmt in &block.statements {
        match stmt {
            Stmt::Let { name, expr, .. } => {
                let ty = type_of(
                    expr.as_ref(),
                    &inner_env,
                    &mut inner_tracker,
                    fns,
                    trait_env,
                    aliases,
                    type_defs,
                    type_params,
                    bounds,
                    depth + 1,
                    None,
                )?;
                if let Some(existing) = inner_env.get(name.as_str()) {
                    if base_types_match(&existing.ty, &ty, aliases)?
                        && refinement_loss(&ty, &existing.ty, aliases)
                    {
                        let sp = expr_span(expr.as_ref());
                        return Err(TyperError::refinement_loss(existing.ty.clone(), ty, sp).into());
                    }
                }
                let is_resource = is_resource_type(&ty, aliases, type_defs)?;
                if is_resource {
                    consume_var_expr(&mut inner_tracker, expr.as_ref())?;
                }
                inner_tracker.register_local(name.as_str(), is_resource);
                inner_env.insert(
                    name.as_str(),
                    LocalBinding {
                        ty,
                        kind: ParamKind::Borrow,
                    },
                );
            }
            Stmt::While {
                cond,
                invariant,
                variant,
                body,
                span,
            } => {
                let baseline = inner_tracker.clone();
                type_while_stmt(
                    cond.as_ref(),
                    invariant.as_ref(),
                    variant.as_ref().map(|v| v.as_ref()),
                    body.as_ref(),
                    &inner_env,
                    &mut inner_tracker,
                    fns,
                    trait_env,
                    aliases,
                    type_defs,
                    type_params,
                    bounds,
                    depth + 1,
                    *span,
                )?;
                inner_tracker = baseline;
            }
            Stmt::Expr { expr, .. } => {
                type_of(
                    expr.as_ref(),
                    &inner_env,
                    &mut inner_tracker,
                    fns,
                    trait_env,
                    aliases,
                    type_defs,
                    type_params,
                    bounds,
                    depth + 1,
                    None,
                )?;
            }
        }
    }
    let result = if let Some(tail) = &block.tail {
        let ty = type_of(
            tail.as_ref(),
            &inner_env,
            &mut inner_tracker,
            fns,
            trait_env,
            aliases,
            type_defs,
            type_params,
            bounds,
            depth + 1,
            expected,
        )?;
        if is_resource_type(&ty, aliases, type_defs)? {
            consume_var_expr(&mut inner_tracker, tail.as_ref())?;
        }
        Ok(ty)
    } else {
        Err(TyperError::block_missing_tail(block.span).into())
    };
    *tracker = inner_tracker;
    result
}
