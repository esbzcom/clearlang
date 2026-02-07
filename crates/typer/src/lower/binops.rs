use crate::check::infer_expr_type;
use anyhow::Result;
use clg_ast::{BinOp, Expr, Type};
use clg_ir::{BinOpIR, Instr, IrType, Value};

use super::block::expr_span_local;
use super::eq::emit_eq_for_type;
use super::layout::std_type_info_for;
use super::{emit_bool_const, emit_u64_overflow_guard, fresh, lower_expr, LowerCtx};

pub(super) fn lower_bin_expr<'a>(
    ctx: &mut LowerCtx<'a>,
    expr: &'a Expr,
    op: &BinOp,
    lhs: &'a Expr,
    rhs: &'a Expr,
    expected: Option<Type>,
) -> Result<Value> {
    let lt = infer_expr_type(
        lhs,
        &ctx.type_env,
        &ctx.fns,
        ctx.trait_env,
        ctx.aliases,
        ctx.type_defs,
        &ctx.type_params,
        &ctx.bounds,
    )?;
    let rt = infer_expr_type(
        rhs,
        &ctx.type_env,
        &ctx.fns,
        ctx.trait_env,
        ctx.aliases,
        ctx.type_defs,
        &ctx.type_params,
        &ctx.bounds,
    )?;
    if matches!(op, BinOp::Eq | BinOp::Neq) {
        let std_l = std_type_info_for(&lt, ctx.aliases, ctx.std_types)?;
        let std_r = std_type_info_for(&rt, ctx.aliases, ctx.std_types)?;
        if std_l.is_some() || std_r.is_some() {
            let lv = lower_expr(ctx, lhs, None)?;
            let rv = lower_expr(ctx, rhs, None)?;
            let eq_ty = if std_l.is_some() { lt.clone() } else { rt.clone() };
            let eq_val = emit_eq_for_type(ctx, &eq_ty, lv, rv, ctx.aliases)?;
            if matches!(op, BinOp::Neq) {
                let zero = emit_bool_const(ctx, false);
                let dst = fresh(ctx);
                ctx.body.push(Instr::IBin {
                    dst,
                    op: BinOpIR::Eq,
                    lhs: eq_val,
                    rhs: zero,
                    ty: IrType::Bool,
                });
                return Ok(dst);
            }
            return Ok(eq_val);
        }
    }
    let op_type = match op {
        BinOp::And | BinOp::Or => Type::Bool,
        BinOp::Eq | BinOp::Neq => {
            if matches!(expected, Some(Type::U64))
                || matches!(lt, Type::U64)
                || matches!(rt, Type::U64)
            {
                Type::U64
            } else {
                Type::Int
            }
        }
        _ => {
            if matches!(expected, Some(Type::U64))
                || matches!(lt, Type::U64)
                || matches!(rt, Type::U64)
            {
                Type::U64
            } else {
                Type::Int
            }
        }
    };
    let operand_expected = if matches!(op_type, Type::U64) {
        Some(Type::U64)
    } else {
        None
    };
    let lv = lower_expr(ctx, lhs, operand_expected.clone())?;
    let rv = lower_expr(ctx, rhs, operand_expected)?;
    let dst = fresh(ctx);
    let irop = match op {
        BinOp::Add => BinOpIR::Add,
        BinOp::Sub => BinOpIR::Sub,
        BinOp::Mul => BinOpIR::Mul,
        BinOp::Div => BinOpIR::Div,
        BinOp::Shl => BinOpIR::Shl,
        BinOp::Shr => BinOpIR::Shr,
        BinOp::BitAnd | BinOp::And => BinOpIR::And,
        BinOp::BitOr | BinOp::Or => BinOpIR::Or,
        BinOp::BitXor => BinOpIR::Xor,
        BinOp::Lt => BinOpIR::Lt,
        BinOp::Le => BinOpIR::Le,
        BinOp::Gt => BinOpIR::Gt,
        BinOp::Ge => BinOpIR::Ge,
        BinOp::Eq => BinOpIR::Eq,
        BinOp::Neq => BinOpIR::Neq,
    };
    let ir_op_ty = match op_type {
        Type::U64 => IrType::U64,
        Type::Bool => IrType::Bool,
        _ => IrType::Int,
    };
    ctx.body.push(Instr::IBin {
        dst,
        op: irop,
        lhs: lv,
        rhs: rv,
        ty: ir_op_ty,
    });
    if matches!(op_type, Type::U64) && matches!(op, BinOp::Add | BinOp::Sub | BinOp::Mul) {
        let span = expr_span_local(expr);
        emit_u64_overflow_guard(ctx, op, lv, rv, dst, span)?;
    }
    Ok(dst)
}
