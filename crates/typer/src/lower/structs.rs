use crate::check::{base_type, infer_expr_type};
use anyhow::Result;
use clg_ast::{Expr, StructFieldInit, Type};
use clg_ir::Value;
use std::collections::HashMap;

use super::layout::struct_layout;
use super::{emit_alloc, load_value_borrow, lower_expr, store_value, LowerCtx};

pub(super) fn lower_struct_lit<'a>(
    ctx: &mut LowerCtx<'a>,
    expr: &'a Expr,
    fields: &'a [StructFieldInit],
) -> Result<Value> {
    let struct_ty = infer_expr_type(
        expr,
        &ctx.type_env,
        &ctx.fns,
        ctx.trait_env,
        ctx.aliases,
        ctx.type_defs,
        &ctx.type_params,
        &ctx.bounds,
    )?;
    let resolved = base_type(&struct_ty, ctx.aliases)?;
    let Type::Named { name: type_name, args } = resolved else {
        anyhow::bail!("struct literal expects a struct value");
    };
    let (decl_fields, layout) = struct_layout(
        ctx.type_defs,
        ctx.aliases,
        ctx.std_types,
        type_name.as_str(),
        &args,
    )?;
    let mut field_offsets: HashMap<&str, (u32, Type)> =
        HashMap::with_capacity(decl_fields.len());
    for (idx, field) in decl_fields.iter().enumerate() {
        let offset = *layout
            .offsets
            .get(idx)
            .ok_or_else(|| anyhow::anyhow!("struct field offset missing"))?;
        field_offsets.insert(field.name.as_str(), (offset, field.ty.clone()));
    }
    let ptr = emit_alloc(ctx, layout.size, layout.align);
    for field in fields {
        let (offset, field_ty) = field_offsets
            .get(field.name.as_str())
            .ok_or_else(|| anyhow::anyhow!("unknown struct field `{}`", field.name))?
            .clone();
        let val = lower_expr(ctx, &field.expr, Some(field_ty.clone()))?;
        store_value(ctx, &field_ty, ptr, offset, val)?;
    }
    Ok(ptr)
}

pub(super) fn lower_field_access<'a>(
    ctx: &mut LowerCtx<'a>,
    base: &'a Expr,
    field: &str,
) -> Result<Value> {
    let base_ty = infer_expr_type(
        base,
        &ctx.type_env,
        &ctx.fns,
        ctx.trait_env,
        ctx.aliases,
        ctx.type_defs,
        &ctx.type_params,
        &ctx.bounds,
    )?;
    let resolved = base_type(&base_ty, ctx.aliases)?;
    let Type::Named { name, args } = resolved else {
        anyhow::bail!("field access expects a struct value");
    };
    let (decl_fields, layout) =
        struct_layout(ctx.type_defs, ctx.aliases, ctx.std_types, name.as_str(), &args)?;
    let mut field_idx: Option<usize> = None;
    let mut field_ty: Option<Type> = None;
    for (idx, f) in decl_fields.iter().enumerate() {
        if f.name == field {
            field_idx = Some(idx);
            field_ty = Some(f.ty.clone());
            break;
        }
    }
    let idx = field_idx.ok_or_else(|| anyhow::anyhow!("unknown field `{}`", field))?;
    let field_ty = field_ty.ok_or_else(|| anyhow::anyhow!("field type missing"))?;
    let offset = *layout
        .offsets
        .get(idx)
        .ok_or_else(|| anyhow::anyhow!("struct field offset missing"))?;
    let base_ptr = lower_expr(ctx, base, None)?;
    load_value_borrow(ctx, &field_ty, base_ptr, offset)
}
