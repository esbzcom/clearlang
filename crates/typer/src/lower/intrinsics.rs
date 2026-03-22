use anyhow::Result;
use clg_ast::{BinOp, Expr, Type};
use clg_ir::{Instr, Value};

use super::{emit_u64_bin, emit_u64_const, emit_u64_overflow_flag, fresh, lower_expr, LowerCtx};

pub(super) fn lower_u64_wrap<'a>(
    ctx: &mut LowerCtx<'a>,
    op: BinOp,
    args: &'a [Expr],
) -> Result<Value> {
    if args.len() != 2 {
        anyhow::bail!("std::u64::*_wrap expects exactly two arguments");
    }
    let lhs = lower_expr(ctx, &args[0], Some(Type::U64))?;
    let rhs = lower_expr(ctx, &args[1], Some(Type::U64))?;
    Ok(emit_u64_bin(ctx, op, lhs, rhs))
}

pub(super) fn lower_u64_sat<'a>(
    ctx: &mut LowerCtx<'a>,
    op: BinOp,
    args: &'a [Expr],
) -> Result<Value> {
    if args.len() != 2 {
        anyhow::bail!("std::u64::*_sat expects exactly two arguments");
    }
    let lhs = lower_expr(ctx, &args[0], Some(Type::U64))?;
    let rhs = lower_expr(ctx, &args[1], Some(Type::U64))?;
    let raw = emit_u64_bin(ctx, op, lhs, rhs);
    let Some(overflow) = emit_u64_overflow_flag(ctx, &op, lhs, rhs, raw)? else {
        return Ok(raw);
    };
    let clamp = match op {
        BinOp::Sub => emit_u64_const(ctx, 0),
        _ => emit_u64_const(ctx, u64::MAX),
    };
    let dst = fresh(ctx);
    ctx.body.push(Instr::ISelect {
        dst,
        cond: overflow,
        then_v: clamp,
        else_v: raw,
    });
    Ok(dst)
}

pub(super) fn lower_u128_from_limbs<'a>(ctx: &mut LowerCtx<'a>, args: &'a [Expr]) -> Result<Value> {
    if args.len() != 2 {
        anyhow::bail!("std::u128::from_limbs expects exactly two arguments");
    }
    let limb_lo = lower_expr(ctx, &args[0], Some(Type::U64))?;
    let limb_hi = lower_expr(ctx, &args[1], Some(Type::U64))?;
    let dst = fresh(ctx);
    ctx.body.push(Instr::U128Init {
        dst,
        limb_lo,
        limb_hi,
    });
    Ok(dst)
}

pub(super) fn lower_u128_load<'a>(
    ctx: &mut LowerCtx<'a>,
    args: &'a [Expr],
    limb: u8,
) -> Result<Value> {
    if args.len() != 1 {
        anyhow::bail!("std::u128::lo/hi expects exactly one argument");
    }
    let value = lower_expr(ctx, &args[0], Some(Type::U128))?;
    let dst = fresh(ctx);
    ctx.body.push(Instr::U128LoadLimb { dst, value, limb });
    Ok(dst)
}

pub(super) fn lower_u256_from_limbs<'a>(ctx: &mut LowerCtx<'a>, args: &'a [Expr]) -> Result<Value> {
    if args.len() != 4 {
        anyhow::bail!("std::u256::from_limbs expects exactly four arguments");
    }
    let limb0 = lower_expr(ctx, &args[0], Some(Type::U64))?;
    let limb1 = lower_expr(ctx, &args[1], Some(Type::U64))?;
    let limb2 = lower_expr(ctx, &args[2], Some(Type::U64))?;
    let limb3 = lower_expr(ctx, &args[3], Some(Type::U64))?;
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

pub(super) fn lower_u256_load<'a>(
    ctx: &mut LowerCtx<'a>,
    args: &'a [Expr],
    limb: u8,
) -> Result<Value> {
    if args.len() != 1 {
        anyhow::bail!("std::u256::limb* expects exactly one argument");
    }
    let value = lower_expr(ctx, &args[0], Some(Type::U256))?;
    let dst = fresh(ctx);
    ctx.body.push(Instr::U256LoadLimb { dst, value, limb });
    Ok(dst)
}
