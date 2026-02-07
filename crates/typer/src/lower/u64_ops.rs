use anyhow::Result;
use clg_ast::{BinOp, Span};
use clg_ir::{BinOpIR, GuardKind, Instr, IrType, TrapCode, Value};

use super::{emit_bool_const, fresh, LowerCtx};

pub(crate) fn emit_u64_const(ctx: &mut LowerCtx<'_>, n: u64) -> Value {
    let dst = fresh(ctx);
    ctx.body.push(Instr::IConst {
        dst,
        ty: IrType::U64,
        n: n as i64,
    });
    dst
}

pub(crate) fn emit_u64_bin(ctx: &mut LowerCtx<'_>, op: BinOp, lhs: Value, rhs: Value) -> Value {
    let dst = fresh(ctx);
    let ir_op = match op {
        BinOp::Add => BinOpIR::Add,
        BinOp::Sub => BinOpIR::Sub,
        BinOp::Mul => BinOpIR::Mul,
        BinOp::Div => BinOpIR::Div,
        _ => BinOpIR::Add,
    };
    ctx.body.push(Instr::IBin {
        dst,
        op: ir_op,
        lhs,
        rhs,
        ty: IrType::U64,
    });
    dst
}

pub(crate) fn emit_u64_overflow_flag(
    ctx: &mut LowerCtx<'_>,
    op: &BinOp,
    lhs: Value,
    rhs: Value,
    dst: Value,
) -> Result<Option<Value>> {
    let overflow = match op {
        BinOp::Add => {
            let overflow = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: overflow,
                op: BinOpIR::Lt,
                lhs: dst,
                rhs: lhs,
                ty: IrType::U64,
            });
            overflow
        }
        BinOp::Sub => {
            let overflow = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: overflow,
                op: BinOpIR::Lt,
                lhs,
                rhs,
                ty: IrType::U64,
            });
            overflow
        }
        BinOp::Mul => {
            let zero = emit_u64_const(ctx, 0);
            let rhs_is_zero = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: rhs_is_zero,
                op: BinOpIR::Eq,
                lhs: rhs,
                rhs: zero,
                ty: IrType::U64,
            });
            let one = emit_u64_const(ctx, 1);
            let rhs_nonzero = fresh(ctx);
            ctx.body.push(Instr::ISelect {
                dst: rhs_nonzero,
                cond: rhs_is_zero,
                then_v: one,
                else_v: rhs,
            });
            let div = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: div,
                op: BinOpIR::Div,
                lhs: dst,
                rhs: rhs_nonzero,
                ty: IrType::U64,
            });
            let div_eq = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: div_eq,
                op: BinOpIR::Eq,
                lhs: div,
                rhs: lhs,
                ty: IrType::U64,
            });
            let ok = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: ok,
                op: BinOpIR::Or,
                lhs: rhs_is_zero,
                rhs: div_eq,
                ty: IrType::Bool,
            });
            let zero = emit_bool_const(ctx, false);
            let overflow = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: overflow,
                op: BinOpIR::Eq,
                lhs: ok,
                rhs: zero,
                ty: IrType::Bool,
            });
            overflow
        }
        _ => return Ok(None),
    };
    Ok(Some(overflow))
}

pub(crate) fn emit_u64_overflow_guard(
    ctx: &mut LowerCtx<'_>,
    op: &BinOp,
    lhs: Value,
    rhs: Value,
    dst: Value,
    span: Span,
) -> Result<()> {
    let Some(overflow) = emit_u64_overflow_flag(ctx, op, lhs, rhs, dst)? else {
        return Ok(());
    };
    let zero = emit_bool_const(ctx, false);
    let ok = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: ok,
        op: BinOpIR::Eq,
        lhs: overflow,
        rhs: zero,
        ty: IrType::Bool,
    });

    ctx.body.push(Instr::Guard {
        cond: ok,
        trap: TrapCode::Overflow,
        span: Some((span.start as u32, span.end as u32)),
        detail: GuardKind::Require,
    });
    Ok(())
}
