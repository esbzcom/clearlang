use super::TyperError;
use crate::check::show_ty;
use clg_ast::{Span, Type};

impl TyperError {
    pub fn unknown_function(callee: &str, span: Span) -> Self {
        Self::new(
            "T001",
            format!(
                "at {}..{}: unknown function `{}`",
                span.start, span.end, callee
            ),
            span.start,
            span.end,
        )
    }

    pub fn arity_mismatch(callee: &str, expected: usize, found: usize, span: Span) -> Self {
        Self::new(
            "T002",
            format!(
                "at {}..{}: arity mismatch calling `{}`: expected {}, found {}",
                span.start, span.end, callee, expected, found
            ),
            span.start,
            span.end,
        )
    }

    pub fn arg_type_mismatch(
        i: usize,
        callee: &str,
        expected: Type,
        found: Type,
        span: Span,
    ) -> Self {
        Self::new(
            "T003",
            format!(
                "at {}..{}: arg {} type mismatch calling `{}`: expected `{}`, found `{}`",
                span.start,
                span.end,
                i,
                callee,
                show_ty(expected),
                show_ty(found)
            ),
            span.start,
            span.end,
        )
    }

    pub fn return_type_mismatch(declared: Type, found: Type, span: Span) -> Self {
        Self::new(
            "T004",
            format!(
                "at {}..{}: return type mismatch: declared `{}`, found `{}`",
                span.start,
                span.end,
                show_ty(declared),
                show_ty(found)
            ),
            span.start,
            span.end,
        )
    }

    pub fn int_operand(what: &str, ty: Type, span: Option<Span>) -> Self {
        if let Some(sp) = span {
            Self::new(
                "T005",
                format!(
                    "at {}..{}: {} must be Int, found `{}`",
                    sp.start,
                    sp.end,
                    what,
                    show_ty(ty)
                ),
                sp.start,
                sp.end,
            )
        } else {
            Self::new(
                "T005",
                format!("{} must be Int, found `{}`", what, show_ty(ty)),
                0,
                0,
            )
        }
    }

    pub fn bool_operand(what: &str, ty: Type, span: Option<Span>) -> Self {
        if let Some(sp) = span {
            Self::new(
                "T012",
                format!(
                    "at {}..{}: {} must be Bool, found `{}`",
                    sp.start,
                    sp.end,
                    what,
                    show_ty(ty)
                ),
                sp.start,
                sp.end,
            )
        } else {
            Self::new(
                "T012",
                format!("{} must be Bool, found `{}`", what, show_ty(ty)),
                0,
                0,
            )
        }
    }

    pub fn binary_operands_mismatch(op: &str, left: Type, right: Type, span: Span) -> Self {
        Self::new(
            "T013",
            format!(
                "at {}..{}: operands of `{}` must have the same type, found `{}` and `{}`",
                span.start,
                span.end,
                op,
                show_ty(left),
                show_ty(right)
            ),
            span.start,
            span.end,
        )
    }

    pub fn contract_not_bool(kind: &str, ty: Type, span: Span) -> Self {
        Self::new(
            "T014",
            format!(
                "at {}..{}: `{}` must be Bool, found `{}`",
                span.start,
                span.end,
                kind,
                show_ty(ty)
            ),
            span.start,
            span.end,
        )
    }

    pub fn unknown_variable(name: &str, sp: Span) -> Self {
        Self::new(
            "T006",
            format!("at {}..{}: unknown variable `{}`", sp.start, sp.end, name),
            sp.start,
            sp.end,
        )
    }

    pub fn duplicate_function(name: &str) -> Self {
        Self::new("T008", format!("duplicate function `{}`", name), 0, 0)
    }

    pub fn duplicate_parameter(name: &str) -> Self {
        Self::new("T010", format!("duplicate parameter `{}`", name), 0, 0)
    }

    pub fn feature_not_supported(feature: &str, span: Span) -> Self {
        Self::new(
            "T017",
            format!(
                "at {}..{}: {} not supported yet",
                span.start, span.end, feature
            ),
            span.start,
            span.end,
        )
    }

    pub fn reserved_function_namespace(name: &str) -> Self {
        Self::new(
            "T017",
            format!(
                "function name `{}` uses reserved compiler namespace `__clg_`",
                name
            ),
            0,
            0,
        )
    }
}
