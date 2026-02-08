use anyhow::Result;
use clg_ir::{BinOpIR, Instr, IrType, Value};

use super::{emit_bool_const, emit_int_const, emit_ptr_add, fresh, LowerCtx};

pub(super) fn emit_eq_bytes_fixed(
    ctx: &mut LowerCtx<'_>,
    lhs: Value,
    rhs: Value,
    byte_len: u32,
) -> Result<Value> {
    if byte_len == 0 {
        return Ok(emit_bool_const(ctx, true));
    }
    let result = emit_bool_const(ctx, true);
    let idx = fresh(ctx);
    ctx.body.push(Instr::IConst {
        dst: idx,
        ty: IrType::Int,
        n: 0,
    });
    let len_val = emit_int_const(ctx, byte_len as i64);
    let one = emit_int_const(ctx, 1);

    ctx.body.push(Instr::BlockBegin);
    ctx.body.push(Instr::LoopBegin);
    let cond = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: cond,
        op: BinOpIR::Lt,
        lhs: idx,
        rhs: len_val,
        ty: IrType::Int,
    });
    ctx.body.push(Instr::BrIfEqz { cond, depth: 1 });
    ctx.body.push(Instr::BrIfEqz {
        cond: result,
        depth: 1,
    });

    let lhs_ptr = emit_ptr_add(ctx, lhs, idx);
    let rhs_ptr = emit_ptr_add(ctx, rhs, idx);
    let lhs_byte = fresh(ctx);
    ctx.body.push(Instr::Load {
        dst: lhs_byte,
        ptr: lhs_ptr,
        offset: 0,
        ty: IrType::U8,
    });
    let rhs_byte = fresh(ctx);
    ctx.body.push(Instr::Load {
        dst: rhs_byte,
        ptr: rhs_ptr,
        offset: 0,
        ty: IrType::U8,
    });
    let eq = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: eq,
        op: BinOpIR::Eq,
        lhs: lhs_byte,
        rhs: rhs_byte,
        ty: IrType::U8,
    });
    ctx.body.push(Instr::IBin {
        dst: result,
        op: BinOpIR::And,
        lhs: result,
        rhs: eq,
        ty: IrType::Bool,
    });

    ctx.body.push(Instr::IBin {
        dst: idx,
        op: BinOpIR::Add,
        lhs: idx,
        rhs: one,
        ty: IrType::Int,
    });
    ctx.body.push(Instr::Br { depth: 0 });
    ctx.body.push(Instr::LoopEnd);
    ctx.body.push(Instr::BlockEnd);
    Ok(result)
}

pub(super) fn emit_intrinsic_eq(
    ctx: &mut LowerCtx<'_>,
    callee: &str,
    lhs: Value,
    rhs: Value,
) -> Result<Value> {
    let idx = ctx
        .fn_indices
        .get(callee)
        .copied()
        .ok_or_else(|| anyhow::anyhow!("missing intrinsic `{}`", callee))?;
    let dst = fresh(ctx);
    ctx.body.push(Instr::Call {
        dst: Some(dst),
        callee: idx,
        args: vec![lhs, rhs],
    });
    Ok(dst)
}

pub(super) fn emit_eq_u128(ctx: &mut LowerCtx<'_>, lhs: Value, rhs: Value) -> Value {
    let lhs_lo = fresh(ctx);
    ctx.body.push(Instr::U128LoadLimb {
        dst: lhs_lo,
        value: lhs,
        limb: 0,
    });
    let lhs_hi = fresh(ctx);
    ctx.body.push(Instr::U128LoadLimb {
        dst: lhs_hi,
        value: lhs,
        limb: 1,
    });
    let rhs_lo = fresh(ctx);
    ctx.body.push(Instr::U128LoadLimb {
        dst: rhs_lo,
        value: rhs,
        limb: 0,
    });
    let rhs_hi = fresh(ctx);
    ctx.body.push(Instr::U128LoadLimb {
        dst: rhs_hi,
        value: rhs,
        limb: 1,
    });
    let eq_lo = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: eq_lo,
        op: BinOpIR::Eq,
        lhs: lhs_lo,
        rhs: rhs_lo,
        ty: IrType::U64,
    });
    let eq_hi = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: eq_hi,
        op: BinOpIR::Eq,
        lhs: lhs_hi,
        rhs: rhs_hi,
        ty: IrType::U64,
    });
    let dst = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst,
        op: BinOpIR::And,
        lhs: eq_lo,
        rhs: eq_hi,
        ty: IrType::Int,
    });
    dst
}

pub(super) fn emit_eq_u256(ctx: &mut LowerCtx<'_>, lhs: Value, rhs: Value) -> Value {
    let mut lhs_limbs = Vec::with_capacity(4);
    let mut rhs_limbs = Vec::with_capacity(4);
    for limb in 0..4u8 {
        let lhs_limb = fresh(ctx);
        ctx.body.push(Instr::U256LoadLimb {
            dst: lhs_limb,
            value: lhs,
            limb,
        });
        lhs_limbs.push(lhs_limb);
        let rhs_limb = fresh(ctx);
        ctx.body.push(Instr::U256LoadLimb {
            dst: rhs_limb,
            value: rhs,
            limb,
        });
        rhs_limbs.push(rhs_limb);
    }
    let mut eq_vals = Vec::with_capacity(4);
    for (lhs_limb, rhs_limb) in lhs_limbs.into_iter().zip(rhs_limbs.into_iter()) {
        let eq = fresh(ctx);
        ctx.body.push(Instr::IBin {
            dst: eq,
            op: BinOpIR::Eq,
            lhs: lhs_limb,
            rhs: rhs_limb,
            ty: IrType::U64,
        });
        eq_vals.push(eq);
    }
    let mut acc = eq_vals
        .pop()
        .expect("u256 eq must compare at least one limb");
    while let Some(eq) = eq_vals.pop() {
        let next = fresh(ctx);
        ctx.body.push(Instr::IBin {
            dst: next,
            op: BinOpIR::And,
            lhs: acc,
            rhs: eq,
            ty: IrType::Int,
        });
        acc = next;
    }
    acc
}
