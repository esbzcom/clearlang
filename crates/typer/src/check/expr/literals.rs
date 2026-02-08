use anyhow::Result;
use clg_ast::{BinOp, Expr, Span, Type};

use super::expr_span;
use super::super::{base_type, AliasMap};
use crate::errors::TyperError;

pub(super) fn ensure_int(ty: Type, aliases: &AliasMap, what: &str, span: Option<Span>) -> Result<()> {
    if base_type(&ty, aliases)? != Type::Int {
        return Err(TyperError::int_operand(what, ty, span).into());
    }
    Ok(())
}

pub(super) fn ensure_bool(ty: Type, aliases: &AliasMap, what: &str, span: Option<Span>) -> Result<()> {
    if base_type(&ty, aliases)? != Type::Bool {
        return Err(TyperError::bool_operand(what, ty, span).into());
    }
    Ok(())
}

pub(super) fn unsigned_literal_value(expr: &Expr) -> Option<u128> {
    match expr {
        Expr::Int(value, _) => (*value >= 0).then_some(*value as u128),
        Expr::Return { expr, .. } => unsigned_literal_value(expr),
        Expr::Block { block } if block.statements.is_empty() => block
            .tail
            .as_ref()
            .and_then(|tail| unsigned_literal_value(tail)),
        Expr::Call { callee, args, .. }
            if matches!(callee.as_str(), "U8" | "U64" | "U128" | "U256") && args.len() == 1 =>
        {
            unsigned_literal_value(&args[0])
        }
        _ => None,
    }
}

pub(super) fn int_literal_value(expr: &Expr) -> Option<i64> {
    match expr {
        Expr::Int(value, _) => Some(*value),
        Expr::Return { expr, .. } => int_literal_value(expr),
        Expr::Block { block } if block.statements.is_empty() => {
            block.tail.as_ref().and_then(|tail| int_literal_value(tail))
        }
        _ => None,
    }
}

fn unsigned_literal_max(target: &Type) -> Option<u128> {
    match target {
        Type::U8 => Some(u8::MAX as u128),
        Type::U64 => Some(u64::MAX as u128),
        Type::U128 => Some(u128::MAX),
        Type::U256 => None,
        _ => None,
    }
}

pub(super) fn unsigned_literal_fits_value(target: &Type, value: u128) -> bool {
    match unsigned_literal_max(target) {
        Some(max) => value <= max,
        None => true,
    }
}

fn unsigned_literal_fits(target: &Type, expr: &Expr) -> bool {
    unsigned_literal_value(expr)
        .map(|value| unsigned_literal_fits_value(target, value))
        .unwrap_or(false)
}

pub(crate) fn literal_can_coerce_unsigned(expected: &Type, actual: &Type, expr: &Expr) -> bool {
    matches!(expected, Type::U8 | Type::U64 | Type::U128 | Type::U256)
        && matches!(actual, Type::Int)
        && unsigned_literal_fits(expected, expr)
}

pub(crate) fn unsigned_literal_range_error(
    expected: &Type,
    actual: &Type,
    expr: &Expr,
) -> Option<TyperError> {
    if matches!(expected, Type::U8 | Type::U64 | Type::U128 | Type::U256)
        && matches!(actual, Type::Int)
    {
        if let Some(value) = unsigned_literal_value(expr) {
            if !unsigned_literal_fits_value(expected, value) {
                return Some(TyperError::unsigned_literal_out_of_range(
                    expected.clone(),
                    value,
                    expr_span(expr),
                ));
            }
        }
    }
    None
}

pub(super) fn u64_literal_overflow(op: &BinOp, lhs: u128, rhs: u128) -> bool {
    let max = u64::MAX as u128;
    match op {
        BinOp::Add => lhs + rhs > max,
        BinOp::Sub => rhs > lhs,
        BinOp::Mul => lhs
            .checked_mul(rhs)
            .map(|value| value > max)
            .unwrap_or(true),
        _ => false,
    }
}
