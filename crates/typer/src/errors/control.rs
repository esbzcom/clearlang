use super::TyperError;
use crate::check::show_ty;
use clg_ast::{Span, Type};

impl TyperError {
    pub fn block_missing_tail(span: Span) -> Self {
        Self::new(
            "T016",
            format!(
                "at {}..{}: block expression requires a tail expression",
                span.start, span.end
            ),
            span.start,
            span.end,
        )
    }

    pub fn while_variant_required(span: Span) -> Self {
        Self::new(
            "T901",
            format!(
                "at {}..{}: while loops in pure functions require a variant (measure) for totality",
                span.start, span.end
            ),
            span.start,
            span.end,
        )
    }

    pub fn recursion_requires_measure(callee: &str, span: Span) -> Self {
        Self::new(
            "T902",
            format!(
                "at {}..{}: recursive call to `{}` requires a decreasing measure for totality",
                span.start, span.end, callee
            ),
            span.start,
            span.end,
        )
    }

    pub fn variant_not_decreasing(span: Span) -> Self {
        Self::new(
            "T903",
            format!(
                "at {}..{}: loop variant must decrease each iteration; constant measures are not allowed",
                span.start, span.end
            ),
            span.start,
            span.end,
        )
    }

    pub fn try_option_return_required(found: Type, span: Span) -> Self {
        Self::new(
            "T601",
            format!(
                "at {}..{}: ? requires function return type Option<_>, found {}",
                span.start,
                span.end,
                show_ty(found)
            ),
            span.start,
            span.end,
        )
    }

    pub fn try_input_not_option_result(found: Type, span: Span) -> Self {
        Self::new(
            "T602",
            format!(
                "at {}..{}: ? operand must be Option<_> or Result<_, _>, found {}",
                span.start,
                span.end,
                show_ty(found)
            ),
            span.start,
            span.end,
        )
    }

    pub fn try_option_inner_mismatch(declared: Type, found: Type, span: Span) -> Self {
        Self::new(
            "T603",
            format!(
                "at {}..{}: ? expects Option inner type {} but expression yields {}",
                span.start,
                span.end,
                show_ty(declared),
                show_ty(found)
            ),
            span.start,
            span.end,
        )
    }

    pub fn try_result_return_required(found: Type, span: Span) -> Self {
        Self::new(
            "T604",
            format!(
                "at {}..{}: ? requires function return type Result<_, _>, found {}",
                span.start,
                span.end,
                show_ty(found)
            ),
            span.start,
            span.end,
        )
    }

    pub fn try_result_mismatch(
        ok_decl: Type,
        err_decl: Type,
        ok_found: Type,
        err_found: Type,
        span: Span,
    ) -> Self {
        Self::new(
            "T605",
            format!(
                "at {}..{}: ? expects Result types {}/{} but expression yields {}/{}",
                span.start,
                span.end,
                show_ty(ok_decl),
                show_ty(err_decl),
                show_ty(ok_found),
                show_ty(err_found)
            ),
            span.start,
            span.end,
        )
    }

    pub fn try_missing_return(span: Span) -> Self {
        Self::new(
            "T606",
            format!(
                "at {}..{}: internal error: missing $return binding for ?",
                span.start, span.end
            ),
            span.start,
            span.end,
        )
    }

    pub fn option_ctor_missing_return(span: Span) -> Self {
        Self::new(
            "T607",
            format!(
                "at {}..{}: internal error: missing  binding for Option constructor",
                span.start, span.end
            ),
            span.start,
            span.end,
        )
    }

    pub fn none_return_required(found: Type, span: Span) -> Self {
        Self::new(
            "T608",
            format!(
                "at {}..{}: None requires function return type Option<_>, found {}",
                span.start,
                span.end,
                show_ty(found)
            ),
            span.start,
            span.end,
        )
    }

    pub fn result_ctor_missing_return(span: Span) -> Self {
        Self::new(
            "T609",
            format!(
                "at {}..{}: internal error: missing  binding for Result constructor",
                span.start, span.end
            ),
            span.start,
            span.end,
        )
    }

    pub fn ok_return_required(found: Type, span: Span) -> Self {
        Self::new(
            "T610",
            format!(
                "at {}..{}: Ok requires function return type Result<_, _>, found {}",
                span.start,
                span.end,
                show_ty(found)
            ),
            span.start,
            span.end,
        )
    }

    pub fn ok_argument_mismatch(expected: Type, found: Type, span: Span) -> Self {
        Self::new(
            "T611",
            format!(
                "at {}..{}: Ok argument type mismatch: expected {}, found {}",
                span.start,
                span.end,
                show_ty(expected),
                show_ty(found)
            ),
            span.start,
            span.end,
        )
    }

    pub fn err_return_required(found: Type, span: Span) -> Self {
        Self::new(
            "T612",
            format!(
                "at {}..{}: Err requires function return type Result<_, _>, found {}",
                span.start,
                span.end,
                show_ty(found)
            ),
            span.start,
            span.end,
        )
    }

    pub fn err_argument_mismatch(expected: Type, found: Type, span: Span) -> Self {
        Self::new(
            "T613",
            format!(
                "at {}..{}: Err argument type mismatch: expected {}, found {}",
                span.start,
                span.end,
                show_ty(expected),
                show_ty(found)
            ),
            span.start,
            span.end,
        )
    }
    // Phase 4.10 — Conditionals typing diagnostics

    pub fn branch_type_mismatch(expected: Type, found: Type, span: Span) -> Self {
        Self::new(
            "T301",
            format!(
                "at {}..{}: branch type mismatch: expected `{}`, found `{}`",
                span.start,
                span.end,
                show_ty(expected),
                show_ty(found)
            ),
            span.start,
            span.end,
        )
    }
}
