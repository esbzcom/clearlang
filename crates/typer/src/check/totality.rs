use super::EffectLevel;
use crate::errors::TyperError;
use anyhow::Result;
use clg_ast::{Block, Effect, Expr, Func, Stmt};

pub(super) fn enforce_totality(func: &Func) -> Result<()> {
    if matches!(func.effect, Effect::None | Effect::Pure) {
        check_totality_expr(&func.body, &func.name)?;
    }
    Ok(())
}

pub(super) fn level_from_effect(effect: Effect) -> EffectLevel {
    match effect {
        Effect::None | Effect::Pure => EffectLevel::Pure,
        Effect::Mut => EffectLevel::Mut,
        Effect::Io => EffectLevel::Io,
    }
}

fn check_totality_expr(expr: &Expr, self_name: &str) -> Result<()> {
    match expr {
        Expr::Block { block } => check_totality_block(block, self_name)?,
        Expr::If {
            cond,
            then_br,
            else_br,
            ..
        } => {
            check_totality_expr(cond, self_name)?;
            check_totality_expr(then_br, self_name)?;
            check_totality_expr(else_br, self_name)?;
        }
        Expr::Match {
            scrutinee, arms, ..
        } => {
            check_totality_expr(scrutinee, self_name)?;
            for arm in arms {
                check_totality_expr(&arm.expr, self_name)?;
            }
        }
        Expr::ArrayLit { elems, .. } | Expr::TupleLit { elems, .. } => {
            for elem in elems {
                check_totality_expr(elem, self_name)?;
            }
        }
        Expr::StructLit { fields, .. } => {
            for field in fields {
                check_totality_expr(&field.expr, self_name)?;
            }
        }
        Expr::FieldAccess { base, .. } => {
            check_totality_expr(base, self_name)?;
        }
        Expr::Index { base, index, .. } => {
            check_totality_expr(base, self_name)?;
            check_totality_expr(index, self_name)?;
        }
        Expr::Unary { expr, .. } | Expr::Return { expr, .. } | Expr::Try { expr, .. } => {
            check_totality_expr(expr, self_name)?;
        }
        Expr::Bin { lhs, rhs, .. } => {
            check_totality_expr(lhs, self_name)?;
            check_totality_expr(rhs, self_name)?;
        }
        Expr::Call { callee, args, span } => {
            if callee == self_name {
                return Err(TyperError::recursion_requires_measure(callee, *span).into());
            }
            for arg in args {
                check_totality_expr(arg, self_name)?;
            }
        }
        Expr::Lambda { body, .. } => {
            check_totality_expr(body, self_name)?;
        }
        Expr::Int(_, _) | Expr::Bool(_, _) | Expr::String(_, _) | Expr::Var(_, _) => {}
    }
    Ok(())
}

fn check_totality_block(block: &Block, self_name: &str) -> Result<()> {
    for stmt in &block.statements {
        match stmt {
            Stmt::Let { expr, .. } | Stmt::Expr { expr, .. } => {
                check_totality_expr(expr.as_ref(), self_name)?;
            }
            Stmt::While {
                cond,
                invariant,
                variant,
                body,
                span,
            } => {
                if variant.is_none() {
                    return Err(TyperError::while_variant_required(*span).into());
                }
                check_totality_expr(cond.as_ref(), self_name)?;
                check_totality_expr(invariant.as_ref(), self_name)?;
                if let Some(v) = variant {
                    check_totality_expr(v.as_ref(), self_name)?;
                    if let Expr::Int(_, sp) = v.as_ref() {
                        return Err(TyperError::variant_not_decreasing(*sp).into());
                    }
                }
                check_totality_block(body.as_ref(), self_name)?;
            }
        }
    }
    if let Some(tail) = &block.tail {
        check_totality_expr(tail.as_ref(), self_name)?;
    }
    Ok(())
}
