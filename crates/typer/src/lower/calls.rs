use crate::check::{base_type, infer_expr_type};
use crate::guards::guard_kind_for_callee;
use anyhow::Result;
use clg_ast::{BinOp, Expr, Type};
use clg_ir::{BinOpIR, Instr, IrType, Value};

use super::array::{
    emit_array_data_ptr, emit_array_len_guard, emit_bytes_data_ptr, emit_bytes_len_guard,
};
use super::collections::lower_collection_call;
use super::intrinsics::{
    lower_u128_from_limbs, lower_u128_load, lower_u256_from_limbs, lower_u256_load, lower_u64_sat,
    lower_u64_wrap,
};
use super::layout::std_type_info_for_module;
use super::r#match::lower_enum_constructor;
use super::{
    emit_alloc, emit_bool_const, emit_int_const, emit_load_i32, emit_memcpy_bytes, emit_u64_const,
    fresh, lower_expr, DispatcherCallPatch, DispatcherSignature, LowerCtx, CLOSURE_CODE_ID_OFFSET,
    CLOSURE_ENV_PTR_OFFSET,
};

pub(super) fn lower_call_expr<'a>(
    ctx: &mut LowerCtx<'a>,
    call_expr: &'a Expr,
    callee: &str,
    args: &'a [Expr],
    expected: Option<&Type>,
) -> Result<Value> {
    if let Some(val) = lower_collection_call(ctx, call_expr, callee, args, expected)? {
        return Ok(val);
    }
    if let Some(module) = callee.strip_suffix("::from_bytes") {
        if module.starts_with("std::") {
            return lower_std_from_bytes(ctx, module, callee, args);
        }
    }
    if let Some(module) = callee.strip_suffix("::from_array") {
        if module.starts_with("std::") {
            return lower_std_from_array(ctx, module, callee, args);
        }
    }
    match callee {
        "std::bytes::equals" => {
            lower_intrinsic_alias_call(ctx, "std::bytes::eq", args, &[Type::Bytes, Type::Bytes])
        }
        "std::bytes::equals_ct" => {
            lower_intrinsic_alias_call(ctx, "std::bytes::eq_ct", args, &[Type::Bytes, Type::Bytes])
        }
        "std::bytes::is_empty" => {
            lower_len_is_empty_call(ctx, "std::bytes::len", args, Type::Bytes)
        }
        "std::str::equals" => {
            lower_intrinsic_alias_call(ctx, "std::str::eq", args, &[Type::String, Type::String])
        }
        "std::str::is_empty" => lower_len_is_empty_call(ctx, "std::str::len", args, Type::String),
        "std::str::to_bytes" => {
            lower_intrinsic_alias_call(ctx, "std::bytes::from_string", args, &[Type::String])
        }
        "std::crypto::sha256" => lower_crypto_sha256_call(ctx, args),
        "std::crypto::hmac_sha256" => lower_crypto_hmac_sha256_call(ctx, args),
        "U8" => {
            if args.len() != 1 {
                anyhow::bail!("`U8` expects exactly one argument");
            }
            lower_expr(ctx, &args[0], Some(Type::U8))
        }
        "U64" => {
            if args.len() != 1 {
                anyhow::bail!("`U64` expects exactly one argument");
            }
            lower_expr(ctx, &args[0], Some(Type::U64))
        }
        "U128" => {
            if args.len() != 1 {
                anyhow::bail!("`U128` expects exactly one argument");
            }
            let arg_ty = infer_expr_type(
                &args[0],
                &ctx.type_env,
                &ctx.fns,
                ctx.trait_env,
                ctx.aliases,
                ctx.type_defs,
                &ctx.type_params,
                &ctx.bounds,
            )?;
            if matches!(arg_ty, Type::U128) {
                return lower_expr(ctx, &args[0], Some(Type::U128));
            }
            let limb_lo = lower_expr(ctx, &args[0], Some(Type::U64))?;
            let limb_hi = emit_u64_const(ctx, 0);
            let dst = fresh(ctx);
            ctx.body.push(Instr::U128Init {
                dst,
                limb_lo,
                limb_hi,
            });
            Ok(dst)
        }
        "U256" => {
            if args.len() != 1 {
                anyhow::bail!("`U256` expects exactly one argument");
            }
            let arg_ty = infer_expr_type(
                &args[0],
                &ctx.type_env,
                &ctx.fns,
                ctx.trait_env,
                ctx.aliases,
                ctx.type_defs,
                &ctx.type_params,
                &ctx.bounds,
            )?;
            if matches!(arg_ty, Type::U256) {
                return lower_expr(ctx, &args[0], Some(Type::U256));
            }
            let limb0 = lower_expr(ctx, &args[0], Some(Type::U64))?;
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
        "std::u64::add_wrap" => lower_u64_wrap(ctx, BinOp::Add, args),
        "std::u64::add_wrapping" => lower_u64_wrap(ctx, BinOp::Add, args),
        "std::u64::sub_wrap" => lower_u64_wrap(ctx, BinOp::Sub, args),
        "std::u64::sub_wrapping" => lower_u64_wrap(ctx, BinOp::Sub, args),
        "std::u64::mul_wrap" => lower_u64_wrap(ctx, BinOp::Mul, args),
        "std::u64::mul_wrapping" => lower_u64_wrap(ctx, BinOp::Mul, args),
        "std::u64::add_sat" => lower_u64_sat(ctx, BinOp::Add, args),
        "std::u64::add_saturating" => lower_u64_sat(ctx, BinOp::Add, args),
        "std::u64::sub_sat" => lower_u64_sat(ctx, BinOp::Sub, args),
        "std::u64::sub_saturating" => lower_u64_sat(ctx, BinOp::Sub, args),
        "std::u64::mul_sat" => lower_u64_sat(ctx, BinOp::Mul, args),
        "std::u64::mul_saturating" => lower_u64_sat(ctx, BinOp::Mul, args),
        "std::u128::from_limbs" => lower_u128_from_limbs(ctx, args),
        "std::u128::lo" => lower_u128_load(ctx, args, 0),
        "std::u128::hi" => lower_u128_load(ctx, args, 1),
        "std::u256::from_limbs" => lower_u256_from_limbs(ctx, args),
        "std::u256::limb0" => lower_u256_load(ctx, args, 0),
        "std::u256::limb1" => lower_u256_load(ctx, args, 1),
        "std::u256::limb2" => lower_u256_load(ctx, args, 2),
        "std::u256::limb3" => lower_u256_load(ctx, args, 3),
        "std::unit::assert_true" => {
            if args.len() != 2 {
                anyhow::bail!("`std::unit::assert_true` expects exactly two arguments");
            }
            let cond = lower_expr(ctx, &args[0], Some(Type::Bool))?;
            let _msg = lower_expr(ctx, &args[1], Some(Type::String))?;
            Ok(cond)
        }
        "std::unit::assert_eq_int" => {
            if args.len() != 3 {
                anyhow::bail!("`std::unit::assert_eq_int` expects exactly three arguments");
            }
            let actual = lower_expr(ctx, &args[0], Some(Type::Int))?;
            let expected = lower_expr(ctx, &args[1], Some(Type::Int))?;
            let _msg = lower_expr(ctx, &args[2], Some(Type::String))?;
            let dst = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst,
                op: BinOpIR::Eq,
                ty: IrType::Int,
                lhs: actual,
                rhs: expected,
            });
            Ok(dst)
        }
        "std::unit::assert_eq_bool" => {
            if args.len() != 3 {
                anyhow::bail!("`std::unit::assert_eq_bool` expects exactly three arguments");
            }
            let actual = lower_expr(ctx, &args[0], Some(Type::Bool))?;
            let expected = lower_expr(ctx, &args[1], Some(Type::Bool))?;
            let _msg = lower_expr(ctx, &args[2], Some(Type::String))?;
            let dst = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst,
                op: BinOpIR::Eq,
                ty: IrType::Bool,
                lhs: actual,
                rhs: expected,
            });
            Ok(dst)
        }
        "std::unit::fail" => {
            if args.len() != 1 {
                anyhow::bail!("`std::unit::fail` expects exactly one argument");
            }
            let _msg = lower_expr(ctx, &args[0], Some(Type::String))?;
            Ok(emit_bool_const(ctx, false))
        }
        "Some" => {
            if args.len() != 1 {
                anyhow::bail!("`Some` expects exactly one argument");
            }
            let payload = lower_expr(ctx, &args[0], None)?;
            let tag = emit_int_const(ctx, 1);
            let zero = emit_int_const(ctx, 0);
            Ok(ctx.variant_init(tag, payload, zero))
        }
        "None" => {
            if !args.is_empty() {
                anyhow::bail!("`None` does not take arguments");
            }
            let tag = emit_int_const(ctx, 0);
            let zero = emit_int_const(ctx, 0);
            Ok(ctx.variant_init(tag, zero, zero))
        }
        "Ok" => {
            if args.len() > 1 {
                anyhow::bail!("`Ok` expects at most one argument");
            }
            let payload = if let Some(arg) = args.first() {
                lower_expr(ctx, arg, None)?
            } else {
                emit_int_const(ctx, 0)
            };
            let tag = emit_int_const(ctx, 1);
            let zero = emit_int_const(ctx, 0);
            Ok(ctx.variant_init(tag, payload, zero))
        }
        "Err" => {
            if args.len() > 1 {
                anyhow::bail!("`Err` expects at most one argument");
            }
            let payload = if let Some(arg) = args.first() {
                lower_expr(ctx, arg, None)?
            } else {
                emit_int_const(ctx, 0)
            };
            let tag = emit_int_const(ctx, 0);
            let zero = emit_int_const(ctx, 0);
            Ok(ctx.variant_init(tag, payload, zero))
        }
        _ => {
            if let Some(binding) = ctx.type_env.get(callee) {
                let binding_ty = binding.ty.clone();
                if let Type::Fn {
                    params: fn_params,
                    ret: fn_ret,
                } = base_type(&binding_ty, ctx.aliases)?
                {
                    if fn_params.len() != args.len() {
                        anyhow::bail!(
                            "arity mismatch in dynamic closure call `{}`: expected {}, found {}",
                            callee,
                            fn_params.len(),
                            args.len()
                        );
                    }
                    let closure_ptr =
                        ctx.env.get(callee).copied().ok_or_else(|| {
                            anyhow::anyhow!("missing closure value for `{}`", callee)
                        })?;
                    let code_id = emit_load_i32(ctx, closure_ptr, CLOSURE_CODE_ID_OFFSET);
                    let env_ptr = emit_load_i32(ctx, closure_ptr, CLOSURE_ENV_PTR_OFFSET);
                    let mut call_args: Vec<Value> = Vec::with_capacity(2 + args.len());
                    call_args.push(code_id);
                    call_args.push(env_ptr);
                    for (idx, arg) in args.iter().enumerate() {
                        let expected_ty = fn_params.get(idx).cloned();
                        call_args.push(lower_expr(ctx, arg, expected_ty)?);
                    }
                    let dst = fresh(ctx);
                    let instr_index = ctx.body.len();
                    ctx.body.push(Instr::Call {
                        dst: Some(dst),
                        callee: 0,
                        args: call_args,
                    });
                    ctx.dispatcher_patches.push(DispatcherCallPatch {
                        function_name: ctx.function_name.clone(),
                        instr_index,
                        signature: DispatcherSignature {
                            params: fn_params,
                            ret: (*fn_ret).clone(),
                        },
                    });
                    return Ok(dst);
                }
            }
            if let Some((enum_name, variant_name)) = callee.rsplit_once("::") {
                if ctx.type_defs.enums.contains_key(enum_name) {
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
                    let resolved = base_type(&call_ty, ctx.aliases)?;
                    let enum_args = match resolved {
                        Type::Named { args, .. } => args,
                        _ => Vec::new(),
                    };
                    if let Some(val) =
                        lower_enum_constructor(ctx, enum_name, variant_name, args, &enum_args)?
                    {
                        return Ok(val);
                    }
                }
            }
            if guard_kind_for_callee(callee).is_some() {
                let dst = fresh(ctx);
                ctx.body.push(Instr::IConst {
                    dst,
                    ty: IrType::Bool,
                    n: 1,
                });
                return Ok(dst);
            }
            let sig = ctx
                .fns
                .get(callee)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!(format!("unknown function `{}`", callee)))?;
            let argv: Result<Vec<_>> = args
                .iter()
                .enumerate()
                .map(|(i, a)| {
                    let expected_ty = sig.params.get(i).map(|p| p.ty.clone());
                    lower_expr(ctx, a, expected_ty)
                })
                .collect();
            let argv = argv?;
            if let Some(idx) = ctx.fn_indices.get(callee).copied() {
                let dst = fresh(ctx);
                ctx.body.push(Instr::Call {
                    dst: Some(dst),
                    callee: idx,
                    args: argv,
                });
                Ok(dst)
            } else {
                anyhow::bail!(format!("unknown function `{}`", callee))
            }
        }
    }
}

fn lower_intrinsic_alias_call<'a>(
    ctx: &mut LowerCtx<'a>,
    target: &str,
    args: &'a [Expr],
    expected_arg_types: &[Type],
) -> Result<Value> {
    if args.len() != expected_arg_types.len() {
        anyhow::bail!(
            "`{}` expects {} arguments",
            target,
            expected_arg_types.len()
        );
    }
    let mut argv = Vec::with_capacity(args.len());
    for (arg, expected_ty) in args.iter().zip(expected_arg_types.iter()) {
        argv.push(lower_expr(ctx, arg, Some(expected_ty.clone()))?);
    }
    let callee_idx = ctx
        .fn_indices
        .get(target)
        .copied()
        .ok_or_else(|| anyhow::anyhow!("missing intrinsic `{}`", target))?;
    let dst = fresh(ctx);
    ctx.body.push(Instr::Call {
        dst: Some(dst),
        callee: callee_idx,
        args: argv,
    });
    Ok(dst)
}

fn lower_len_is_empty_call<'a>(
    ctx: &mut LowerCtx<'a>,
    len_callee: &str,
    args: &'a [Expr],
    expected_arg_type: Type,
) -> Result<Value> {
    if args.len() != 1 {
        anyhow::bail!("`{}` expects exactly one argument", len_callee);
    }
    let value = lower_expr(ctx, &args[0], Some(expected_arg_type))?;
    let len_idx = ctx
        .fn_indices
        .get(len_callee)
        .copied()
        .ok_or_else(|| anyhow::anyhow!("missing intrinsic `{}`", len_callee))?;
    let len_val = fresh(ctx);
    ctx.body.push(Instr::Call {
        dst: Some(len_val),
        callee: len_idx,
        args: vec![value],
    });
    let zero = emit_int_const(ctx, 0);
    let out = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: out,
        op: BinOpIR::Eq,
        ty: IrType::Int,
        lhs: len_val,
        rhs: zero,
    });
    Ok(out)
}

fn lower_crypto_sha256_call<'a>(ctx: &mut LowerCtx<'a>, args: &'a [Expr]) -> Result<Value> {
    if args.len() != 1 {
        anyhow::bail!("`std::crypto::sha256` expects exactly one argument");
    }
    let data = lower_expr(ctx, &args[0], Some(Type::Bytes))?;
    let alg = fresh(ctx);
    ctx.body.push(Instr::IStringConst {
        dst: alg,
        s: "sha256".to_string(),
    });
    let hash_idx = ctx
        .fn_indices
        .get("std::crypto::hash")
        .copied()
        .ok_or_else(|| anyhow::anyhow!("missing intrinsic `std::crypto::hash`"))?;
    let out = fresh(ctx);
    ctx.body.push(Instr::Call {
        dst: Some(out),
        callee: hash_idx,
        args: vec![alg, data],
    });
    Ok(out)
}

fn lower_crypto_hmac_sha256_call<'a>(ctx: &mut LowerCtx<'a>, args: &'a [Expr]) -> Result<Value> {
    if args.len() != 2 {
        anyhow::bail!("`std::crypto::hmac_sha256` expects exactly two arguments");
    }
    let key = lower_expr(ctx, &args[0], Some(Type::Bytes))?;
    let data = lower_expr(ctx, &args[1], Some(Type::Bytes))?;
    let alg = fresh(ctx);
    ctx.body.push(Instr::IStringConst {
        dst: alg,
        s: "sha256".to_string(),
    });
    let hmac_idx = ctx
        .fn_indices
        .get("std::crypto::hmac")
        .copied()
        .ok_or_else(|| anyhow::anyhow!("missing intrinsic `std::crypto::hmac`"))?;
    let out = fresh(ctx);
    ctx.body.push(Instr::Call {
        dst: Some(out),
        callee: hmac_idx,
        args: vec![alg, key, data],
    });
    Ok(out)
}

fn lower_std_from_bytes<'a>(
    ctx: &mut LowerCtx<'a>,
    module: &str,
    callee: &str,
    args: &'a [Expr],
) -> Result<Value> {
    if args.len() != 1 {
        anyhow::bail!("`{}` expects one argument", callee);
    }
    let info = std_type_info_for_module(module, ctx.std_types)?;
    let bytes_ptr = lower_expr(ctx, &args[0], Some(Type::Bytes))?;
    emit_bytes_len_guard(ctx, bytes_ptr, info.byte_len);
    let data_ptr = emit_bytes_data_ptr(ctx, bytes_ptr);
    let dst = emit_alloc(ctx, info.byte_len, info.align);
    let len_val = emit_int_const(ctx, info.byte_len as i64);
    emit_memcpy_bytes(ctx, data_ptr, dst, len_val)?;
    Ok(dst)
}

fn lower_std_from_array<'a>(
    ctx: &mut LowerCtx<'a>,
    module: &str,
    callee: &str,
    args: &'a [Expr],
) -> Result<Value> {
    if args.len() != 1 {
        anyhow::bail!("`{}` expects one argument", callee);
    }
    let info = std_type_info_for_module(module, ctx.std_types)?;
    let array_ty = Type::Array(Box::new(Type::U8), None);
    let array_ptr = lower_expr(ctx, &args[0], Some(array_ty))?;
    emit_array_len_guard(ctx, array_ptr, info.byte_len);
    let data_ptr = emit_array_data_ptr(ctx, array_ptr);
    let dst = emit_alloc(ctx, info.byte_len, info.align);
    let len_val = emit_int_const(ctx, info.byte_len as i64);
    emit_memcpy_bytes(ctx, data_ptr, dst, len_val)?;
    Ok(dst)
}
