use crate::check::{infer_expr_type, LocalBinding};
use anyhow::Result;
use clg_ast::{Block, Expr, ParamKind, Span, Stmt, Type};
use clg_ir::{BinOpIR, GuardKind, Instr, IrType, TrapCode, Value};

use super::{emit_int_const, fresh, lower_expr, LowerCtx};

pub(super) fn lower_block_expr<'a>(
    ctx: &mut LowerCtx<'a>,
    block: &'a Block,
    expected: Option<Type>,
) -> Result<Value> {
    let mut inserted: Vec<ScopeEntry<'a>> = Vec::new();
    let result = (|| -> Result<Value> {
        lower_block_statements(ctx, block, &mut inserted)?;
        let tail = block
            .tail
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("block expressions require a tail value"))?;
        lower_expr(ctx, tail.as_ref(), expected)
    })();
    restore_scope(ctx, inserted);
    result
}

pub(super) fn lower_block_stmt<'a>(ctx: &mut LowerCtx<'a>, block: &'a Block) -> Result<()> {
    let mut inserted: Vec<ScopeEntry<'a>> = Vec::new();
    let result = (|| -> Result<()> {
        lower_block_statements(ctx, block, &mut inserted)?;
        if let Some(tail) = &block.tail {
            let _ = lower_expr(ctx, tail.as_ref(), None)?;
        }
        Ok(())
    })();
    restore_scope(ctx, inserted);
    result
}

fn lower_block_statements<'a>(
    ctx: &mut LowerCtx<'a>,
    block: &'a Block,
    inserted: &mut Vec<ScopeEntry<'a>>,
) -> Result<()> {
    for stmt in &block.statements {
        match stmt {
            Stmt::Let { name, expr, .. } => {
                let val = lower_expr(ctx, expr.as_ref(), None)?;
                let ty = infer_expr_type(
                    expr.as_ref(),
                    &ctx.type_env,
                    &ctx.fns,
                    ctx.trait_env,
                    ctx.aliases,
                    ctx.type_defs,
                    &ctx.type_params,
                    &ctx.bounds,
                )?;
                let key = name.as_str();
                let prev = ctx.env.insert(key, val);
                let prev_ty = ctx.type_env.insert(
                    key,
                    LocalBinding {
                        ty,
                        kind: ParamKind::Borrow,
                    },
                );
                inserted.push(ScopeEntry {
                    name: key,
                    prev_val: prev,
                    prev_ty,
                });
            }
            Stmt::Expr { expr, .. } => {
                let _ = lower_expr(ctx, expr.as_ref(), None)?;
            }
            Stmt::While {
                cond,
                invariant,
                variant,
                body,
                span,
            } => {
                lower_while_stmt(
                    ctx,
                    cond.as_ref(),
                    invariant.as_ref(),
                    variant.as_ref().map(|v| v.as_ref()),
                    body.as_ref(),
                    *span,
                )?;
            }
        }
    }
    Ok(())
}

pub(super) struct ScopeEntry<'a> {
    pub(super) name: &'a str,
    pub(super) prev_val: Option<Value>,
    pub(super) prev_ty: Option<LocalBinding>,
}

pub(super) fn restore_scope<'a>(ctx: &mut LowerCtx<'a>, inserted: Vec<ScopeEntry<'a>>) {
    for entry in inserted.into_iter().rev() {
        match entry.prev_val {
            Some(val) => {
                ctx.env.insert(entry.name, val);
            }
            None => {
                ctx.env.remove(entry.name);
            }
        }
        match entry.prev_ty {
            Some(ty) => {
                ctx.type_env.insert(entry.name, ty);
            }
            None => {
                ctx.type_env.remove(entry.name);
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn lower_while_stmt<'a>(
    ctx: &mut LowerCtx<'a>,
    cond: &'a Expr,
    invariant: &'a Expr,
    variant: Option<&'a Expr>,
    body: &'a Block,
    _span: Span,
) -> Result<()> {
    ctx.body.push(Instr::BlockBegin);
    ctx.body.push(Instr::LoopBegin);

    let inv_span = expr_span_local(invariant);
    let inv_head = lower_expr(ctx, invariant, None)?;
    push_guard(ctx, inv_head, inv_span, GuardKind::LoopInvariant);

    let cond_val = lower_expr(ctx, cond, None)?;
    ctx.body.push(Instr::BrIfEqz {
        cond: cond_val,
        depth: 1,
    });

    let mut before_variant: Option<(Value, Span)> = None;
    if let Some(var_expr) = variant {
        let v_before = lower_expr(ctx, var_expr, None)?;
        let v_span = expr_span_local(var_expr);
        push_non_negative_guard(ctx, v_before, v_span);
        before_variant = Some((v_before, v_span));
    }

    lower_block_stmt(ctx, body)?;

    let inv_tail = lower_expr(ctx, invariant, None)?;
    push_guard(ctx, inv_tail, inv_span, GuardKind::LoopInvariant);

    if let Some((v_before, v_span)) = before_variant {
        let v_after = lower_expr(ctx, variant.expect("variant expression lost"), None)?;
        push_non_negative_guard(ctx, v_after, v_span);
        let progress = fresh(ctx);
        ctx.body.push(Instr::IBin {
            dst: progress,
            op: BinOpIR::Lt,
            lhs: v_after,
            rhs: v_before,
            ty: IrType::Int,
        });
        push_guard(ctx, progress, v_span, GuardKind::LoopVariantProgress);
    }

    ctx.body.push(Instr::Br { depth: 0 });
    ctx.body.push(Instr::LoopEnd);
    ctx.body.push(Instr::BlockEnd);
    Ok(())
}

fn push_guard(ctx: &mut LowerCtx<'_>, cond: Value, span: Span, detail: GuardKind) {
    ctx.body.push(Instr::Guard {
        cond,
        trap: TrapCode::ContractViolation,
        span: Some((span.start as u32, span.end as u32)),
        detail,
    });
}

fn push_non_negative_guard(ctx: &mut LowerCtx<'_>, value: Value, span: Span) {
    let zero = emit_int_const(ctx, 0);
    let ok = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: ok,
        op: BinOpIR::Ge,
        lhs: value,
        rhs: zero,
        ty: IrType::Int,
    });
    push_guard(ctx, ok, span, GuardKind::LoopVariant);
}

pub(super) fn expr_span_local(e: &Expr) -> Span {
    match e {
        Expr::Int(_, sp) | Expr::Bool(_, sp) | Expr::String(_, sp) | Expr::Var(_, sp) => *sp,
        Expr::ArrayLit { span, .. }
        | Expr::TupleLit { span, .. }
        | Expr::StructLit { span, .. }
        | Expr::FieldAccess { span, .. }
        | Expr::Index { span, .. } => *span,
        Expr::Bin { span, .. }
        | Expr::Call { span, .. }
        | Expr::Match { span, .. }
        | Expr::Return { span, .. }
        | Expr::If { span, .. }
        | Expr::Unary { span, .. }
        | Expr::Try { span, .. } => *span,
        Expr::Block { block } => block.span,
    }
}
