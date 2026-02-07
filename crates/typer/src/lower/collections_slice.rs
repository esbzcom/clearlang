use anyhow::Result;
use clg_ast::{Expr, Type};
use clg_ir::{BinOpIR, Instr, IrType, Value};

use super::array::{
    emit_array_data_ptr, emit_array_len, ARRAY_HEADER_ALIGN, ARRAY_HEADER_DATA_OFFSET,
    ARRAY_HEADER_LEN_OFFSET, ARRAY_HEADER_SIZE,
};
use super::block::expr_span_local;
use super::collection_types::slice_elem_type;
use super::collections_helpers::emit_collection_guard;
use super::layout::collection_layout;
use super::{emit_alloc, emit_int_const, emit_ptr_add, fresh, lower_expr, LowerCtx};

pub(super) fn lower_slice_call<'a>(
    ctx: &mut LowerCtx<'a>,
    callee: &str,
    args: &'a [Expr],
) -> Result<Option<Value>> {
    match callee {
        "std::array::len" => {
            if args.len() != 1 {
                anyhow::bail!("`std::array::len` expects one argument");
            }
            let arr_val = lower_expr(ctx, &args[0], None)?;
            Ok(Some(emit_array_len(ctx, arr_val)))
        }
        "std::slice::len" => {
            if args.len() != 1 {
                anyhow::bail!("`std::slice::len` expects one argument");
            }
            let slice_val = lower_expr(ctx, &args[0], None)?;
            Ok(Some(emit_array_len(ctx, slice_val)))
        }
        "std::slice::from_array" => {
            if args.len() != 1 {
                anyhow::bail!("`std::slice::from_array` expects one argument");
            }
            let array_val = lower_expr(ctx, &args[0], None)?;
            let len = emit_array_len(ctx, array_val);
            let data_ptr = emit_array_data_ptr(ctx, array_val);
            let header = emit_alloc(ctx, ARRAY_HEADER_SIZE, ARRAY_HEADER_ALIGN);
            ctx.body.push(Instr::Store {
                ptr: header,
                src: len,
                offset: ARRAY_HEADER_LEN_OFFSET,
                ty: IrType::Int,
            });
            ctx.body.push(Instr::Store {
                ptr: header,
                src: data_ptr,
                offset: ARRAY_HEADER_DATA_OFFSET,
                ty: IrType::Int,
            });
            Ok(Some(header))
        }
        "std::slice::sub" => {
            if args.len() != 3 {
                anyhow::bail!("`std::slice::sub` expects three arguments");
            }
            let elem_ty = slice_elem_type(ctx, &args[0])?;
            let slice_val = lower_expr(ctx, &args[0], None)?;
            let start = lower_expr(ctx, &args[1], Some(Type::Int))?;
            let len = lower_expr(ctx, &args[2], Some(Type::Int))?;
            let base_len = emit_array_len(ctx, slice_val);
            let base_ptr = emit_array_data_ptr(ctx, slice_val);
            let zero = emit_int_const(ctx, 0);
            let start_ge_zero = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: start_ge_zero,
                op: BinOpIR::Ge,
                lhs: start,
                rhs: zero,
                ty: IrType::Int,
            });
            let len_ge_zero = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: len_ge_zero,
                op: BinOpIR::Ge,
                lhs: len,
                rhs: zero,
                ty: IrType::Int,
            });
            let sum = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: sum,
                op: BinOpIR::Add,
                lhs: start,
                rhs: len,
                ty: IrType::Int,
            });
            let sum_le = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: sum_le,
                op: BinOpIR::LeU,
                lhs: sum,
                rhs: base_len,
                ty: IrType::Int,
            });
            let ok1 = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: ok1,
                op: BinOpIR::And,
                lhs: start_ge_zero,
                rhs: len_ge_zero,
                ty: IrType::Bool,
            });
            let ok = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: ok,
                op: BinOpIR::And,
                lhs: ok1,
                rhs: sum_le,
                ty: IrType::Bool,
            });
            emit_collection_guard(ctx, ok, expr_span_local(&args[0]));
            let (_size, _align, stride) = collection_layout(&elem_ty, ctx.aliases, ctx.std_types)?;
            let stride_val = emit_int_const(ctx, stride as i64);
            let offset_val = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: offset_val,
                op: BinOpIR::Mul,
                lhs: start,
                rhs: stride_val,
                ty: IrType::Int,
            });
            let data_ptr = emit_ptr_add(ctx, base_ptr, offset_val);
            let header = emit_alloc(ctx, ARRAY_HEADER_SIZE, ARRAY_HEADER_ALIGN);
            ctx.body.push(Instr::Store {
                ptr: header,
                src: len,
                offset: ARRAY_HEADER_LEN_OFFSET,
                ty: IrType::Int,
            });
            ctx.body.push(Instr::Store {
                ptr: header,
                src: data_ptr,
                offset: ARRAY_HEADER_DATA_OFFSET,
                ty: IrType::Int,
            });
            Ok(Some(header))
        }
        _ => Ok(None),
    }
}
