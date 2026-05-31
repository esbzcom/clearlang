use crate::check::{base_type, infer_expr_type};
use anyhow::Result;
use clg_ast::{Expr, Type};

use super::LowerCtx;

pub(super) fn normalize_collection_callee(callee: &str) -> &str {
    crate::guards::canonical_collection_alias_callee(callee)
}

pub(super) fn list_elem_type(ctx: &LowerCtx<'_>, arg: &Expr) -> Result<Type> {
    let ty = infer_expr_type(
        arg,
        &ctx.type_env,
        &ctx.fns,
        ctx.trait_env,
        ctx.aliases,
        ctx.type_defs,
        &ctx.type_params,
        &ctx.bounds,
    )?;
    let base = base_type(&ty, ctx.aliases)?;
    match base {
        Type::List(inner) => Ok(*inner),
        other => anyhow::bail!("expected List argument, found {:?}", other),
    }
}

pub(super) fn slice_elem_type(ctx: &LowerCtx<'_>, arg: &Expr) -> Result<Type> {
    let ty = infer_expr_type(
        arg,
        &ctx.type_env,
        &ctx.fns,
        ctx.trait_env,
        ctx.aliases,
        ctx.type_defs,
        &ctx.type_params,
        &ctx.bounds,
    )?;
    let base = base_type(&ty, ctx.aliases)?;
    match base {
        Type::Slice(inner) => Ok(*inner),
        other => anyhow::bail!("expected Slice argument, found {:?}", other),
    }
}

pub(super) fn set_elem_type(ctx: &LowerCtx<'_>, arg: &Expr) -> Result<Type> {
    let ty = infer_expr_type(
        arg,
        &ctx.type_env,
        &ctx.fns,
        ctx.trait_env,
        ctx.aliases,
        ctx.type_defs,
        &ctx.type_params,
        &ctx.bounds,
    )?;
    let base = base_type(&ty, ctx.aliases)?;
    match base {
        Type::Set(inner) => Ok(*inner),
        other => anyhow::bail!("expected Set argument, found {:?}", other),
    }
}

pub(super) fn map_key_val_type(ctx: &LowerCtx<'_>, arg: &Expr) -> Result<(Type, Type)> {
    let ty = infer_expr_type(
        arg,
        &ctx.type_env,
        &ctx.fns,
        ctx.trait_env,
        ctx.aliases,
        ctx.type_defs,
        &ctx.type_params,
        &ctx.bounds,
    )?;
    let base = base_type(&ty, ctx.aliases)?;
    match base {
        Type::Map(key, val) => Ok((*key, *val)),
        other => anyhow::bail!("expected Map argument, found {:?}", other),
    }
}
