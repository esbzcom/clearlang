use anyhow::Result;
use clg_ast::{Expr, Type};
use clg_ir::{BinOpIR, Instr, IrType, Value};

use super::{emit_alloc, emit_int_const, fresh, lower_expr, LowerCtx};

const VERIFY_RESULT_ALIGN: u32 = 4;
const VERIFY_RESULT_VALID_OFFSET: u32 = 0;
const VERIFY_RESULT_ERROR_OFFSET: u32 = 4;
const CRYPTO_ERROR_CODE_OFFSET: u32 = 0;
const ERROR_CODE_ALIGN: u32 = 4;
const ERROR_CODE_VALUE_OFFSET: u32 = 0;

fn verify_result_type() -> Type {
    Type::Named {
        name: "std::crypto::VerifyResult".to_string(),
        args: Vec::new(),
    }
}

fn crypto_error_type() -> Type {
    Type::Named {
        name: "std::crypto::CryptoError".to_string(),
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

fn emit_make_verify_result(ctx: &mut LowerCtx<'_>, valid: Value, err_ptr: Value) -> Value {
    let out = emit_alloc(ctx, 8, VERIFY_RESULT_ALIGN);
    emit_store_i32(ctx, out, VERIFY_RESULT_VALID_OFFSET, valid);
    emit_store_i32(ctx, out, VERIFY_RESULT_ERROR_OFFSET, err_ptr);
    out
}

fn emit_make_error_code(ctx: &mut LowerCtx<'_>, code: Value) -> Value {
    let out = emit_alloc(ctx, 4, ERROR_CODE_ALIGN);
    emit_store_i32(ctx, out, ERROR_CODE_VALUE_OFFSET, code);
    out
}

pub(super) fn lower_crypto_helper_call<'a>(
    ctx: &mut LowerCtx<'a>,
    callee: &str,
    args: &'a [Expr],
) -> Result<Option<Value>> {
    match callee {
        "std::crypto::verify_result::valid" => {
            if !args.is_empty() {
                anyhow::bail!("`std::crypto::verify_result::valid` expects no arguments");
            }
            let true_v = emit_int_const(ctx, 1);
            let no_err = emit_int_const(ctx, 0);
            Ok(Some(emit_make_verify_result(ctx, true_v, no_err)))
        }
        "std::crypto::verify_result::invalid" => {
            if args.len() != 1 {
                anyhow::bail!("`std::crypto::verify_result::invalid` expects one argument");
            }
            let err = lower_expr(ctx, &args[0], Some(crypto_error_type()))?;
            let false_v = emit_int_const(ctx, 0);
            Ok(Some(emit_make_verify_result(ctx, false_v, err)))
        }
        "std::crypto::verify_result::is_valid" => {
            if args.len() != 1 {
                anyhow::bail!("`std::crypto::verify_result::is_valid` expects one argument");
            }
            let value = lower_expr(ctx, &args[0], Some(verify_result_type()))?;
            let flag = emit_load_i32(ctx, value, VERIFY_RESULT_VALID_OFFSET);
            let true_v = emit_int_const(ctx, 1);
            let out = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: out,
                op: BinOpIR::Eq,
                lhs: flag,
                rhs: true_v,
                ty: IrType::Int,
            });
            Ok(Some(out))
        }
        "std::crypto::verify_result::error_or_none" => {
            if args.len() != 1 {
                anyhow::bail!("`std::crypto::verify_result::error_or_none` expects one argument");
            }
            let value = lower_expr(ctx, &args[0], Some(verify_result_type()))?;
            let flag = emit_load_i32(ctx, value, VERIFY_RESULT_VALID_OFFSET);
            let err_ptr = emit_load_i32(ctx, value, VERIFY_RESULT_ERROR_OFFSET);
            let false_v = emit_int_const(ctx, 0);
            let is_invalid = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: is_invalid,
                op: BinOpIR::Eq,
                lhs: flag,
                rhs: false_v,
                ty: IrType::Int,
            });
            let zero = emit_int_const(ctx, 0);
            let payload = fresh(ctx);
            ctx.body.push(Instr::ISelect {
                dst: payload,
                cond: is_invalid,
                then_v: err_ptr,
                else_v: zero,
            });
            let zero_hi = emit_int_const(ctx, 0);
            Ok(Some(ctx.variant_init(is_invalid, payload, zero_hi)))
        }
        "std::crypto::crypto_error::code" => {
            if args.len() != 1 {
                anyhow::bail!("`std::crypto::crypto_error::code` expects one argument");
            }
            let err = lower_expr(ctx, &args[0], Some(crypto_error_type()))?;
            let code = emit_load_i32(ctx, err, CRYPTO_ERROR_CODE_OFFSET);
            Ok(Some(emit_make_error_code(ctx, code)))
        }
        "std::crypto::crypto_error::equals" => {
            if args.len() != 2 {
                anyhow::bail!("`std::crypto::crypto_error::equals` expects two arguments");
            }
            let lhs = lower_expr(ctx, &args[0], Some(crypto_error_type()))?;
            let rhs = lower_expr(ctx, &args[1], Some(crypto_error_type()))?;
            let lhs_code = emit_load_i32(ctx, lhs, CRYPTO_ERROR_CODE_OFFSET);
            let rhs_code = emit_load_i32(ctx, rhs, CRYPTO_ERROR_CODE_OFFSET);
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
