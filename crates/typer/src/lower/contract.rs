use anyhow::Result;
use clg_ast::{BinOp, Expr, Type};
use clg_ir::{BinOpIR, Instr, IrType, Value};

use super::array::{emit_bytes_data_ptr, emit_bytes_len, BYTES_HEADER_DATA_OFFSET};
use super::u64_ops::{emit_u64_bin, emit_u64_const, emit_u64_overflow_flag};
use super::{
    emit_alloc, emit_alloc_dyn, emit_bool_const, emit_int_const, emit_memcpy_bytes, fresh,
    lower_expr, LowerCtx,
};

const ADDRESS_LEN: u32 = 20;
const ADDRESS_ALIGN: u32 = 1;
const AMOUNT_ALIGN: u32 = 8;
const AMOUNT_VALUE_OFFSET: u32 = 0;
const EVENT_ALIGN: u32 = 4;
const EVENT_TOPIC_OFFSET: u32 = 0;
const EVENT_PAYLOAD_OFFSET: u32 = 4;
const CONTRACT_ERROR_ALIGN: u32 = 4;
const CONTRACT_ERROR_CODE_OFFSET: u32 = 0;
const ERROR_CODE_ALIGN: u32 = 4;
const ERROR_CODE_VALUE_OFFSET: u32 = 0;

const CONTRACT_ERR_INVALID_ADDRESS: i64 = 1;
const CONTRACT_ERR_AMOUNT_OVERFLOW: i64 = 2;
const CONTRACT_ERR_AMOUNT_UNDERFLOW: i64 = 3;

fn address_type() -> Type {
    Type::Named {
        name: "std::contract::Address".to_string(),
        args: Vec::new(),
    }
}

fn amount_type() -> Type {
    Type::Named {
        name: "std::contract::Amount".to_string(),
        args: Vec::new(),
    }
}

fn event_type() -> Type {
    Type::Named {
        name: "std::contract::Event".to_string(),
        args: Vec::new(),
    }
}

fn contract_error_type() -> Type {
    Type::Named {
        name: "std::contract::ContractError".to_string(),
        args: Vec::new(),
    }
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

fn emit_load_u64(ctx: &mut LowerCtx<'_>, ptr: Value, offset: u32) -> Value {
    let dst = fresh(ctx);
    ctx.body.push(Instr::Load {
        dst,
        ptr,
        offset,
        ty: IrType::U64,
    });
    dst
}

fn emit_store_i32(ctx: &mut LowerCtx<'_>, ptr: Value, offset: u32, src: Value) {
    ctx.body.push(Instr::Store {
        ptr,
        src,
        offset,
        ty: IrType::Int,
    });
}

fn emit_store_u64(ctx: &mut LowerCtx<'_>, ptr: Value, offset: u32, src: Value) {
    ctx.body.push(Instr::Store {
        ptr,
        src,
        offset,
        ty: IrType::U64,
    });
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
    let out = emit_alloc_dyn(ctx, total, 4);
    emit_store_i32(ctx, out, 0, len);
    out
}

fn emit_make_contract_error(ctx: &mut LowerCtx<'_>, code: Value) -> Value {
    let out = emit_alloc(ctx, 4, CONTRACT_ERROR_ALIGN);
    emit_store_i32(ctx, out, CONTRACT_ERROR_CODE_OFFSET, code);
    out
}

fn emit_make_error_code(ctx: &mut LowerCtx<'_>, code: Value) -> Value {
    let out = emit_alloc(ctx, 4, ERROR_CODE_ALIGN);
    emit_store_i32(ctx, out, ERROR_CODE_VALUE_OFFSET, code);
    out
}

fn emit_make_amount(ctx: &mut LowerCtx<'_>, value: Value) -> Value {
    let out = emit_alloc(ctx, 8, AMOUNT_ALIGN);
    emit_store_u64(ctx, out, AMOUNT_VALUE_OFFSET, value);
    out
}

fn emit_make_event(ctx: &mut LowerCtx<'_>, topic: Value, payload: Value) -> Value {
    let out = emit_alloc(ctx, 8, EVENT_ALIGN);
    emit_store_i32(ctx, out, EVENT_TOPIC_OFFSET, topic);
    emit_store_i32(ctx, out, EVENT_PAYLOAD_OFFSET, payload);
    out
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

fn emit_address_from_bytes(ctx: &mut LowerCtx<'_>, bytes_val: Value) -> Result<Value> {
    let len = emit_bytes_len(ctx, bytes_val);
    let need = emit_int_const(ctx, ADDRESS_LEN as i64);
    let ok = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: ok,
        op: BinOpIR::Eq,
        lhs: len,
        rhs: need,
        ty: IrType::Int,
    });

    let out = emit_alloc(ctx, ADDRESS_LEN, ADDRESS_ALIGN);
    let src = emit_bytes_data_ptr(ctx, bytes_val);
    let copy_len = fresh(ctx);
    let zero = emit_int_const(ctx, 0);
    ctx.body.push(Instr::ISelect {
        dst: copy_len,
        cond: ok,
        then_v: need,
        else_v: zero,
    });
    emit_memcpy_bytes(ctx, src, out, copy_len)?;
    let invalid_address_code = emit_int_const(ctx, CONTRACT_ERR_INVALID_ADDRESS);
    let err = emit_make_contract_error(ctx, invalid_address_code);
    Ok(emit_select_result(ctx, ok, out, err))
}

fn emit_address_to_bytes(ctx: &mut LowerCtx<'_>, address: Value) -> Result<Value> {
    let len = emit_int_const(ctx, ADDRESS_LEN as i64);
    let out = emit_bytes_alloc(ctx, len);
    let dst = emit_bytes_data_ptr(ctx, out);
    emit_memcpy_bytes(ctx, address, dst, len)?;
    Ok(out)
}

fn emit_address_equals(ctx: &mut LowerCtx<'_>, lhs: Value, rhs: Value) -> Value {
    let mut acc = emit_bool_const(ctx, true);
    for offset in 0..ADDRESS_LEN {
        let lhs_b = emit_load_u8(ctx, lhs, offset);
        let rhs_b = emit_load_u8(ctx, rhs, offset);
        let eq = fresh(ctx);
        ctx.body.push(Instr::IBin {
            dst: eq,
            op: BinOpIR::Eq,
            lhs: lhs_b,
            rhs: rhs_b,
            ty: IrType::U8,
        });
        let next = fresh(ctx);
        ctx.body.push(Instr::IBin {
            dst: next,
            op: BinOpIR::And,
            lhs: acc,
            rhs: eq,
            ty: IrType::Bool,
        });
        acc = next;
    }
    acc
}

fn emit_contract_error_code(ctx: &mut LowerCtx<'_>, err: Value) -> Value {
    let code = emit_load_i32(ctx, err, CONTRACT_ERROR_CODE_OFFSET);
    emit_make_error_code(ctx, code)
}

pub(super) fn lower_contract_call<'a>(
    ctx: &mut LowerCtx<'a>,
    callee: &str,
    args: &'a [Expr],
) -> Result<Option<Value>> {
    match callee {
        "std::contract::address::from_bytes" => {
            if args.len() != 1 {
                anyhow::bail!("`std::contract::address::from_bytes` expects one argument");
            }
            let input = lower_expr(ctx, &args[0], Some(Type::Bytes))?;
            Ok(Some(emit_address_from_bytes(ctx, input)?))
        }
        "std::contract::address::to_bytes" => {
            if args.len() != 1 {
                anyhow::bail!("`std::contract::address::to_bytes` expects one argument");
            }
            let address = lower_expr(ctx, &args[0], Some(address_type()))?;
            Ok(Some(emit_address_to_bytes(ctx, address)?))
        }
        "std::contract::address::equals" => {
            if args.len() != 2 {
                anyhow::bail!("`std::contract::address::equals` expects two arguments");
            }
            let lhs = lower_expr(ctx, &args[0], Some(address_type()))?;
            let rhs = lower_expr(ctx, &args[1], Some(address_type()))?;
            Ok(Some(emit_address_equals(ctx, lhs, rhs)))
        }
        "std::contract::amount::from_u64" => {
            if args.len() != 1 {
                anyhow::bail!("`std::contract::amount::from_u64` expects one argument");
            }
            let value = lower_expr(ctx, &args[0], Some(Type::U64))?;
            Ok(Some(emit_make_amount(ctx, value)))
        }
        "std::contract::amount::value" => {
            if args.len() != 1 {
                anyhow::bail!("`std::contract::amount::value` expects one argument");
            }
            let amount = lower_expr(ctx, &args[0], Some(amount_type()))?;
            Ok(Some(emit_load_u64(ctx, amount, AMOUNT_VALUE_OFFSET)))
        }
        "std::contract::amount::add_checked" => {
            if args.len() != 2 {
                anyhow::bail!("`std::contract::amount::add_checked` expects two arguments");
            }
            let lhs = lower_expr(ctx, &args[0], Some(amount_type()))?;
            let rhs = lower_expr(ctx, &args[1], Some(amount_type()))?;
            let lhs_v = emit_load_u64(ctx, lhs, AMOUNT_VALUE_OFFSET);
            let rhs_v = emit_load_u64(ctx, rhs, AMOUNT_VALUE_OFFSET);
            let sum = emit_u64_bin(ctx, BinOp::Add, lhs_v, rhs_v);
            let overflow = emit_u64_overflow_flag(ctx, &BinOp::Add, lhs_v, rhs_v, sum)?
                .expect("u64 add overflow flag");
            let false_v = emit_bool_const(ctx, false);
            let ok = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: ok,
                op: BinOpIR::Eq,
                lhs: overflow,
                rhs: false_v,
                ty: IrType::Bool,
            });
            let amount = emit_make_amount(ctx, sum);
            let overflow_code = emit_int_const(ctx, CONTRACT_ERR_AMOUNT_OVERFLOW);
            let err = emit_make_contract_error(ctx, overflow_code);
            Ok(Some(emit_select_result(ctx, ok, amount, err)))
        }
        "std::contract::amount::sub_checked" => {
            if args.len() != 2 {
                anyhow::bail!("`std::contract::amount::sub_checked` expects two arguments");
            }
            let lhs = lower_expr(ctx, &args[0], Some(amount_type()))?;
            let rhs = lower_expr(ctx, &args[1], Some(amount_type()))?;
            let lhs_v = emit_load_u64(ctx, lhs, AMOUNT_VALUE_OFFSET);
            let rhs_v = emit_load_u64(ctx, rhs, AMOUNT_VALUE_OFFSET);
            let diff = emit_u64_bin(ctx, BinOp::Sub, lhs_v, rhs_v);
            let overflow = emit_u64_overflow_flag(ctx, &BinOp::Sub, lhs_v, rhs_v, diff)?
                .expect("u64 sub overflow flag");
            let false_v = emit_bool_const(ctx, false);
            let ok = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: ok,
                op: BinOpIR::Eq,
                lhs: overflow,
                rhs: false_v,
                ty: IrType::Bool,
            });
            let amount = emit_make_amount(ctx, diff);
            let underflow_code = emit_int_const(ctx, CONTRACT_ERR_AMOUNT_UNDERFLOW);
            let err = emit_make_contract_error(ctx, underflow_code);
            Ok(Some(emit_select_result(ctx, ok, amount, err)))
        }
        "std::contract::amount::is_zero" => {
            if args.len() != 1 {
                anyhow::bail!("`std::contract::amount::is_zero` expects one argument");
            }
            let amount = lower_expr(ctx, &args[0], Some(amount_type()))?;
            let value = emit_load_u64(ctx, amount, AMOUNT_VALUE_OFFSET);
            let zero = emit_u64_const(ctx, 0);
            let out = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: out,
                op: BinOpIR::Eq,
                lhs: value,
                rhs: zero,
                ty: IrType::U64,
            });
            Ok(Some(out))
        }
        "std::contract::event::new" => {
            if args.len() != 2 {
                anyhow::bail!("`std::contract::event::new` expects two arguments");
            }
            let topic = lower_expr(ctx, &args[0], Some(Type::Bytes))?;
            let payload = lower_expr(ctx, &args[1], Some(Type::Bytes))?;
            Ok(Some(emit_make_event(ctx, topic, payload)))
        }
        "std::contract::event::topic" => {
            if args.len() != 1 {
                anyhow::bail!("`std::contract::event::topic` expects one argument");
            }
            let event = lower_expr(ctx, &args[0], Some(event_type()))?;
            Ok(Some(emit_load_i32(ctx, event, EVENT_TOPIC_OFFSET)))
        }
        "std::contract::event::payload" => {
            if args.len() != 1 {
                anyhow::bail!("`std::contract::event::payload` expects one argument");
            }
            let event = lower_expr(ctx, &args[0], Some(event_type()))?;
            Ok(Some(emit_load_i32(ctx, event, EVENT_PAYLOAD_OFFSET)))
        }
        "std::contract::contract_error::code" => {
            if args.len() != 1 {
                anyhow::bail!("`std::contract::contract_error::code` expects one argument");
            }
            let err = lower_expr(ctx, &args[0], Some(contract_error_type()))?;
            Ok(Some(emit_contract_error_code(ctx, err)))
        }
        "std::contract::contract_error::equals" => {
            if args.len() != 2 {
                anyhow::bail!("`std::contract::contract_error::equals` expects two arguments");
            }
            let lhs = lower_expr(ctx, &args[0], Some(contract_error_type()))?;
            let rhs = lower_expr(ctx, &args[1], Some(contract_error_type()))?;
            let lhs_code = emit_load_i32(ctx, lhs, CONTRACT_ERROR_CODE_OFFSET);
            let rhs_code = emit_load_i32(ctx, rhs, CONTRACT_ERROR_CODE_OFFSET);
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
