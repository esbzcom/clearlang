use super::render_type;
use super::TyperError;
use clg_ast::{Span, Type};

impl TyperError {
    pub fn cyclic_alias(name: &str, span: Span) -> Self {
        Self::new(
            "T703",
            format!(
                "at {}..{}: cyclic refinement alias detected involving `{}`",
                span.start, span.end, name
            ),
            span.start,
            span.end,
        )
    }

    pub fn alias_predicate_not_bool(name: &str, span: Span) -> Self {
        Self::new(
            "T704",
            format!(
                "at {}..{}: refinement predicate for `{}` must be Bool",
                span.start, span.end, name
            ),
            span.start,
            span.end,
        )
    }

    pub fn refinement_loss(expected: Type, found: Type, span: Span) -> Self {
        Self::new(
            "T705",
            format!(
                "at {}..{}: refinement would be lost: expected `{}`, found `{}`",
                span.start,
                span.end,
                render_type(&expected),
                render_type(&found)
            ),
            span.start,
            span.end,
        )
    }

    pub fn refined_resource_not_supported(name: &str, span: Span) -> Self {
        Self::new(
            "T706",
            format!(
                "at {}..{}: refined alias `{}` cannot wrap resource types yet",
                span.start, span.end, name
            ),
            span.start,
            span.end,
        )
    }

    pub fn alias_predicate_impure(name: &str, span: Span) -> Self {
        Self::new(
            "T707",
            format!(
                "at {}..{}: refinement predicate for `{}` must be pure",
                span.start, span.end, name
            ),
            span.start,
            span.end,
        )
    }

    pub fn alias_predicate_unsat(name: &str, span: Span) -> Self {
        Self::new(
            "T708",
            format!(
                "at {}..{}: refinement predicate for `{}` is unsatisfiable",
                span.start, span.end, name
            ),
            span.start,
            span.end,
        )
    }
}
