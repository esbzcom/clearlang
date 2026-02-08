use super::TyperError;
use crate::check::show_ty;
use clg_ast::{Span, Type};

impl TyperError {
    pub fn unsigned_int_not_supported(ty: Type, span: Option<Span>) -> Self {
        if let Some(sp) = span {
            Self::new(
                "T110",
                format!(
                    "at {}..{}: unsigned integer operations for `{}` are not supported yet",
                    sp.start,
                    sp.end,
                    show_ty(ty)
                ),
                sp.start,
                sp.end,
            )
        } else {
            Self::new(
                "T110",
                format!(
                    "unsigned integer operations for `{}` are not supported yet",
                    show_ty(ty)
                ),
                0,
                0,
            )
        }
    }


    pub fn unsigned_cast_invalid(target: &str, found: Type, span: Span) -> Self {
        Self::new(
            "T111",
            format!(
                "at {}..{}: {} cast expects an unsigned literal or {}, found `{}`",
                span.start,
                span.end,
                target,
                target,
                show_ty(found)
            ),
            span.start,
            span.end,
        )
    }


    pub fn unsigned_literal_out_of_range(target: Type, value: u128, span: Span) -> Self {
        let max = match target {
            Type::U8 => Some(u8::MAX as u128),
            Type::U64 => Some(u64::MAX as u128),
            Type::U128 => Some(u128::MAX),
            Type::U256 => None,
            _ => None,
        };
        let suffix = match max {
            Some(max) => format!("max {}", max),
            None => "max unbounded".to_string(),
        };
        Self::new(
            "T112",
            format!(
                "at {}..{}: unsigned literal out of range for `{}`: {value} ({suffix})",
                span.start,
                span.end,
                show_ty(target)
            ),
            span.start,
            span.end,
        )
    }


    pub fn unsigned_constant_overflow(op: &str, span: Span) -> Self {
        Self::new(
            "T113",
            format!(
                "at {}..{}: unsigned constant overflow in `{}`",
                span.start, span.end, op
            ),
            span.start,
            span.end,
        )
    }
}
