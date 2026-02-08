use crate::check::{base_type, substitute_type, AliasMap};
use anyhow::Result;
use clg_ast::Type;
use clg_ir::{BinOpIR, Instr, IrType, Value, VariantKind};

use super::eq_primitives::{
    emit_eq_bytes_fixed, emit_eq_u128, emit_eq_u256, emit_intrinsic_eq,
};
use super::layout::{build_type_param_subst, struct_layout, tuple_layout};
use super::{emit_bool_const, emit_int_const, fresh, load_value_borrow, LowerCtx};

pub(super) fn emit_eq_for_type(
    ctx: &mut LowerCtx<'_>,
    ty: &Type,
    lhs: Value,
    rhs: Value,
    aliases: &AliasMap,
) -> Result<Value> {
    let base = base_type(ty, aliases)?;
    match base {
        Type::String => emit_intrinsic_eq(ctx, "std::str::eq", lhs, rhs),
        Type::Bytes => emit_intrinsic_eq(ctx, "std::bytes::eq", lhs, rhs),
        Type::U128 => Ok(emit_eq_u128(ctx, lhs, rhs)),
        Type::U256 => Ok(emit_eq_u256(ctx, lhs, rhs)),
        Type::Option(inner) => emit_eq_option(ctx, inner.as_ref(), lhs, rhs, aliases),
        Type::Result(ok, err) => emit_eq_result(ctx, ok.as_ref(), err.as_ref(), lhs, rhs, aliases),
        Type::Tuple(elements) => emit_eq_tuple(ctx, &elements, lhs, rhs, aliases),
        Type::Array(_, _) | Type::Slice(_) => {
            anyhow::bail!("array/slice equality is not supported")
        }
        Type::Named { name, args } => {
            if let Some(info) = ctx.std_types.get(name.as_str()) {
                return emit_eq_bytes_fixed(ctx, lhs, rhs, info.byte_len);
            }
            if ctx.type_defs.structs.contains_key(name.as_str()) {
                emit_eq_struct(ctx, name.as_str(), &args, lhs, rhs, aliases)
            } else if ctx.type_defs.enums.contains_key(name.as_str()) {
                emit_eq_enum(ctx, name.as_str(), &args, lhs, rhs, aliases)
            } else {
                let dst = fresh(ctx);
                ctx.body.push(Instr::IBin {
                    dst,
                    op: BinOpIR::Eq,
                    lhs,
                    rhs,
                    ty: IrType::Int,
                });
                Ok(dst)
            }
        }
        _ => {
            let ir_ty = if matches!(base, Type::U64) {
                IrType::U64
            } else {
                IrType::Int
            };
            let dst = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst,
                op: BinOpIR::Eq,
                lhs,
                rhs,
                ty: ir_ty,
            });
            Ok(dst)
        }
    }
}

fn emit_eq_struct(
    ctx: &mut LowerCtx<'_>,
    name: &str,
    args: &[Type],
    lhs: Value,
    rhs: Value,
    aliases: &AliasMap,
) -> Result<Value> {
    let (fields, layout) = struct_layout(ctx.type_defs, aliases, ctx.std_types, name, args)?;
    let result = emit_bool_const(ctx, true);
    ctx.body.push(Instr::BlockBegin);
    for (idx, field) in fields.iter().enumerate() {
        ctx.body.push(Instr::BrIfEqz { cond: result, depth: 0 });
        let offset = *layout
            .offsets
            .get(idx)
            .ok_or_else(|| anyhow::anyhow!("struct field offset missing"))?;
        let lhs_val = load_value_borrow(ctx, &field.ty, lhs, offset)?;
        let rhs_val = load_value_borrow(ctx, &field.ty, rhs, offset)?;
        let eq = emit_eq_for_type(ctx, &field.ty, lhs_val, rhs_val, aliases)?;
        ctx.body.push(Instr::IBin {
            dst: result,
            op: BinOpIR::And,
            lhs: result,
            rhs: eq,
            ty: IrType::Bool,
        });
    }
    ctx.body.push(Instr::BlockEnd);
    Ok(result)
}

fn emit_eq_tuple(
    ctx: &mut LowerCtx<'_>,
    elements: &[Type],
    lhs: Value,
    rhs: Value,
    aliases: &AliasMap,
) -> Result<Value> {
    let layout = tuple_layout(elements, aliases, ctx.std_types)?;
    let result = emit_bool_const(ctx, true);
    ctx.body.push(Instr::BlockBegin);
    for (idx, elem_ty) in elements.iter().enumerate() {
        ctx.body.push(Instr::BrIfEqz { cond: result, depth: 0 });
        let offset = *layout
            .offsets
            .get(idx)
            .ok_or_else(|| anyhow::anyhow!("tuple element offset missing"))?;
        let lhs_val = load_value_borrow(ctx, elem_ty, lhs, offset)?;
        let rhs_val = load_value_borrow(ctx, elem_ty, rhs, offset)?;
        let eq = emit_eq_for_type(ctx, elem_ty, lhs_val, rhs_val, aliases)?;
        ctx.body.push(Instr::IBin {
            dst: result,
            op: BinOpIR::And,
            lhs: result,
            rhs: eq,
            ty: IrType::Bool,
        });
    }
    ctx.body.push(Instr::BlockEnd);
    Ok(result)
}

fn emit_eq_option(
    ctx: &mut LowerCtx<'_>,
    inner: &Type,
    lhs: Value,
    rhs: Value,
    aliases: &AliasMap,
) -> Result<Value> {
    let lhs_parts = ctx.variant_destructure(lhs, VariantKind::Option);
    let rhs_parts = ctx.variant_destructure(rhs, VariantKind::Option);
    let result = emit_bool_const(ctx, true);
    let tags_eq = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: tags_eq,
        op: BinOpIR::Eq,
        lhs: lhs_parts.tag,
        rhs: rhs_parts.tag,
        ty: IrType::Int,
    });

    ctx.body.push(Instr::BlockBegin);
    ctx.body.push(Instr::BlockBegin);
    ctx.body.push(Instr::BrIf { cond: tags_eq, depth: 0 });
    ctx.body.push(Instr::IConst {
        dst: result,
        ty: IrType::Bool,
        n: 0,
    });
    ctx.body.push(Instr::Br { depth: 1 });
    ctx.body.push(Instr::BlockEnd);

    let tag_some = emit_int_const(ctx, 1);
    let is_some = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: is_some,
        op: BinOpIR::Eq,
        lhs: lhs_parts.tag,
        rhs: tag_some,
        ty: IrType::Int,
    });
    ctx.body.push(Instr::BlockBegin);
    ctx.body.push(Instr::BrIfEqz { cond: is_some, depth: 0 });
    let payload_eq = emit_eq_for_type(ctx, inner, lhs_parts.payload_lo, rhs_parts.payload_lo, aliases)?;
    ctx.body.push(Instr::ISelect {
        dst: result,
        cond: is_some,
        then_v: payload_eq,
        else_v: result,
    });
    ctx.body.push(Instr::BlockEnd);
    ctx.body.push(Instr::BlockEnd);

    Ok(result)
}

fn emit_eq_result(
    ctx: &mut LowerCtx<'_>,
    ok_ty: &Type,
    err_ty: &Type,
    lhs: Value,
    rhs: Value,
    aliases: &AliasMap,
) -> Result<Value> {
    let lhs_parts = ctx.variant_destructure(lhs, VariantKind::Result);
    let rhs_parts = ctx.variant_destructure(rhs, VariantKind::Result);
    let result = emit_bool_const(ctx, false);
    let tags_eq = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: tags_eq,
        op: BinOpIR::Eq,
        lhs: lhs_parts.tag,
        rhs: rhs_parts.tag,
        ty: IrType::Int,
    });

    ctx.body.push(Instr::BlockBegin);
    ctx.body.push(Instr::BrIfEqz { cond: tags_eq, depth: 0 });

    let tag_ok = emit_int_const(ctx, 1);
    let is_ok = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: is_ok,
        op: BinOpIR::Eq,
        lhs: lhs_parts.tag,
        rhs: tag_ok,
        ty: IrType::Int,
    });
    ctx.body.push(Instr::BlockBegin);
    ctx.body.push(Instr::BrIfEqz { cond: is_ok, depth: 0 });
    let ok_eq = emit_eq_for_type(ctx, ok_ty, lhs_parts.payload_lo, rhs_parts.payload_lo, aliases)?;
    ctx.body.push(Instr::ISelect {
        dst: result,
        cond: is_ok,
        then_v: ok_eq,
        else_v: result,
    });
    ctx.body.push(Instr::BlockEnd);

    let tag_err = emit_int_const(ctx, 0);
    let is_err = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: is_err,
        op: BinOpIR::Eq,
        lhs: lhs_parts.tag,
        rhs: tag_err,
        ty: IrType::Int,
    });
    ctx.body.push(Instr::BlockBegin);
    ctx.body.push(Instr::BrIfEqz { cond: is_err, depth: 0 });
    let err_eq =
        emit_eq_for_type(ctx, err_ty, lhs_parts.payload_lo, rhs_parts.payload_lo, aliases)?;
    ctx.body.push(Instr::ISelect {
        dst: result,
        cond: is_err,
        then_v: err_eq,
        else_v: result,
    });
    ctx.body.push(Instr::BlockEnd);
    ctx.body.push(Instr::BlockEnd);

    Ok(result)
}

fn emit_eq_enum(
    ctx: &mut LowerCtx<'_>,
    name: &str,
    args: &[Type],
    lhs: Value,
    rhs: Value,
    aliases: &AliasMap,
) -> Result<Value> {
    let info = ctx
        .type_defs
        .enums
        .get(name)
        .ok_or_else(|| anyhow::anyhow!("unknown enum `{}`", name))?;
    let subst = build_type_param_subst(&info.decl.type_params, args)?;
    let variant_count = info.decl.variants.len() as u32;
    let lhs_parts = ctx.variant_destructure(lhs, VariantKind::Enum { max_tag: variant_count });
    let rhs_parts = ctx.variant_destructure(rhs, VariantKind::Enum { max_tag: variant_count });

    let result = emit_bool_const(ctx, false);
    let tags_eq = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: tags_eq,
        op: BinOpIR::Eq,
        lhs: lhs_parts.tag,
        rhs: rhs_parts.tag,
        ty: IrType::Int,
    });

    ctx.body.push(Instr::BlockBegin);
    ctx.body.push(Instr::BrIfEqz { cond: tags_eq, depth: 0 });

    for (index, variant) in info.decl.variants.iter().enumerate() {
        let mut fields = Vec::with_capacity(variant.fields.len());
        for ty in &variant.fields {
            fields.push(substitute_type(ty, &subst));
        }
        let tag_val = emit_int_const(ctx, index as i64);
        let tag_match = fresh(ctx);
        ctx.body.push(Instr::IBin {
            dst: tag_match,
            op: BinOpIR::Eq,
            lhs: lhs_parts.tag,
            rhs: tag_val,
            ty: IrType::Int,
        });
        ctx.body.push(Instr::BlockBegin);
        ctx.body.push(Instr::BrIfEqz { cond: tag_match, depth: 0 });
        let payload_eq = match fields.len() {
            0 => emit_bool_const(ctx, true),
            1 => emit_eq_for_type(ctx, &fields[0], lhs_parts.payload_lo, rhs_parts.payload_lo, aliases)?,
            _ => emit_eq_tuple(ctx, &fields, lhs_parts.payload_lo, rhs_parts.payload_lo, aliases)?,
        };
        ctx.body.push(Instr::ISelect {
            dst: result,
            cond: tag_match,
            then_v: payload_eq,
            else_v: result,
        });
        ctx.body.push(Instr::BlockEnd);
    }

    ctx.body.push(Instr::BlockEnd);
    Ok(result)
}

