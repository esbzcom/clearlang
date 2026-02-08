use super::TyperError;
use crate::check::show_ty;
use clg_ast::{Span, Type};

impl TyperError {
    pub fn cannot_infer_type_params(callee: &str, span: Span) -> Self {
        Self::new(
            "T238",
            format!(
                "at {}..{}: cannot infer type parameters for `{}`",
                span.start, span.end, callee
            ),
            span.start,
            span.end,
        )
    }

    pub fn duplicate_type_param(name: &str, span: Span) -> Self {
        Self::new(
            "T239",
            format!(
                "at {}..{}: duplicate type parameter `{}`",
                span.start, span.end, name
            ),
            span.start,
            span.end,
        )
    }

    pub fn type_param_conflict(name: &str, span: Span) -> Self {
        Self::new(
            "T240",
            format!(
                "at {}..{}: type parameter `{}` conflicts with an existing type name",
                span.start, span.end, name
            ),
            span.start,
            span.end,
        )
    }

    pub fn unknown_type_param(name: &str, span: Span) -> Self {
        Self::new(
            "T241",
            format!(
                "at {}..{}: unknown type parameter `{}` in bound",
                span.start, span.end, name
            ),
            span.start,
            span.end,
        )
    }

    pub fn type_arg_count_mismatch(
        name: &str,
        expected: usize,
        found: usize,
        span: Option<Span>,
    ) -> Self {
        if let Some(sp) = span {
            Self::new(
                "T242",
                format!(
                    "at {}..{}: `{}` expects {} type argument(s), found {}",
                    sp.start, sp.end, name, expected, found
                ),
                sp.start,
                sp.end,
            )
        } else {
            Self::new(
                "T242",
                format!(
                    "`{}` expects {} type argument(s), found {}",
                    name, expected, found
                ),
                0,
                0,
            )
        }
    }

    pub fn type_param_has_args(name: &str, span: Option<Span>) -> Self {
        if let Some(sp) = span {
            Self::new(
                "T243",
                format!(
                    "at {}..{}: type parameter `{}` cannot take type arguments",
                    sp.start, sp.end, name
                ),
                sp.start,
                sp.end,
            )
        } else {
            Self::new(
                "T243",
                format!("type parameter `{}` cannot take type arguments", name),
                0,
                0,
            )
        }
    }

    pub fn generic_alias_not_supported(name: &str, span: Span) -> Self {
        Self::new(
            "T244",
            format!(
                "at {}..{}: generic refinement alias `{}` is not supported yet",
                span.start, span.end, name
            ),
            span.start,
            span.end,
        )
    }

    pub fn impl_method_generics_not_supported(name: &str, span: Option<Span>) -> Self {
        if let Some(sp) = span {
            Self::new(
                "T245",
                format!(
                    "at {}..{}: impl method `{}` cannot declare its own type parameters yet",
                    sp.start, sp.end, name
                ),
                sp.start,
                sp.end,
            )
        } else {
            Self::new(
                "T245",
                format!(
                    "impl method `{}` cannot declare its own type parameters yet",
                    name
                ),
                0,
                0,
            )
        }
    }

    pub fn trait_type_params_not_supported(name: &str, span: Span) -> Self {
        Self::new(
            "T246",
            format!(
                "at {}..{}: trait `{}` cannot declare type parameters yet",
                span.start, span.end, name
            ),
            span.start,
            span.end,
        )
    }

    pub fn type_param_mismatch(name: &str, expected: Type, found: Type) -> Self {
        Self::new(
            "T247",
            format!(
                "conflicting inference for `{}`: expected `{}`, found `{}`",
                name,
                show_ty(expected),
                show_ty(found)
            ),
            0,
            0,
        )
    }
}
