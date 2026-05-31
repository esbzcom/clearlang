use anyhow::Result;
use clg_ast::{Expr, Type};
use clg_ir::{BinOpIR, Instr, IrType, Value};

use super::array::{emit_bytes_data_ptr, emit_bytes_len, BYTES_HEADER_DATA_OFFSET};
use super::layout::tuple_layout;
use super::{
    emit_alloc, emit_alloc_dyn, emit_bool_const, emit_int_const, emit_memcpy_bytes, emit_ptr_add,
    emit_store_i32, fresh, lower_expr, store_value, LowerCtx,
};

const BYTES_HEADER_ALIGN: u32 = 4;
const ENCODER_BUF_OFFSET: u32 = 0;
const DECODER_INPUT_OFFSET: u32 = 0;
const DECODER_POS_OFFSET: u32 = 4;
const DECODE_ERROR_CODE_OFFSET: u32 = 0;
const DECODE_ERROR_OFFSET_OFFSET: u32 = 4;
const ENCODE_ERROR_CODE_OFFSET: u32 = 0;
const ERROR_CODE_VALUE_OFFSET: u32 = 0;

const DECODE_ERR_TRUNCATED: i64 = 1;
const DECODE_ERR_INVALID_LENGTH: i64 = 2;
const DECODE_ERR_INVALID_BOOL: i64 = 3;

fn encoder_type() -> Type {
    Type::Named {
        name: "std::encoder::Encoder".to_string(),
        args: Vec::new(),
    }
}

fn decoder_type() -> Type {
    Type::Named {
        name: "std::decoder::Decoder".to_string(),
        args: Vec::new(),
    }
}

fn decode_error_type() -> Type {
    Type::Named {
        name: "std::decode_error::DecodeError".to_string(),
        args: Vec::new(),
    }
}

fn encode_error_type() -> Type {
    Type::Named {
        name: "std::encode_error::EncodeError".to_string(),
        args: Vec::new(),
    }
}

fn emit_u8_const(ctx: &mut LowerCtx<'_>, value: u8) -> Value {
    let dst = fresh(ctx);
    ctx.body.push(Instr::IConst {
        dst,
        ty: IrType::U8,
        n: value as i64,
    });
    dst
}

fn emit_load_i32(ctx: &mut LowerCtx<'_>, ptr: Value, offset: u32) -> Value {
    let dst = fresh(ctx);
    ctx.body.push(Instr::Load {
        dst,
        ptr,
        offset,
        ty: IrType::Int,
    });
    dst
}

fn emit_load_u8(ctx: &mut LowerCtx<'_>, ptr: Value, offset: u32) -> Value {
    let dst = fresh(ctx);
    ctx.body.push(Instr::Load {
        dst,
        ptr,
        offset,
        ty: IrType::U8,
    });
    dst
}

fn emit_ptr_add_const(ctx: &mut LowerCtx<'_>, ptr: Value, offset: u32) -> Value {
    let off = emit_int_const(ctx, offset as i64);
    emit_ptr_add(ctx, ptr, off)
}

fn emit_bytes_alloc(ctx: &mut LowerCtx<'_>, len: Value) -> Value {
    let header = emit_int_const(ctx, BYTES_HEADER_DATA_OFFSET as i64);
    let total = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: total,
        op: BinOpIR::Add,
        lhs: len,
        rhs: header,
        ty: IrType::Int,
    });
    let out = emit_alloc_dyn(ctx, total, BYTES_HEADER_ALIGN);
    emit_store_i32(ctx, out, 0, len);
    out
}

fn emit_empty_bytes(ctx: &mut LowerCtx<'_>) -> Value {
    let out = emit_alloc(ctx, BYTES_HEADER_DATA_OFFSET, BYTES_HEADER_ALIGN);
    let zero = emit_int_const(ctx, 0);
    emit_store_i32(ctx, out, 0, zero);
    out
}

fn emit_fixed_zero_bytes(ctx: &mut LowerCtx<'_>, len: u32) -> Value {
    let out = emit_alloc(ctx, BYTES_HEADER_DATA_OFFSET + len, BYTES_HEADER_ALIGN);
    let len_val = emit_int_const(ctx, len as i64);
    emit_store_i32(ctx, out, 0, len_val);
    let data_ptr = emit_ptr_add_const(ctx, out, BYTES_HEADER_DATA_OFFSET);
    let zero = emit_u8_const(ctx, 0);
    for idx in 0..len {
        ctx.body.push(Instr::Store {
            ptr: data_ptr,
            src: zero,
            offset: idx,
            ty: IrType::U8,
        });
    }
    out
}

fn emit_int_prefix_bytes(ctx: &mut LowerCtx<'_>, value: Value) -> Value {
    let out = emit_alloc(ctx, BYTES_HEADER_DATA_OFFSET + 4, BYTES_HEADER_ALIGN);
    let len_val = emit_int_const(ctx, 4);
    emit_store_i32(ctx, out, 0, len_val);
    ctx.body.push(Instr::Store {
        ptr: out,
        src: value,
        offset: BYTES_HEADER_DATA_OFFSET,
        ty: IrType::Int,
    });
    out
}

fn emit_bytes_slice(
    ctx: &mut LowerCtx<'_>,
    bytes_val: Value,
    start: Value,
    len: Value,
) -> Result<Value> {
    let out = emit_bytes_alloc(ctx, len);
    let src_data = emit_bytes_data_ptr(ctx, bytes_val);
    let src_ptr = emit_ptr_add(ctx, src_data, start);
    let dst_data = emit_bytes_data_ptr(ctx, out);
    emit_memcpy_bytes(ctx, src_ptr, dst_data, len)?;
    Ok(out)
}

fn emit_fixed_zero_padded_slice(
    ctx: &mut LowerCtx<'_>,
    bytes_val: Value,
    start: Value,
    copy_len: Value,
    out_len: u32,
) -> Result<Value> {
    let out = emit_fixed_zero_bytes(ctx, out_len);
    let src_data = emit_bytes_data_ptr(ctx, bytes_val);
    let src_ptr = emit_ptr_add(ctx, src_data, start);
    let dst_data = emit_bytes_data_ptr(ctx, out);
    emit_memcpy_bytes(ctx, src_ptr, dst_data, copy_len)?;
    Ok(out)
}

fn emit_bytes_concat(ctx: &mut LowerCtx<'_>, lhs: Value, rhs: Value) -> Result<Value> {
    let lhs_len = emit_bytes_len(ctx, lhs);
    let rhs_len = emit_bytes_len(ctx, rhs);
    let total_len = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: total_len,
        op: BinOpIR::Add,
        lhs: lhs_len,
        rhs: rhs_len,
        ty: IrType::Int,
    });
    let out = emit_bytes_alloc(ctx, total_len);
    let out_data = emit_bytes_data_ptr(ctx, out);

    let lhs_data = emit_bytes_data_ptr(ctx, lhs);
    emit_memcpy_bytes(ctx, lhs_data, out_data, lhs_len)?;

    let rhs_dst = emit_ptr_add(ctx, out_data, lhs_len);
    let rhs_data = emit_bytes_data_ptr(ctx, rhs);
    emit_memcpy_bytes(ctx, rhs_data, rhs_dst, rhs_len)?;
    Ok(out)
}

fn emit_make_encoder(ctx: &mut LowerCtx<'_>, bytes_ptr: Value) -> Value {
    let out = emit_alloc(ctx, 4, 4);
    emit_store_i32(ctx, out, ENCODER_BUF_OFFSET, bytes_ptr);
    out
}

fn emit_encoder_bytes(ctx: &mut LowerCtx<'_>, encoder: Value) -> Value {
    emit_load_i32(ctx, encoder, ENCODER_BUF_OFFSET)
}

fn emit_make_decoder(ctx: &mut LowerCtx<'_>, input: Value, pos: Value) -> Value {
    let out = emit_alloc(ctx, 8, 4);
    emit_store_i32(ctx, out, DECODER_INPUT_OFFSET, input);
    emit_store_i32(ctx, out, DECODER_POS_OFFSET, pos);
    out
}

fn emit_decoder_input(ctx: &mut LowerCtx<'_>, decoder: Value) -> Value {
    emit_load_i32(ctx, decoder, DECODER_INPUT_OFFSET)
}

fn emit_decoder_pos(ctx: &mut LowerCtx<'_>, decoder: Value) -> Value {
    emit_load_i32(ctx, decoder, DECODER_POS_OFFSET)
}

fn emit_decoder_remaining(ctx: &mut LowerCtx<'_>, decoder: Value) -> Value {
    let input = emit_decoder_input(ctx, decoder);
    let total = emit_bytes_len(ctx, input);
    let pos = emit_decoder_pos(ctx, decoder);
    let remaining = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: remaining,
        op: BinOpIR::Sub,
        lhs: total,
        rhs: pos,
        ty: IrType::Int,
    });
    remaining
}

fn emit_make_decode_error(ctx: &mut LowerCtx<'_>, code: Value, offset: Value) -> Value {
    let out = emit_alloc(ctx, 8, 4);
    emit_store_i32(ctx, out, DECODE_ERROR_CODE_OFFSET, code);
    emit_store_i32(ctx, out, DECODE_ERROR_OFFSET_OFFSET, offset);
    out
}

fn emit_make_error_code(ctx: &mut LowerCtx<'_>, code: Value) -> Value {
    let out = emit_alloc(ctx, 4, 4);
    emit_store_i32(ctx, out, ERROR_CODE_VALUE_OFFSET, code);
    out
}

fn emit_make_tuple(ctx: &mut LowerCtx<'_>, elem_tys: &[Type], values: &[Value]) -> Result<Value> {
    let layout = tuple_layout(elem_tys, ctx.aliases, ctx.std_types)?;
    let out = emit_alloc(ctx, layout.size, layout.align);
    for ((elem_ty, value), offset) in elem_tys
        .iter()
        .zip(values.iter())
        .zip(layout.offsets.iter())
    {
        store_value(ctx, elem_ty, out, *offset, *value)?;
    }
    Ok(out)
}

fn emit_ok_result(ctx: &mut LowerCtx<'_>, payload: Value) -> Value {
    let tag = emit_int_const(ctx, 1);
    let zero = emit_int_const(ctx, 0);
    ctx.variant_init(tag, payload, zero)
}

fn emit_select_result(
    ctx: &mut LowerCtx<'_>,
    ok: Value,
    ok_payload: Value,
    err_payload: Value,
) -> Value {
    let payload = fresh(ctx);
    ctx.body.push(Instr::ISelect {
        dst: payload,
        cond: ok,
        then_v: ok_payload,
        else_v: err_payload,
    });
    let zero = emit_int_const(ctx, 0);
    let one = emit_int_const(ctx, 1);
    let tag = fresh(ctx);
    ctx.body.push(Instr::ISelect {
        dst: tag,
        cond: ok,
        then_v: one,
        else_v: zero,
    });
    let zero_hi = emit_int_const(ctx, 0);
    ctx.variant_init(tag, payload, zero_hi)
}

fn emit_intrinsic_call(ctx: &mut LowerCtx<'_>, callee: &str, args: Vec<Value>) -> Result<Value> {
    let idx = ctx
        .fn_indices
        .get(callee)
        .copied()
        .ok_or_else(|| anyhow::anyhow!("missing intrinsic `{}`", callee))?;
    let out = fresh(ctx);
    ctx.body.push(Instr::Call {
        dst: Some(out),
        callee: idx,
        args,
    });
    Ok(out)
}

pub(super) fn lower_codec_call<'a>(
    ctx: &mut LowerCtx<'a>,
    callee: &str,
    args: &'a [Expr],
) -> Result<Option<Value>> {
    match callee {
        "std::encoder::new" => {
            if !args.is_empty() {
                anyhow::bail!("`std::encoder::new` expects no arguments");
            }
            let empty = emit_empty_bytes(ctx);
            let out = emit_make_encoder(ctx, empty);
            Ok(Some(out))
        }
        "std::encoder::write_u64" => {
            if args.len() != 2 {
                anyhow::bail!("`std::encoder::write_u64` expects two arguments");
            }
            let encoder = lower_expr(ctx, &args[0], Some(encoder_type()))?;
            let value = lower_expr(ctx, &args[1], Some(Type::U64))?;
            let encoded = emit_intrinsic_call(ctx, "std::u64::to_bytes_le", vec![value])?;
            let encoder_bytes = emit_encoder_bytes(ctx, encoder);
            let out_bytes = emit_bytes_concat(ctx, encoder_bytes, encoded)?;
            let out = emit_make_encoder(ctx, out_bytes);
            Ok(Some(emit_ok_result(ctx, out)))
        }
        "std::encoder::write_bool" => {
            if args.len() != 2 {
                anyhow::bail!("`std::encoder::write_bool` expects two arguments");
            }
            let encoder = lower_expr(ctx, &args[0], Some(encoder_type()))?;
            let value = lower_expr(ctx, &args[1], Some(Type::Bool))?;
            let encoded = emit_fixed_zero_bytes(ctx, 1);
            let data_ptr = emit_bytes_data_ptr(ctx, encoded);
            let one = emit_u8_const(ctx, 1);
            let zero = emit_u8_const(ctx, 0);
            let byte = fresh(ctx);
            ctx.body.push(Instr::ISelect {
                dst: byte,
                cond: value,
                then_v: one,
                else_v: zero,
            });
            ctx.body.push(Instr::Store {
                ptr: data_ptr,
                src: byte,
                offset: 0,
                ty: IrType::U8,
            });
            let encoder_bytes = emit_encoder_bytes(ctx, encoder);
            let out_bytes = emit_bytes_concat(ctx, encoder_bytes, encoded)?;
            let out = emit_make_encoder(ctx, out_bytes);
            Ok(Some(emit_ok_result(ctx, out)))
        }
        "std::encoder::write_bytes" => {
            if args.len() != 2 {
                anyhow::bail!("`std::encoder::write_bytes` expects two arguments");
            }
            let encoder = lower_expr(ctx, &args[0], Some(encoder_type()))?;
            let value = lower_expr(ctx, &args[1], Some(Type::Bytes))?;
            let value_len = emit_bytes_len(ctx, value);
            let prefix = emit_int_prefix_bytes(ctx, value_len);
            let framed = emit_bytes_concat(ctx, prefix, value)?;
            let encoder_bytes = emit_encoder_bytes(ctx, encoder);
            let out_bytes = emit_bytes_concat(ctx, encoder_bytes, framed)?;
            let out = emit_make_encoder(ctx, out_bytes);
            Ok(Some(emit_ok_result(ctx, out)))
        }
        "std::encoder::finish" => {
            if args.len() != 1 {
                anyhow::bail!("`std::encoder::finish` expects one argument");
            }
            let encoder = lower_expr(ctx, &args[0], Some(encoder_type()))?;
            let encoder_bytes = emit_encoder_bytes(ctx, encoder);
            Ok(Some(emit_ok_result(ctx, encoder_bytes)))
        }
        "std::decoder::new" => {
            if args.len() != 1 {
                anyhow::bail!("`std::decoder::new` expects one argument");
            }
            let input = lower_expr(ctx, &args[0], Some(Type::Bytes))?;
            let zero = emit_int_const(ctx, 0);
            Ok(Some(emit_make_decoder(ctx, input, zero)))
        }
        "std::decoder::read_u64" => {
            if args.len() != 1 {
                anyhow::bail!("`std::decoder::read_u64` expects one argument");
            }
            let decoder = lower_expr(ctx, &args[0], Some(decoder_type()))?;
            let input = emit_decoder_input(ctx, decoder);
            let pos = emit_decoder_pos(ctx, decoder);
            let remaining = emit_decoder_remaining(ctx, decoder);
            let need = emit_int_const(ctx, 8);
            let ok = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: ok,
                op: BinOpIR::Ge,
                lhs: remaining,
                rhs: need,
                ty: IrType::Int,
            });
            let zero = emit_int_const(ctx, 0);
            let copy_len = fresh(ctx);
            ctx.body.push(Instr::ISelect {
                dst: copy_len,
                cond: ok,
                then_v: need,
                else_v: zero,
            });
            let fixed = emit_fixed_zero_padded_slice(ctx, input, pos, copy_len, 8)?;
            let value = emit_intrinsic_call(ctx, "std::u64::from_bytes_le", vec![fixed])?;
            let advance = fresh(ctx);
            ctx.body.push(Instr::ISelect {
                dst: advance,
                cond: ok,
                then_v: need,
                else_v: zero,
            });
            let next_pos = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: next_pos,
                op: BinOpIR::Add,
                lhs: pos,
                rhs: advance,
                ty: IrType::Int,
            });
            let next_decoder = emit_make_decoder(ctx, input, next_pos);
            let ok_tuple =
                emit_make_tuple(ctx, &[decoder_type(), Type::U64], &[next_decoder, value])?;
            let trunc_code = emit_int_const(ctx, DECODE_ERR_TRUNCATED);
            let err = emit_make_decode_error(ctx, trunc_code, pos);
            Ok(Some(emit_select_result(ctx, ok, ok_tuple, err)))
        }
        "std::decoder::read_bool" => {
            if args.len() != 1 {
                anyhow::bail!("`std::decoder::read_bool` expects one argument");
            }
            let decoder = lower_expr(ctx, &args[0], Some(decoder_type()))?;
            let input = emit_decoder_input(ctx, decoder);
            let pos = emit_decoder_pos(ctx, decoder);
            let remaining = emit_decoder_remaining(ctx, decoder);
            let need = emit_int_const(ctx, 1);
            let has_byte = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: has_byte,
                op: BinOpIR::Ge,
                lhs: remaining,
                rhs: need,
                ty: IrType::Int,
            });
            let zero = emit_int_const(ctx, 0);
            let copy_len = fresh(ctx);
            ctx.body.push(Instr::ISelect {
                dst: copy_len,
                cond: has_byte,
                then_v: need,
                else_v: zero,
            });
            let fixed = emit_fixed_zero_padded_slice(ctx, input, pos, copy_len, 1)?;
            let byte_ptr = emit_bytes_data_ptr(ctx, fixed);
            let byte = emit_load_u8(ctx, byte_ptr, 0);
            let zero_u8 = emit_u8_const(ctx, 0);
            let one_u8 = emit_u8_const(ctx, 1);
            let is_zero = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: is_zero,
                op: BinOpIR::Eq,
                lhs: byte,
                rhs: zero_u8,
                ty: IrType::U8,
            });
            let is_one = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: is_one,
                op: BinOpIR::Eq,
                lhs: byte,
                rhs: one_u8,
                ty: IrType::U8,
            });
            let byte_ok = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: byte_ok,
                op: BinOpIR::Or,
                lhs: is_zero,
                rhs: is_one,
                ty: IrType::Bool,
            });
            let ok = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: ok,
                op: BinOpIR::And,
                lhs: has_byte,
                rhs: byte_ok,
                ty: IrType::Bool,
            });
            let bool_val = emit_bool_const(ctx, false);
            let true_val = emit_bool_const(ctx, true);
            let payload_bool = fresh(ctx);
            ctx.body.push(Instr::ISelect {
                dst: payload_bool,
                cond: is_one,
                then_v: true_val,
                else_v: bool_val,
            });
            let advance = fresh(ctx);
            ctx.body.push(Instr::ISelect {
                dst: advance,
                cond: ok,
                then_v: need,
                else_v: zero,
            });
            let next_pos = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: next_pos,
                op: BinOpIR::Add,
                lhs: pos,
                rhs: advance,
                ty: IrType::Int,
            });
            let next_decoder = emit_make_decoder(ctx, input, next_pos);
            let ok_tuple = emit_make_tuple(
                ctx,
                &[decoder_type(), Type::Bool],
                &[next_decoder, payload_bool],
            )?;
            let invalid_code = emit_int_const(ctx, DECODE_ERR_INVALID_BOOL);
            let trunc_code = emit_int_const(ctx, DECODE_ERR_TRUNCATED);
            let err_code = fresh(ctx);
            ctx.body.push(Instr::ISelect {
                dst: err_code,
                cond: has_byte,
                then_v: invalid_code,
                else_v: trunc_code,
            });
            let err = emit_make_decode_error(ctx, err_code, pos);
            Ok(Some(emit_select_result(ctx, ok, ok_tuple, err)))
        }
        "std::decoder::read_bytes" => {
            if args.len() != 1 {
                anyhow::bail!("`std::decoder::read_bytes` expects one argument");
            }
            let decoder = lower_expr(ctx, &args[0], Some(decoder_type()))?;
            let input = emit_decoder_input(ctx, decoder);
            let pos = emit_decoder_pos(ctx, decoder);
            let remaining = emit_decoder_remaining(ctx, decoder);
            let prefix_len = emit_int_const(ctx, 4);
            let has_prefix = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: has_prefix,
                op: BinOpIR::Ge,
                lhs: remaining,
                rhs: prefix_len,
                ty: IrType::Int,
            });
            let zero = emit_int_const(ctx, 0);
            let prefix_copy_len = fresh(ctx);
            ctx.body.push(Instr::ISelect {
                dst: prefix_copy_len,
                cond: has_prefix,
                then_v: prefix_len,
                else_v: zero,
            });
            let prefix = emit_fixed_zero_padded_slice(ctx, input, pos, prefix_copy_len, 4)?;
            let prefix_ptr = emit_bytes_data_ptr(ctx, prefix);
            let declared_len = emit_load_i32(ctx, prefix_ptr, 0);
            let declared_nonneg = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: declared_nonneg,
                op: BinOpIR::Ge,
                lhs: declared_len,
                rhs: zero,
                ty: IrType::Int,
            });
            let after_prefix = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: after_prefix,
                op: BinOpIR::Sub,
                lhs: remaining,
                rhs: prefix_len,
                ty: IrType::Int,
            });
            let payload_fits = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: payload_fits,
                op: BinOpIR::Le,
                lhs: declared_len,
                rhs: after_prefix,
                ty: IrType::Int,
            });
            let tmp = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: tmp,
                op: BinOpIR::And,
                lhs: has_prefix,
                rhs: declared_nonneg,
                ty: IrType::Bool,
            });
            let ok = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: ok,
                op: BinOpIR::And,
                lhs: tmp,
                rhs: payload_fits,
                ty: IrType::Bool,
            });
            let safe_len = fresh(ctx);
            ctx.body.push(Instr::ISelect {
                dst: safe_len,
                cond: ok,
                then_v: declared_len,
                else_v: zero,
            });
            let payload_start = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: payload_start,
                op: BinOpIR::Add,
                lhs: pos,
                rhs: prefix_len,
                ty: IrType::Int,
            });
            let bytes_out = emit_bytes_slice(ctx, input, payload_start, safe_len)?;
            let total_advance = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: total_advance,
                op: BinOpIR::Add,
                lhs: prefix_len,
                rhs: safe_len,
                ty: IrType::Int,
            });
            let next_pos = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: next_pos,
                op: BinOpIR::Add,
                lhs: pos,
                rhs: total_advance,
                ty: IrType::Int,
            });
            let next_decoder = emit_make_decoder(ctx, input, next_pos);
            let ok_tuple = emit_make_tuple(
                ctx,
                &[decoder_type(), Type::Bytes],
                &[next_decoder, bytes_out],
            )?;
            let neg_code = emit_int_const(ctx, DECODE_ERR_INVALID_LENGTH);
            let trunc_code = emit_int_const(ctx, DECODE_ERR_TRUNCATED);
            let base_err_code = fresh(ctx);
            ctx.body.push(Instr::ISelect {
                dst: base_err_code,
                cond: declared_nonneg,
                then_v: trunc_code,
                else_v: neg_code,
            });
            let err_code = fresh(ctx);
            ctx.body.push(Instr::ISelect {
                dst: err_code,
                cond: has_prefix,
                then_v: base_err_code,
                else_v: trunc_code,
            });
            let err = emit_make_decode_error(ctx, err_code, pos);
            Ok(Some(emit_select_result(ctx, ok, ok_tuple, err)))
        }
        "std::decoder::read_fixed" => {
            if args.len() != 2 {
                anyhow::bail!("`std::decoder::read_fixed` expects two arguments");
            }
            let decoder = lower_expr(ctx, &args[0], Some(decoder_type()))?;
            let requested = lower_expr(ctx, &args[1], Some(Type::Int))?;
            let input = emit_decoder_input(ctx, decoder);
            let pos = emit_decoder_pos(ctx, decoder);
            let remaining = emit_decoder_remaining(ctx, decoder);
            let zero = emit_int_const(ctx, 0);
            let nonneg = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: nonneg,
                op: BinOpIR::Ge,
                lhs: requested,
                rhs: zero,
                ty: IrType::Int,
            });
            let fits = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: fits,
                op: BinOpIR::Le,
                lhs: requested,
                rhs: remaining,
                ty: IrType::Int,
            });
            let ok = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: ok,
                op: BinOpIR::And,
                lhs: nonneg,
                rhs: fits,
                ty: IrType::Bool,
            });
            let safe_len = fresh(ctx);
            ctx.body.push(Instr::ISelect {
                dst: safe_len,
                cond: ok,
                then_v: requested,
                else_v: zero,
            });
            let bytes_out = emit_bytes_slice(ctx, input, pos, safe_len)?;
            let next_pos = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: next_pos,
                op: BinOpIR::Add,
                lhs: pos,
                rhs: safe_len,
                ty: IrType::Int,
            });
            let next_decoder = emit_make_decoder(ctx, input, next_pos);
            let ok_tuple = emit_make_tuple(
                ctx,
                &[decoder_type(), Type::Bytes],
                &[next_decoder, bytes_out],
            )?;
            let invalid_code = emit_int_const(ctx, DECODE_ERR_INVALID_LENGTH);
            let trunc_code = emit_int_const(ctx, DECODE_ERR_TRUNCATED);
            let err_code = fresh(ctx);
            ctx.body.push(Instr::ISelect {
                dst: err_code,
                cond: nonneg,
                then_v: trunc_code,
                else_v: invalid_code,
            });
            let err = emit_make_decode_error(ctx, err_code, pos);
            Ok(Some(emit_select_result(ctx, ok, ok_tuple, err)))
        }
        "std::decoder::position" => {
            if args.len() != 1 {
                anyhow::bail!("`std::decoder::position` expects one argument");
            }
            let decoder = lower_expr(ctx, &args[0], Some(decoder_type()))?;
            Ok(Some(emit_decoder_pos(ctx, decoder)))
        }
        "std::decoder::remaining" => {
            if args.len() != 1 {
                anyhow::bail!("`std::decoder::remaining` expects one argument");
            }
            let decoder = lower_expr(ctx, &args[0], Some(decoder_type()))?;
            Ok(Some(emit_decoder_remaining(ctx, decoder)))
        }
        "std::decoder::is_eof" => {
            if args.len() != 1 {
                anyhow::bail!("`std::decoder::is_eof` expects one argument");
            }
            let decoder = lower_expr(ctx, &args[0], Some(decoder_type()))?;
            let remaining = emit_decoder_remaining(ctx, decoder);
            let zero = emit_int_const(ctx, 0);
            let out = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: out,
                op: BinOpIR::Eq,
                lhs: remaining,
                rhs: zero,
                ty: IrType::Int,
            });
            Ok(Some(out))
        }
        "std::decode_error::code" => {
            if args.len() != 1 {
                anyhow::bail!("`std::decode_error::code` expects one argument");
            }
            let err = lower_expr(ctx, &args[0], Some(decode_error_type()))?;
            let code = emit_load_i32(ctx, err, DECODE_ERROR_CODE_OFFSET);
            Ok(Some(emit_make_error_code(ctx, code)))
        }
        "std::decode_error::offset" => {
            if args.len() != 1 {
                anyhow::bail!("`std::decode_error::offset` expects one argument");
            }
            let err = lower_expr(ctx, &args[0], Some(decode_error_type()))?;
            let offset = emit_load_i32(ctx, err, DECODE_ERROR_OFFSET_OFFSET);
            let zero = emit_int_const(ctx, 0);
            let some = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: some,
                op: BinOpIR::Ge,
                lhs: offset,
                rhs: zero,
                ty: IrType::Int,
            });
            let payload = fresh(ctx);
            ctx.body.push(Instr::ISelect {
                dst: payload,
                cond: some,
                then_v: offset,
                else_v: zero,
            });
            let zero_hi = emit_int_const(ctx, 0);
            Ok(Some(ctx.variant_init(some, payload, zero_hi)))
        }
        "std::decode_error::equals" => {
            if args.len() != 2 {
                anyhow::bail!("`std::decode_error::equals` expects two arguments");
            }
            let lhs = lower_expr(ctx, &args[0], Some(decode_error_type()))?;
            let rhs = lower_expr(ctx, &args[1], Some(decode_error_type()))?;
            let lhs_code = emit_load_i32(ctx, lhs, DECODE_ERROR_CODE_OFFSET);
            let rhs_code = emit_load_i32(ctx, rhs, DECODE_ERROR_CODE_OFFSET);
            let code_eq = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: code_eq,
                op: BinOpIR::Eq,
                lhs: lhs_code,
                rhs: rhs_code,
                ty: IrType::Int,
            });
            let lhs_offset = emit_load_i32(ctx, lhs, DECODE_ERROR_OFFSET_OFFSET);
            let rhs_offset = emit_load_i32(ctx, rhs, DECODE_ERROR_OFFSET_OFFSET);
            let offset_eq = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: offset_eq,
                op: BinOpIR::Eq,
                lhs: lhs_offset,
                rhs: rhs_offset,
                ty: IrType::Int,
            });
            let out = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: out,
                op: BinOpIR::And,
                lhs: code_eq,
                rhs: offset_eq,
                ty: IrType::Bool,
            });
            Ok(Some(out))
        }
        "std::encode_error::code" => {
            if args.len() != 1 {
                anyhow::bail!("`std::encode_error::code` expects one argument");
            }
            let err = lower_expr(ctx, &args[0], Some(encode_error_type()))?;
            let code = emit_load_i32(ctx, err, ENCODE_ERROR_CODE_OFFSET);
            Ok(Some(emit_make_error_code(ctx, code)))
        }
        "std::encode_error::equals" => {
            if args.len() != 2 {
                anyhow::bail!("`std::encode_error::equals` expects two arguments");
            }
            let lhs = lower_expr(ctx, &args[0], Some(encode_error_type()))?;
            let rhs = lower_expr(ctx, &args[1], Some(encode_error_type()))?;
            let lhs_code = emit_load_i32(ctx, lhs, ENCODE_ERROR_CODE_OFFSET);
            let rhs_code = emit_load_i32(ctx, rhs, ENCODE_ERROR_CODE_OFFSET);
            let out = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: out,
                op: BinOpIR::Eq,
                lhs: lhs_code,
                rhs: rhs_code,
                ty: IrType::Int,
            });
            Ok(Some(out))
        }
        _ => Ok(None),
    }
}
