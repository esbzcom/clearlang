use anyhow::Result;
use clg_ast::{Expr, Type};
use clg_ir::{BinOpIR, Instr, IrType, Value};

use super::{emit_alloc, emit_int_const, fresh, lower_expr, pack_variant_payload, LowerCtx};

const HOST_ERROR_CODE_OFFSET: u32 = 0;

fn host_error_type() -> Type {
    Type::Named {
        name: "std::host::HostError".to_string(),
        args: Vec::new(),
    }
}

fn error_code_type() -> Type {
    Type::Named {
        name: "std::core::ErrorCode".to_string(),
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

fn emit_store_i32(ctx: &mut LowerCtx<'_>, ptr: Value, offset: u32, src: Value) {
    ctx.body.push(Instr::Store {
        ptr,
        src,
        offset,
        ty: IrType::Int,
    });
}

fn emit_make_error_code(ctx: &mut LowerCtx<'_>, code: Value) -> Value {
    let out = emit_alloc(ctx, 4, 4);
    emit_store_i32(ctx, out, 0, code);
    out
}

fn emit_ok_result(ctx: &mut LowerCtx<'_>, payload_ty: &Type, payload: Value) -> Result<Value> {
    let packed = pack_variant_payload(ctx, payload_ty, payload)?;
    let tag = emit_int_const(ctx, 1);
    let zero = emit_int_const(ctx, 0);
    Ok(ctx.variant_init(tag, packed, zero))
}

fn emit_call_raw(ctx: &mut LowerCtx<'_>, callee: &str, args: Vec<Value>) -> Result<Value> {
    let idx = ctx
        .fn_indices
        .get(callee)
        .copied()
        .ok_or_else(|| anyhow::anyhow!("missing host raw intrinsic `{}`", callee))?;
    let out = fresh(ctx);
    ctx.body.push(Instr::Call {
        dst: Some(out),
        callee: idx,
        args,
    });
    Ok(out)
}

pub(super) fn lower_host_call<'a>(
    ctx: &mut LowerCtx<'a>,
    callee: &str,
    args: &'a [Expr],
) -> Result<Option<Value>> {
    match callee {
        "std::host::storage::contains" => {
            if args.len() != 1 {
                anyhow::bail!("`std::host::storage::contains` expects one argument");
            }
            let key = lower_expr(ctx, &args[0], Some(Type::Bytes))?;
            let value = emit_call_raw(ctx, "std::host::__storage_contains_raw", vec![key])?;
            Ok(Some(emit_ok_result(ctx, &Type::Bool, value)?))
        }
        "std::host::storage::get" => {
            if args.len() != 1 {
                anyhow::bail!("`std::host::storage::get` expects one argument");
            }
            let key = lower_expr(ctx, &args[0], Some(Type::Bytes))?;
            let raw = emit_call_raw(ctx, "std::host::__storage_get_raw", vec![key])?;
            let none_sentinel = emit_int_const(ctx, -1);
            let is_some = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: is_some,
                op: BinOpIR::Neq,
                lhs: raw,
                rhs: none_sentinel,
                ty: IrType::Int,
            });
            let zero = emit_int_const(ctx, 0);
            let payload = fresh(ctx);
            ctx.body.push(Instr::ISelect {
                dst: payload,
                cond: is_some,
                then_v: raw,
                else_v: zero,
            });
            let zero_hi = emit_int_const(ctx, 0);
            let option = ctx.variant_init(is_some, payload, zero_hi);
            Ok(Some(emit_ok_result(
                ctx,
                &Type::Option(Box::new(Type::Bytes)),
                option,
            )?))
        }
        "std::host::storage::set" => {
            if args.len() != 2 {
                anyhow::bail!("`std::host::storage::set` expects two arguments");
            }
            let key = lower_expr(ctx, &args[0], Some(Type::Bytes))?;
            let value = lower_expr(ctx, &args[1], Some(Type::Bytes))?;
            let changed = emit_call_raw(ctx, "std::host::__storage_set_raw", vec![key, value])?;
            Ok(Some(emit_ok_result(ctx, &Type::Bool, changed)?))
        }
        "std::host::storage::delete" => {
            if args.len() != 1 {
                anyhow::bail!("`std::host::storage::delete` expects one argument");
            }
            let key = lower_expr(ctx, &args[0], Some(Type::Bytes))?;
            let deleted = emit_call_raw(ctx, "std::host::__storage_delete_raw", vec![key])?;
            Ok(Some(emit_ok_result(ctx, &Type::Bool, deleted)?))
        }
        "std::host::log::info" | "std::host::log::warn" | "std::host::log::error" => {
            if args.len() != 2 {
                anyhow::bail!("`{}` expects two arguments", callee);
            }
            let code = lower_expr(ctx, &args[0], Some(error_code_type()))?;
            let code_value = emit_load_i32(ctx, code, 0);
            let message = lower_expr(ctx, &args[1], Some(Type::String))?;
            let raw = match callee {
                "std::host::log::info" => "std::host::__log_info_raw",
                "std::host::log::warn" => "std::host::__log_warn_raw",
                _ => "std::host::__log_error_raw",
            };
            let ok = emit_call_raw(ctx, raw, vec![code_value, message])?;
            Ok(Some(emit_ok_result(ctx, &Type::Bool, ok)?))
        }
        "std::host::env::chain_id" => {
            if !args.is_empty() {
                anyhow::bail!("`std::host::env::chain_id` expects no arguments");
            }
            let value = emit_call_raw(ctx, "std::host::__env_chain_id_raw", Vec::new())?;
            Ok(Some(emit_ok_result(ctx, &Type::U64, value)?))
        }
        "std::host::env::caller" => {
            if !args.is_empty() {
                anyhow::bail!("`std::host::env::caller` expects no arguments");
            }
            let value = emit_call_raw(ctx, "std::host::__env_caller_raw", Vec::new())?;
            Ok(Some(emit_ok_result(ctx, &Type::Bytes, value)?))
        }
        "std::host::env::block_height" => {
            if !args.is_empty() {
                anyhow::bail!("`std::host::env::block_height` expects no arguments");
            }
            let value = emit_call_raw(ctx, "std::host::__env_block_height_raw", Vec::new())?;
            Ok(Some(emit_ok_result(ctx, &Type::U64, value)?))
        }
        "std::host::env::timestamp" => {
            if !args.is_empty() {
                anyhow::bail!("`std::host::env::timestamp` expects no arguments");
            }
            let value = emit_call_raw(ctx, "std::host::__env_timestamp_raw", Vec::new())?;
            Ok(Some(emit_ok_result(ctx, &Type::U64, value)?))
        }
        "std::host::host_error::code" => {
            if args.len() != 1 {
                anyhow::bail!("`std::host::host_error::code` expects one argument");
            }
            let err = lower_expr(ctx, &args[0], Some(host_error_type()))?;
            let code = emit_load_i32(ctx, err, HOST_ERROR_CODE_OFFSET);
            Ok(Some(emit_make_error_code(ctx, code)))
        }
        "std::host::host_error::equals" => {
            if args.len() != 2 {
                anyhow::bail!("`std::host::host_error::equals` expects two arguments");
            }
            let lhs = lower_expr(ctx, &args[0], Some(host_error_type()))?;
            let rhs = lower_expr(ctx, &args[1], Some(host_error_type()))?;
            let lhs_code = emit_load_i32(ctx, lhs, HOST_ERROR_CODE_OFFSET);
            let rhs_code = emit_load_i32(ctx, rhs, HOST_ERROR_CODE_OFFSET);
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
