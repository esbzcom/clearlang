use anyhow::Result;
use clg_ast::{Expr, Type};
use clg_ir::{BinOpIR, Instr, IrType, Value, VariantKind};

use super::{emit_int_const, fresh, lower_expr, LowerCtx};

pub(super) fn lower_try_expr<'a>(ctx: &mut LowerCtx<'a>, expr: &'a Expr) -> Result<Value> {
    let kind = match &ctx.ret_ty {
        Type::Option(_) => VariantKind::Option,
        Type::Result(_, _) => VariantKind::Result,
        other => {
            return Err(anyhow::anyhow!(
                "`?` requires Option/Result return type, found {:?}",
                other
            ));
        }
    };
    let variant = lower_expr(ctx, expr, None)?;
    let parts = ctx.variant_destructure(variant, kind);
    let failure_tag = emit_int_const(ctx, 0);
    let cond = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: cond,
        op: BinOpIR::Eq,
        lhs: parts.tag,
        rhs: failure_tag,
        ty: IrType::Int,
    });
    ctx.body.push(Instr::ReturnIf { cond, ret: variant });
    Ok(parts.payload_lo)
}

pub(super) fn lower_if_expr<'a>(
    ctx: &mut LowerCtx<'a>,
    cond: &'a Expr,
    then_br: &'a Expr,
    else_br: &'a Expr,
    expected: Option<Type>,
) -> Result<Value> {
    let cv = lower_expr(ctx, cond, None)?;
    let tv = lower_expr(ctx, then_br, expected.clone())?;
    let ev = lower_expr(ctx, else_br, expected)?;
    let dst = fresh(ctx);
    ctx.body.push(Instr::ISelect {
        dst,
        cond: cv,
        then_v: tv,
        else_v: ev,
    });
    Ok(dst)
}
