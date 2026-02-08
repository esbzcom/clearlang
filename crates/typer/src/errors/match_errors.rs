use super::TyperError;
use crate::check::show_ty;
use clg_ast::{Span, Type};

impl TyperError {
    pub fn match_non_exhaustive(span: Span) -> Self {
        Self::new(
            "T201",
            format!(
                "at {}..{}: non-exhaustive match (missing arm)",
                span.start, span.end
            ),
            span.start,
            span.end,
        )
    }

    pub fn match_duplicate_arm(arm: &str, span: Span) -> Self {
        Self::new(
            "T202",
            format!(
                "at {}..{}: duplicate match arm `{}`",
                span.start, span.end, arm
            ),
            span.start,
            span.end,
        )
    }

    pub fn match_invalid_scrutinee(found: Type, span: Span) -> Self {
        Self::new(
            "T203",
            format!(
                "at {}..{}: invalid match scrutinee: expected `Option`, `Result`, or enum type, found `{}`",
                span.start,
                span.end,
                show_ty(found)
            ),
            span.start,
            span.end,
        )
    }

    pub fn match_arm_type_mismatch(expected: Type, found: Type, span: Span) -> Self {
        Self::new(
            "T204",
            format!(
                "at {}..{}: match arm type mismatch: expected `{}`, found `{}`",
                span.start,
                span.end,
                show_ty(expected),
                show_ty(found)
            ),
            span.start,
            span.end,
        )
    }

    pub fn binder_conflict(name: &str, span: Span) -> Self {
        Self::new(
            "T205",
            format!(
                "at {}..{}: binder `{}` conflicts with an existing name",
                span.start, span.end, name
            ),
            span.start,
            span.end,
        )
    }

    pub fn match_unreachable_arm(span: Span) -> Self {
        Self::new(
            "T209",
            format!("at {}..{}: unreachable match arm", span.start, span.end),
            span.start,
            span.end,
        )
    }
}
