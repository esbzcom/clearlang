use crate::check::{base_type, infer_expr_type};
use anyhow::Result;
use clg_ast::{Expr, Type};
use clg_ir::{Instr, IrType, Value};

use super::array::{
    ARRAY_HEADER_ALIGN, ARRAY_HEADER_DATA_OFFSET, ARRAY_HEADER_LEN_OFFSET, ARRAY_HEADER_SIZE,
};
use super::layout::{array_layout, tuple_layout};
use super::{emit_alloc, emit_int_const, emit_u64_const, fresh, lower_expr, store_value, LowerCtx};

pub(super) fn lower_int_lit(
    ctx: &mut LowerCtx<'_>,
    n: i64,
    expected: Option<Type>,
) -> Result<Value> {
    match expected {
        Some(Type::U8) => {
            let dst = fresh(ctx);
            ctx.body.push(Instr::IConst {
                dst,
                ty: IrType::U8,
                n,
            });
            Ok(dst)
        }
        Some(Type::U64) => {
            let dst = fresh(ctx);
            ctx.body.push(Instr::IConst {
                dst,
                ty: IrType::U64,
                n,
            });
            Ok(dst)
        }
        Some(Type::U128) => {
            if n < 0 {
                anyhow::bail!("U128 literal must be non-negative");
            }
            let limb_lo = emit_u64_const(ctx, n as u64);
            let limb_hi = emit_u64_const(ctx, 0);
            let dst = fresh(ctx);
            ctx.body.push(Instr::U128Init {
                dst,
                limb_lo,
                limb_hi,
            });
            Ok(dst)
        }
        Some(Type::U256) => {
            if n < 0 {
                anyhow::bail!("U256 literal must be non-negative");
            }
            let limb0 = emit_u64_const(ctx, n as u64);
            let limb1 = emit_u64_const(ctx, 0);
            let limb2 = emit_u64_const(ctx, 0);
            let limb3 = emit_u64_const(ctx, 0);
            let dst = fresh(ctx);
            ctx.body.push(Instr::U256Init {
                dst,
                limb0,
                limb1,
                limb2,
                limb3,
            });
            Ok(dst)
        }
        _ => {
            let dst = fresh(ctx);
            ctx.body.push(Instr::IConst {
                dst,
                ty: IrType::Int,
                n,
            });
            Ok(dst)
        }
    }
}

pub(super) fn lower_bool_lit(ctx: &mut LowerCtx<'_>, value: bool) -> Result<Value> {
    let dst = fresh(ctx);
    ctx.body.push(Instr::IConst {
        dst,
        ty: IrType::Bool,
        n: if value { 1 } else { 0 },
    });
    Ok(dst)
}

pub(super) fn lower_string_lit(ctx: &mut LowerCtx<'_>, value: &str) -> Result<Value> {
    let dst = fresh(ctx);
    ctx.body.push(Instr::IStringConst {
        dst,
        s: value.to_owned(),
    });
    Ok(dst)
}

pub(super) fn lower_array_lit<'a>(ctx: &mut LowerCtx<'a>, elems: &'a [Expr]) -> Result<Value> {
    let first = elems
        .first()
        .ok_or_else(|| anyhow::anyhow!("array literal requires at least one element"))?;
    let elem_ty = infer_expr_type(
        first,
        &ctx.type_env,
        &ctx.fns,
        ctx.trait_env,
        ctx.aliases,
        ctx.type_defs,
        &ctx.type_params,
        &ctx.bounds,
    )?;
    let elem_ty = base_type(&elem_ty, ctx.aliases)?;
    let layout = array_layout(&elem_ty, elems.len() as u32, ctx.aliases, ctx.std_types)?;
    let data_ptr = emit_alloc(ctx, layout.size, layout.align);
    let header_ptr = emit_alloc(ctx, ARRAY_HEADER_SIZE, ARRAY_HEADER_ALIGN);
    let len_val = emit_int_const(ctx, elems.len() as i64);
    ctx.body.push(Instr::Store {
        ptr: header_ptr,
        src: len_val,
        offset: ARRAY_HEADER_LEN_OFFSET,
        ty: IrType::Int,
    });
    ctx.body.push(Instr::Store {
        ptr: header_ptr,
        src: data_ptr,
        offset: ARRAY_HEADER_DATA_OFFSET,
        ty: IrType::Int,
    });
    for (idx, elem) in elems.iter().enumerate() {
        let offset = (idx as u64)
            .checked_mul(layout.stride as u64)
            .ok_or_else(|| anyhow::anyhow!("array literal offset overflow"))?;
        if offset > u32::MAX as u64 {
            anyhow::bail!("array literal offset exceeds u32 limits");
        }
        let val = lower_expr(ctx, elem, Some(elem_ty.clone()))?;
        store_value(ctx, &elem_ty, data_ptr, offset as u32, val)?;
    }
    Ok(header_ptr)
}

pub(super) fn lower_tuple_lit<'a>(ctx: &mut LowerCtx<'a>, elems: &'a [Expr]) -> Result<Value> {
    let mut elem_tys = Vec::with_capacity(elems.len());
    for elem in elems {
        let ty = infer_expr_type(
            elem,
            &ctx.type_env,
            &ctx.fns,
            ctx.trait_env,
            ctx.aliases,
            ctx.type_defs,
            &ctx.type_params,
            &ctx.bounds,
        )?;
        elem_tys.push(base_type(&ty, ctx.aliases)?);
    }
    let layout = tuple_layout(&elem_tys, ctx.aliases, ctx.std_types)?;
    let ptr = emit_alloc(ctx, layout.size, layout.align);
    for (idx, elem) in elems.iter().enumerate() {
        let elem_ty = elem_tys
            .get(idx)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("tuple element missing"))?;
        let val = lower_expr(ctx, elem, Some(elem_ty.clone()))?;
        let offset = *layout
            .offsets
            .get(idx)
            .ok_or_else(|| anyhow::anyhow!("tuple offset missing"))?;
        store_value(ctx, &elem_ty, ptr, offset, val)?;
    }
    Ok(ptr)
}
