use crate::check::{base_type, infer_expr_type};
use anyhow::Result;
use clg_ast::{Expr, Span, Type};
use clg_ir::{BinOpIR, GuardKind, Instr, IrType, TrapCode, Value};

use super::array::{emit_array_data_ptr, emit_array_len};
use super::layout::{collection_layout, tuple_layout};
use super::{emit_int_const, fresh, load_value_borrow, lower_expr, LowerCtx};

pub(super) fn lower_index_expr<'a>(
    ctx: &mut LowerCtx<'a>,
    base: &'a Expr,
    index: &'a Expr,
    span: &Span,
) -> Result<Value> {
    let base_ty = infer_expr_type(
        base,
        &ctx.type_env,
        &ctx.fns,
        ctx.trait_env,
        ctx.aliases,
        ctx.type_defs,
        &ctx.type_params,
        &ctx.bounds,
    )?;
    let resolved = base_type(&base_ty, ctx.aliases)?;
    let base_ptr = lower_expr(ctx, base, None)?;
    match resolved {
        Type::Array(inner, _) | Type::Slice(inner) => {
            let elem_ty = *inner;
            let (_, _, stride) = collection_layout(&elem_ty, ctx.aliases, ctx.std_types)?;
            let len_val = emit_array_len(ctx, base_ptr);
            let data_ptr = emit_array_data_ptr(ctx, base_ptr);
            let idx_val = match index {
                Expr::Int(n, _) => emit_int_const(ctx, *n),
                _ => lower_expr(ctx, index, None)?,
            };
            let zero = emit_int_const(ctx, 0);
            let ge_zero = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: ge_zero,
                op: BinOpIR::Ge,
                lhs: idx_val,
                rhs: zero,
                ty: IrType::Int,
            });
            let lt_len = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: lt_len,
                op: BinOpIR::Lt,
                lhs: idx_val,
                rhs: len_val,
                ty: IrType::Int,
            });
            let ok = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: ok,
                op: BinOpIR::And,
                lhs: ge_zero,
                rhs: lt_len,
                ty: IrType::Bool,
            });
            ctx.body.push(Instr::Guard {
                cond: ok,
                trap: TrapCode::ContractViolation,
                span: Some((span.start as u32, span.end as u32)),
                detail: GuardKind::Require,
            });
            let stride_val = emit_int_const(ctx, stride as i64);
            let offset_val = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: offset_val,
                op: BinOpIR::Mul,
                lhs: idx_val,
                rhs: stride_val,
                ty: IrType::Int,
            });
            let addr = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: addr,
                op: BinOpIR::Add,
                lhs: data_ptr,
                rhs: offset_val,
                ty: IrType::Int,
            });
            load_value_borrow(ctx, &elem_ty, addr, 0)
        }
        Type::Tuple(elems) => {
            let idx = match index {
                Expr::Int(n, _) => *n,
                _ => anyhow::bail!("tuple index must be a constant integer"),
            };
            if idx < 0 || idx as usize >= elems.len() {
                anyhow::bail!("tuple index out of bounds in lowering");
            }
            let layout = tuple_layout(&elems, ctx.aliases, ctx.std_types)?;
            let elem_ty = elems[idx as usize].clone();
            let offset = *layout
                .offsets
                .get(idx as usize)
                .ok_or_else(|| anyhow::anyhow!("tuple offset missing"))?;
            load_value_borrow(ctx, &elem_ty, base_ptr, offset)
        }
        other => anyhow::bail!("indexing not supported for {:?}", other),
    }
}
