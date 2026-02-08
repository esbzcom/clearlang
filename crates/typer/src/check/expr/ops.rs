use anyhow::Result;
use clg_ast::{BinOp, Expr, Span, Type, UnaryOp};
use std::collections::{HashMap, HashSet};

use super::super::{
    base_type, base_types_match, AliasMap, BoundsMap, FnSig, LocalBinding, TraitEnv, TypeDefs,
};
use super::literals::{
    ensure_bool, literal_can_coerce_unsigned, u64_literal_overflow, unsigned_literal_range_error,
    unsigned_literal_value,
};
use super::{type_of, ResourceTracker};
use crate::errors::TyperError;

pub(super) fn type_unary_expr<'a>(
    op: UnaryOp,
    expr: &Expr,
    span: Span,
    env: &HashMap<&'a str, LocalBinding>,
    tracker: &mut ResourceTracker,
    fns: &HashMap<&'a str, FnSig>,
    trait_env: &TraitEnv<'a>,
    aliases: &AliasMap,
    type_defs: &TypeDefs,
    type_params: &HashSet<String>,
    bounds: &BoundsMap,
    depth: usize,
) -> Result<Type> {
    let inner = type_of(
        expr,
        env,
        tracker,
        fns,
        trait_env,
        aliases,
        type_defs,
        type_params,
        bounds,
        depth + 1,
        None,
    )?;
    match op {
        UnaryOp::Not => {
            ensure_bool(inner, aliases, "operand", Some(span))?;
            Ok(Type::Bool)
        }
    }
}

pub(super) fn type_bin_expr<'a>(
    op: BinOp,
    lhs: &Expr,
    rhs: &Expr,
    span: Span,
    env: &HashMap<&'a str, LocalBinding>,
    tracker: &mut ResourceTracker,
    fns: &HashMap<&'a str, FnSig>,
    trait_env: &TraitEnv<'a>,
    aliases: &AliasMap,
    type_defs: &TypeDefs,
    type_params: &HashSet<String>,
    bounds: &BoundsMap,
    depth: usize,
) -> Result<Type> {
    let lt = type_of(
        lhs,
        env,
        tracker,
        fns,
        trait_env,
        aliases,
        type_defs,
        type_params,
        bounds,
        depth + 1,
        None,
    )?;
    let rt = type_of(
        rhs,
        env,
        tracker,
        fns,
        trait_env,
        aliases,
        type_defs,
        type_params,
        bounds,
        depth + 1,
        None,
    )?;
    match op {
        BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div => {
            let op_str = match op {
                BinOp::Add => "+",
                BinOp::Sub => "-",
                BinOp::Mul => "*",
                BinOp::Div => "/",
                _ => "?",
            };
            let lt_base = base_type(&lt, aliases)?;
            let rt_base = base_type(&rt, aliases)?;
            if matches!(lt_base, Type::U128 | Type::U256) {
                return Err(TyperError::unsigned_int_not_supported(lt_base, Some(span)).into());
            }
            if matches!(rt_base, Type::U128 | Type::U256) {
                return Err(TyperError::unsigned_int_not_supported(rt_base, Some(span)).into());
            }
            if matches!(lt_base, Type::U64) || matches!(rt_base, Type::U64) {
                if let (Some(lv), Some(rv)) =
                    (unsigned_literal_value(lhs), unsigned_literal_value(rhs))
                {
                    if u64_literal_overflow(&op, lv, rv) {
                        return Err(TyperError::unsigned_constant_overflow(op_str, span).into());
                    }
                }
            }
            let lt_is_int = matches!(lt_base, Type::Int);
            let rt_is_int = matches!(rt_base, Type::Int);
            let lt_is_u64 = matches!(lt_base, Type::U64);
            let rt_is_u64 = matches!(rt_base, Type::U64);
            if lt_is_int && !rt_is_int && !rt_is_u64 {
                return Err(TyperError::int_operand("right operand", rt, Some(span)).into());
            }
            if rt_is_int && !lt_is_int && !lt_is_u64 {
                return Err(TyperError::int_operand("left operand", lt, Some(span)).into());
            }
            match (&lt_base, &rt_base) {
                (Type::U64, Type::U64) => Ok(Type::U64),
                (Type::U64, Type::Int) if unsigned_literal_value(rhs).is_some() => Ok(Type::U64),
                (Type::Int, Type::U64) if unsigned_literal_value(lhs).is_some() => Ok(Type::U64),
                (Type::Int, Type::Int) => Ok(Type::Int),
                _ => Err(TyperError::binary_operands_mismatch(op_str, lt, rt, span).into()),
            }
        }
        BinOp::BitAnd | BinOp::BitOr | BinOp::BitXor => {
            let op_str = match op {
                BinOp::BitAnd => "&",
                BinOp::BitOr => "|",
                BinOp::BitXor => "^",
                _ => "?",
            };
            let lt_base = base_type(&lt, aliases)?;
            let rt_base = base_type(&rt, aliases)?;
            if matches!(lt_base, Type::U128 | Type::U256) {
                return Err(TyperError::unsigned_int_not_supported(lt_base, Some(span)).into());
            }
            if matches!(rt_base, Type::U128 | Type::U256) {
                return Err(TyperError::unsigned_int_not_supported(rt_base, Some(span)).into());
            }
            match (&lt_base, &rt_base) {
                (Type::U64, Type::U64) => Ok(Type::U64),
                (Type::U64, Type::Int) if unsigned_literal_value(rhs).is_some() => Ok(Type::U64),
                (Type::Int, Type::U64) if unsigned_literal_value(lhs).is_some() => Ok(Type::U64),
                (Type::Int, Type::Int) => Ok(Type::Int),
                _ => Err(TyperError::binary_operands_mismatch(op_str, lt, rt, span).into()),
            }
        }
        BinOp::Shl | BinOp::Shr => {
            let op_str = match op {
                BinOp::Shl => "<<",
                BinOp::Shr => ">>",
                _ => "?",
            };
            let lt_base = base_type(&lt, aliases)?;
            let rt_base = base_type(&rt, aliases)?;
            if matches!(lt_base, Type::U128 | Type::U256) {
                return Err(TyperError::unsigned_int_not_supported(lt_base, Some(span)).into());
            }
            if matches!(rt_base, Type::U128 | Type::U256) {
                return Err(TyperError::unsigned_int_not_supported(rt_base, Some(span)).into());
            }
            match (&lt_base, &rt_base) {
                (Type::Int, Type::Int) => Ok(Type::Int),
                (Type::U64, Type::U64) => Ok(Type::U64),
                (Type::U64, Type::Int) if unsigned_literal_value(rhs).is_some() => Ok(Type::U64),
                _ => Err(TyperError::binary_operands_mismatch(op_str, lt, rt, span).into()),
            }
        }
        BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
            let op_str = match op {
                BinOp::Lt => "<",
                BinOp::Le => "<=",
                BinOp::Gt => ">",
                BinOp::Ge => ">=",
                _ => "?",
            };
            let lt_base = base_type(&lt, aliases)?;
            let rt_base = base_type(&rt, aliases)?;
            if matches!(lt_base, Type::U128 | Type::U256) {
                return Err(TyperError::unsigned_int_not_supported(lt_base, Some(span)).into());
            }
            if matches!(rt_base, Type::U128 | Type::U256) {
                return Err(TyperError::unsigned_int_not_supported(rt_base, Some(span)).into());
            }
            match (&lt_base, &rt_base) {
                (Type::U64, Type::U64) => Ok(Type::Bool),
                (Type::U64, Type::Int) if unsigned_literal_value(rhs).is_some() => Ok(Type::Bool),
                (Type::Int, Type::U64) if unsigned_literal_value(lhs).is_some() => Ok(Type::Bool),
                (Type::Int, Type::Int) => Ok(Type::Bool),
                _ => Err(TyperError::binary_operands_mismatch(op_str, lt, rt, span).into()),
            }
        }
        BinOp::Eq | BinOp::Neq => {
            let lt_base = base_type(&lt, aliases)?;
            let rt_base = base_type(&rt, aliases)?;
            if matches!(lt_base, Type::U128 | Type::U256) {
                return Err(TyperError::unsigned_int_not_supported(lt_base, Some(span)).into());
            }
            if matches!(rt_base, Type::U128 | Type::U256) {
                return Err(TyperError::unsigned_int_not_supported(rt_base, Some(span)).into());
            }
            if !base_types_match(&lt, &rt, aliases)?
                && !literal_can_coerce_unsigned(&lt, &rt, rhs)
                && !literal_can_coerce_unsigned(&rt, &lt, lhs)
            {
                if let Some(err) = unsigned_literal_range_error(&lt, &rt, rhs) {
                    return Err(err.into());
                }
                if let Some(err) = unsigned_literal_range_error(&rt, &lt, lhs) {
                    return Err(err.into());
                }
                let op_str = if op == BinOp::Eq { "==" } else { "!=" };
                return Err(TyperError::binary_operands_mismatch(op_str, lt, rt, span).into());
            }
            Ok(Type::Bool)
        }
        BinOp::And | BinOp::Or => {
            ensure_bool(lt, aliases, "left operand", Some(span))?;
            ensure_bool(rt, aliases, "right operand", Some(span))?;
            Ok(Type::Bool)
        }
    }
}
