use crate::check::{base_type, infer_expr_type};
use anyhow::Result;
use clg_ast::{Expr, Type};
use clg_ir::{BinOpIR, Instr, IrType, Value};

use super::block::expr_span_local;
use super::collection_types::list_elem_type;
use super::collections_helpers::{
    emit_cap_from_len, emit_collection_cap, emit_collection_data_ptr, emit_collection_guard,
    emit_collection_header, emit_collection_len, emit_collection_payload_guard,
};
use super::layout::collection_layout;
use super::{
    emit_alloc_dyn, emit_int_const, emit_memcpy_bytes, emit_ptr_add, fresh, load_value_copy,
    lower_expr, store_value, zero_value_for_type, LowerCtx,
};

pub(super) fn lower_list_call<'a>(
    ctx: &mut LowerCtx<'a>,
    call_expr: &'a Expr,
    callee: &str,
    args: &'a [Expr],
    expected: Option<&Type>,
) -> Result<Option<Value>> {
    match callee {
        "std::list::new" => {
            if !args.is_empty() {
                anyhow::bail!("`std::list::new` expects no arguments");
            }
            let elem_ty = match expected {
                Some(Type::List(inner)) => *inner.clone(),
                _ => {
                    let call_ty = infer_expr_type(
                        call_expr,
                        &ctx.type_env,
                        &ctx.fns,
                        ctx.trait_env,
                        ctx.aliases,
                        ctx.type_defs,
                        &ctx.type_params,
                        &ctx.bounds,
                    )?;
                    let call_ty = base_type(&call_ty, ctx.aliases)?;
                    match call_ty {
                        Type::List(inner) => *inner,
                        other => anyhow::bail!("cannot infer list element type: {:?}", other),
                    }
                }
            };
            let (_size, align, stride) = collection_layout(&elem_ty, ctx.aliases, ctx.std_types)?;
            let len = emit_int_const(ctx, 0);
            let cap = emit_int_const(ctx, 1);
            let buf_bytes = emit_int_const(ctx, stride as i64);
            let data_ptr = emit_alloc_dyn(ctx, buf_bytes, align);
            let header = emit_collection_header(ctx, len, cap, data_ptr);
            Ok(Some(header))
        }
        "std::list::len" => {
            if args.len() != 1 {
                anyhow::bail!("`std::list::len` expects one argument");
            }
            let list_val = lower_expr(ctx, &args[0], None)?;
            Ok(Some(emit_collection_len(ctx, list_val)))
        }
        "std::list::get" => {
            if args.len() != 2 {
                anyhow::bail!("`std::list::get` expects two arguments");
            }
            let elem_ty = list_elem_type(ctx, &args[0])?;
            let list_val = lower_expr(ctx, &args[0], None)?;
            let idx = lower_expr(ctx, &args[1], Some(Type::Int))?;
            let len = emit_collection_len(ctx, list_val);
            let zero = emit_int_const(ctx, 0);
            let idx_ge_zero = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: idx_ge_zero,
                op: BinOpIR::Ge,
                lhs: idx,
                rhs: zero,
                ty: IrType::Int,
            });
            let idx_lt_len = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: idx_lt_len,
                op: BinOpIR::Lt,
                lhs: idx,
                rhs: len,
                ty: IrType::Int,
            });
            let ok = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: ok,
                op: BinOpIR::And,
                lhs: idx_ge_zero,
                rhs: idx_lt_len,
                ty: IrType::Int,
            });
            let safe_idx = fresh(ctx);
            ctx.body.push(Instr::ISelect {
                dst: safe_idx,
                cond: ok,
                then_v: idx,
                else_v: zero,
            });
            let (_size, align, stride) = collection_layout(&elem_ty, ctx.aliases, ctx.std_types)?;
            let stride_val = emit_int_const(ctx, stride as i64);
            let offset = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: offset,
                op: BinOpIR::Mul,
                lhs: safe_idx,
                rhs: stride_val,
                ty: IrType::Int,
            });
            let data_ptr = emit_collection_data_ptr(ctx, list_val);
            let cap = emit_collection_cap(ctx, list_val);
            emit_collection_payload_guard(ctx, data_ptr, len, cap, stride, align);
            let elem_ptr = emit_ptr_add(ctx, data_ptr, offset);
            let elem_val = load_value_copy(ctx, &elem_ty, elem_ptr, 0)?;
            let zero_payload = zero_value_for_type(ctx, &elem_ty)?;
            let payload = fresh(ctx);
            ctx.body.push(Instr::ISelect {
                dst: payload,
                cond: ok,
                then_v: elem_val,
                else_v: zero_payload,
            });
            let zero_hi = emit_int_const(ctx, 0);
            Ok(Some(ctx.variant_init(ok, payload, zero_hi)))
        }
        "std::list::push" => {
            if args.len() != 2 {
                anyhow::bail!("`std::list::push` expects two arguments");
            }
            let elem_ty = list_elem_type(ctx, &args[0])?;
            let list_val = lower_expr(ctx, &args[0], None)?;
            let elem_val = lower_expr(ctx, &args[1], Some(elem_ty.clone()))?;
            let len = emit_collection_len(ctx, list_val);
            let one = emit_int_const(ctx, 1);
            let new_len = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: new_len,
                op: BinOpIR::Add,
                lhs: len,
                rhs: one,
                ty: IrType::Int,
            });
            let (_size, align, stride) = collection_layout(&elem_ty, ctx.aliases, ctx.std_types)?;
            let stride_val = emit_int_const(ctx, stride as i64);
            let new_cap = emit_cap_from_len(ctx, new_len);
            let buf_bytes = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: buf_bytes,
                op: BinOpIR::Mul,
                lhs: new_cap,
                rhs: stride_val,
                ty: IrType::Int,
            });
            let new_data = emit_alloc_dyn(ctx, buf_bytes, align);
            let old_data = emit_collection_data_ptr(ctx, list_val);
            let cap = emit_collection_cap(ctx, list_val);
            emit_collection_payload_guard(ctx, old_data, len, cap, stride, align);
            let copy_bytes = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: copy_bytes,
                op: BinOpIR::Mul,
                lhs: len,
                rhs: stride_val,
                ty: IrType::Int,
            });
            emit_memcpy_bytes(ctx, old_data, new_data, copy_bytes)?;
            let offset = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: offset,
                op: BinOpIR::Mul,
                lhs: len,
                rhs: stride_val,
                ty: IrType::Int,
            });
            let elem_ptr = emit_ptr_add(ctx, new_data, offset);
            store_value(ctx, &elem_ty, elem_ptr, 0, elem_val)?;
            let header = emit_collection_header(ctx, new_len, new_cap, new_data);
            Ok(Some(header))
        }
        "std::list::insert" => {
            if args.len() != 3 {
                anyhow::bail!("`std::list::insert` expects three arguments");
            }
            let elem_ty = list_elem_type(ctx, &args[0])?;
            let list_val = lower_expr(ctx, &args[0], None)?;
            let elem_val = lower_expr(ctx, &args[1], Some(elem_ty.clone()))?;
            let idx = lower_expr(ctx, &args[2], Some(Type::Int))?;
            let len = emit_collection_len(ctx, list_val);
            let zero = emit_int_const(ctx, 0);
            let idx_ge_zero = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: idx_ge_zero,
                op: BinOpIR::Ge,
                lhs: idx,
                rhs: zero,
                ty: IrType::Int,
            });
            let idx_le_len = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: idx_le_len,
                op: BinOpIR::Le,
                lhs: idx,
                rhs: len,
                ty: IrType::Int,
            });
            let ok = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: ok,
                op: BinOpIR::And,
                lhs: idx_ge_zero,
                rhs: idx_le_len,
                ty: IrType::Int,
            });
            emit_collection_guard(ctx, ok, expr_span_local(&args[2]));
            let one = emit_int_const(ctx, 1);
            let new_len = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: new_len,
                op: BinOpIR::Add,
                lhs: len,
                rhs: one,
                ty: IrType::Int,
            });
            let (_size, align, stride) = collection_layout(&elem_ty, ctx.aliases, ctx.std_types)?;
            let stride_val = emit_int_const(ctx, stride as i64);
            let new_cap = emit_cap_from_len(ctx, new_len);
            let buf_bytes = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: buf_bytes,
                op: BinOpIR::Mul,
                lhs: new_cap,
                rhs: stride_val,
                ty: IrType::Int,
            });
            let new_data = emit_alloc_dyn(ctx, buf_bytes, align);
            let old_data = emit_collection_data_ptr(ctx, list_val);
            let cap = emit_collection_cap(ctx, list_val);
            emit_collection_payload_guard(ctx, old_data, len, cap, stride, align);
            let bytes_before = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: bytes_before,
                op: BinOpIR::Mul,
                lhs: idx,
                rhs: stride_val,
                ty: IrType::Int,
            });
            emit_memcpy_bytes(ctx, old_data, new_data, bytes_before)?;
            let offset = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: offset,
                op: BinOpIR::Mul,
                lhs: idx,
                rhs: stride_val,
                ty: IrType::Int,
            });
            let elem_ptr = emit_ptr_add(ctx, new_data, offset);
            store_value(ctx, &elem_ty, elem_ptr, 0, elem_val)?;
            let idx_plus_one = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: idx_plus_one,
                op: BinOpIR::Add,
                lhs: idx,
                rhs: one,
                ty: IrType::Int,
            });
            let remaining = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: remaining,
                op: BinOpIR::Sub,
                lhs: len,
                rhs: idx,
                ty: IrType::Int,
            });
            let bytes_after = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: bytes_after,
                op: BinOpIR::Mul,
                lhs: remaining,
                rhs: stride_val,
                ty: IrType::Int,
            });
            let src_ptr = emit_ptr_add(ctx, old_data, offset);
            let dst_offset = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: dst_offset,
                op: BinOpIR::Mul,
                lhs: idx_plus_one,
                rhs: stride_val,
                ty: IrType::Int,
            });
            let dst_ptr = emit_ptr_add(ctx, new_data, dst_offset);
            emit_memcpy_bytes(ctx, src_ptr, dst_ptr, bytes_after)?;
            let header = emit_collection_header(ctx, new_len, new_cap, new_data);
            Ok(Some(header))
        }
        "std::list::remove" => {
            if args.len() != 2 {
                anyhow::bail!("`std::list::remove` expects two arguments");
            }
            let elem_ty = list_elem_type(ctx, &args[0])?;
            let list_val = lower_expr(ctx, &args[0], None)?;
            let idx = lower_expr(ctx, &args[1], Some(Type::Int))?;
            let len = emit_collection_len(ctx, list_val);
            let zero = emit_int_const(ctx, 0);
            let idx_ge_zero = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: idx_ge_zero,
                op: BinOpIR::Ge,
                lhs: idx,
                rhs: zero,
                ty: IrType::Int,
            });
            let idx_lt_len = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: idx_lt_len,
                op: BinOpIR::Lt,
                lhs: idx,
                rhs: len,
                ty: IrType::Int,
            });
            let ok = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: ok,
                op: BinOpIR::And,
                lhs: idx_ge_zero,
                rhs: idx_lt_len,
                ty: IrType::Int,
            });
            emit_collection_guard(ctx, ok, expr_span_local(&args[1]));
            let one = emit_int_const(ctx, 1);
            let new_len = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: new_len,
                op: BinOpIR::Sub,
                lhs: len,
                rhs: one,
                ty: IrType::Int,
            });
            let (_size, align, stride) = collection_layout(&elem_ty, ctx.aliases, ctx.std_types)?;
            let stride_val = emit_int_const(ctx, stride as i64);
            let new_cap = emit_cap_from_len(ctx, new_len);
            let buf_bytes = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: buf_bytes,
                op: BinOpIR::Mul,
                lhs: new_cap,
                rhs: stride_val,
                ty: IrType::Int,
            });
            let new_data = emit_alloc_dyn(ctx, buf_bytes, align);
            let old_data = emit_collection_data_ptr(ctx, list_val);
            let cap = emit_collection_cap(ctx, list_val);
            emit_collection_payload_guard(ctx, old_data, len, cap, stride, align);
            let bytes_before = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: bytes_before,
                op: BinOpIR::Mul,
                lhs: idx,
                rhs: stride_val,
                ty: IrType::Int,
            });
            emit_memcpy_bytes(ctx, old_data, new_data, bytes_before)?;
            let idx_plus_one = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: idx_plus_one,
                op: BinOpIR::Add,
                lhs: idx,
                rhs: one,
                ty: IrType::Int,
            });
            let remaining = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: remaining,
                op: BinOpIR::Sub,
                lhs: len,
                rhs: idx_plus_one,
                ty: IrType::Int,
            });
            let bytes_after = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: bytes_after,
                op: BinOpIR::Mul,
                lhs: remaining,
                rhs: stride_val,
                ty: IrType::Int,
            });
            let src_offset = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: src_offset,
                op: BinOpIR::Mul,
                lhs: idx_plus_one,
                rhs: stride_val,
                ty: IrType::Int,
            });
            let src_ptr = emit_ptr_add(ctx, old_data, src_offset);
            let dst_offset = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: dst_offset,
                op: BinOpIR::Mul,
                lhs: idx,
                rhs: stride_val,
                ty: IrType::Int,
            });
            let dst_ptr = emit_ptr_add(ctx, new_data, dst_offset);
            emit_memcpy_bytes(ctx, src_ptr, dst_ptr, bytes_after)?;
            let header = emit_collection_header(ctx, new_len, new_cap, new_data);
            Ok(Some(header))
        }
        "std::list::pop" => {
            if args.len() != 1 {
                anyhow::bail!("`std::list::pop` expects one argument");
            }
            let elem_ty = list_elem_type(ctx, &args[0])?;
            let list_val = lower_expr(ctx, &args[0], None)?;
            let len = emit_collection_len(ctx, list_val);
            let zero = emit_int_const(ctx, 0);
            let ok = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: ok,
                op: BinOpIR::Gt,
                lhs: len,
                rhs: zero,
                ty: IrType::Int,
            });
            let one = emit_int_const(ctx, 1);
            let idx = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: idx,
                op: BinOpIR::Sub,
                lhs: len,
                rhs: one,
                ty: IrType::Int,
            });
            let safe_idx = fresh(ctx);
            ctx.body.push(Instr::ISelect {
                dst: safe_idx,
                cond: ok,
                then_v: idx,
                else_v: zero,
            });
            let (_size, align, stride) = collection_layout(&elem_ty, ctx.aliases, ctx.std_types)?;
            let stride_val = emit_int_const(ctx, stride as i64);
            let offset = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: offset,
                op: BinOpIR::Mul,
                lhs: safe_idx,
                rhs: stride_val,
                ty: IrType::Int,
            });
            let data_ptr = emit_collection_data_ptr(ctx, list_val);
            let cap = emit_collection_cap(ctx, list_val);
            emit_collection_payload_guard(ctx, data_ptr, len, cap, stride, align);
            let elem_ptr = emit_ptr_add(ctx, data_ptr, offset);
            let elem_val = load_value_copy(ctx, &elem_ty, elem_ptr, 0)?;
            let zero_payload = zero_value_for_type(ctx, &elem_ty)?;
            let payload = fresh(ctx);
            ctx.body.push(Instr::ISelect {
                dst: payload,
                cond: ok,
                then_v: elem_val,
                else_v: zero_payload,
            });
            let zero_hi = emit_int_const(ctx, 0);
            Ok(Some(ctx.variant_init(ok, payload, zero_hi)))
        }
        _ => Ok(None),
    }
}
