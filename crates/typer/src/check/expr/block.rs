use anyhow::Result;
use clg_ast::{Block, Expr, ParamKind, Span, Stmt, Type};
use std::collections::{HashMap, HashSet};

use super::super::{
    base_types_match, is_resource_type, refinement_loss, AliasMap, BoundsMap, FnSig, LocalBinding,
    TraitEnv, TypeDefs,
};
use super::literals::{ensure_bool, ensure_int};
use super::{consume_var_expr, expr_span, type_of, ResourceTracker};
use crate::errors::TyperError;

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
