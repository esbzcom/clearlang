use crate::check::{base_type, infer_expr_type};
use anyhow::Result;
use clg_ast::{Expr, Type};
use clg_ir::{BinOpIR, Instr, IrType, Value};

use super::collection_types::set_elem_type;
use super::collections_helpers::{
    emit_cap_from_len, emit_collection_cap, emit_collection_data_ptr, emit_collection_header,
    emit_collection_len, emit_collection_payload_guard, emit_find_index,
};
use super::layout::collection_layout;
use super::{
    emit_alloc_dyn, emit_int_const, emit_memcpy_bytes, emit_ptr_add, fresh, lower_expr,
    store_value, LowerCtx,
};

pub(super) fn lower_set_call<'a>(
    ctx: &mut LowerCtx<'a>,
    call_expr: &'a Expr,
    callee: &str,
    args: &'a [Expr],
    expected: Option<&Type>,
) -> Result<Option<Value>> {
    match callee {
        "std::set::new" => {
            if !args.is_empty() {
                anyhow::bail!("`std::set::new` expects no arguments");
            }
            let elem_ty = match expected {
                Some(Type::Set(inner)) => *inner.clone(),
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
                        Type::Set(inner) => *inner,
                        other => anyhow::bail!("cannot infer set element type: {:?}", other),
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
        "std::set::len" => {
            if args.len() != 1 {
                anyhow::bail!("`std::set::len` expects one argument");
            }
            let set_val = lower_expr(ctx, &args[0], None)?;
            Ok(Some(emit_collection_len(ctx, set_val)))
        }
        "std::set::is_empty" => {
            if args.len() != 1 {
                anyhow::bail!("`std::set::is_empty` expects one argument");
            }
            let set_val = lower_expr(ctx, &args[0], None)?;
            let len = emit_collection_len(ctx, set_val);
            let zero = emit_int_const(ctx, 0);
            let out = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: out,
                op: BinOpIR::Eq,
                lhs: len,
                rhs: zero,
                ty: IrType::Int,
            });
            Ok(Some(out))
        }
        "std::set::contains" => {
            if args.len() != 2 {
                anyhow::bail!("`std::set::contains` expects two arguments");
            }
            let elem_ty = set_elem_type(ctx, &args[0])?;
            let set_val = lower_expr(ctx, &args[0], None)?;
            let elem_val = lower_expr(ctx, &args[1], Some(elem_ty.clone()))?;
            let len = emit_collection_len(ctx, set_val);
            let data_ptr = emit_collection_data_ptr(ctx, set_val);
            let (_size, align, stride) = collection_layout(&elem_ty, ctx.aliases, ctx.std_types)?;
            let cap = emit_collection_cap(ctx, set_val);
            emit_collection_payload_guard(ctx, data_ptr, len, cap, stride, align);
            let (found, _idx) = emit_find_index(
                ctx,
                data_ptr,
                len,
                stride,
                elem_val,
                &elem_ty,
                0,
                ctx.aliases,
            )?;
            Ok(Some(found))
        }
        "std::set::insert" => {
            if args.len() != 2 {
                anyhow::bail!("`std::set::insert` expects two arguments");
            }
            let elem_ty = set_elem_type(ctx, &args[0])?;
            let set_val = lower_expr(ctx, &args[0], None)?;
            let elem_val = lower_expr(ctx, &args[1], Some(elem_ty.clone()))?;
            let len = emit_collection_len(ctx, set_val);
            let data_ptr = emit_collection_data_ptr(ctx, set_val);
            let (_size, align, stride) = collection_layout(&elem_ty, ctx.aliases, ctx.std_types)?;
            let cap = emit_collection_cap(ctx, set_val);
            emit_collection_payload_guard(ctx, data_ptr, len, cap, stride, align);
            let (found, _idx) = emit_find_index(
                ctx,
                data_ptr,
                len,
                stride,
                elem_val,
                &elem_ty,
                0,
                ctx.aliases,
            )?;
            let one = emit_int_const(ctx, 1);
            let len_plus_one = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: len_plus_one,
                op: BinOpIR::Add,
                lhs: len,
                rhs: one,
                ty: IrType::Int,
            });
            let new_len = fresh(ctx);
            ctx.body.push(Instr::ISelect {
                dst: new_len,
                cond: found,
                then_v: len,
                else_v: len_plus_one,
            });
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
            let copy_bytes = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: copy_bytes,
                op: BinOpIR::Mul,
                lhs: len,
                rhs: stride_val,
                ty: IrType::Int,
            });
            emit_memcpy_bytes(ctx, data_ptr, new_data, copy_bytes)?;
            ctx.body.push(Instr::BlockBegin);
            ctx.body.push(Instr::BrIf {
                cond: found,
                depth: 0,
            });
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
            ctx.body.push(Instr::BlockEnd);
            let header = emit_collection_header(ctx, new_len, new_cap, new_data);
            Ok(Some(header))
        }
        "std::set::remove" => {
            if args.len() != 2 {
                anyhow::bail!("`std::set::remove` expects two arguments");
            }
            let elem_ty = set_elem_type(ctx, &args[0])?;
            let set_val = lower_expr(ctx, &args[0], None)?;
            let elem_val = lower_expr(ctx, &args[1], Some(elem_ty.clone()))?;
            let len = emit_collection_len(ctx, set_val);
            let data_ptr = emit_collection_data_ptr(ctx, set_val);
            let (_size, align, stride) = collection_layout(&elem_ty, ctx.aliases, ctx.std_types)?;
            let cap = emit_collection_cap(ctx, set_val);
            emit_collection_payload_guard(ctx, data_ptr, len, cap, stride, align);
            let (found, found_idx) = emit_find_index(
                ctx,
                data_ptr,
                len,
                stride,
                elem_val,
                &elem_ty,
                0,
                ctx.aliases,
            )?;
            let one = emit_int_const(ctx, 1);
            let len_minus_one = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: len_minus_one,
                op: BinOpIR::Sub,
                lhs: len,
                rhs: one,
                ty: IrType::Int,
            });
            let new_len = fresh(ctx);
            ctx.body.push(Instr::ISelect {
                dst: new_len,
                cond: found,
                then_v: len_minus_one,
                else_v: len,
            });
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
            ctx.body.push(Instr::BlockBegin);
            ctx.body.push(Instr::BlockBegin);
            ctx.body.push(Instr::BrIfEqz {
                cond: found,
                depth: 0,
            });
            let bytes_before = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: bytes_before,
                op: BinOpIR::Mul,
                lhs: found_idx,
                rhs: stride_val,
                ty: IrType::Int,
            });
            emit_memcpy_bytes(ctx, data_ptr, new_data, bytes_before)?;
            let idx_plus_one = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: idx_plus_one,
                op: BinOpIR::Add,
                lhs: found_idx,
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
            let src_ptr = emit_ptr_add(ctx, data_ptr, src_offset);
            let dst_offset = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: dst_offset,
                op: BinOpIR::Mul,
                lhs: found_idx,
                rhs: stride_val,
                ty: IrType::Int,
            });
            let dst_ptr = emit_ptr_add(ctx, new_data, dst_offset);
            emit_memcpy_bytes(ctx, src_ptr, dst_ptr, bytes_after)?;
            ctx.body.push(Instr::Br { depth: 1 });
            ctx.body.push(Instr::BlockEnd);
            let copy_bytes = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: copy_bytes,
                op: BinOpIR::Mul,
                lhs: len,
                rhs: stride_val,
                ty: IrType::Int,
            });
            emit_memcpy_bytes(ctx, data_ptr, new_data, copy_bytes)?;
            ctx.body.push(Instr::BlockEnd);
            let header = emit_collection_header(ctx, new_len, new_cap, new_data);
            Ok(Some(header))
        }
        _ => Ok(None),
    }
}
