use super::TyperError;
use clg_ast::{Span};

impl TyperError {
    pub fn resource_use_after_consume(name: &str, consumed_at: Span, use_span: Span) -> Self {
        Self::new(
            "T801",
            format!(
                "resource `{}` was consumed at {}..{} and cannot be used again",
                name, consumed_at.start, consumed_at.end
            ),
            use_span.start,
            use_span.end,
        )
    }


    pub fn resource_double_consume(name: &str, first: Span, second: Span) -> Self {
        Self::new(
            "T802",
            format!(
                "resource `{}` already consumed at {}..{}; second consume here",
                name, first.start, first.end
            ),
            second.start,
            second.end,
        )
    }


    pub fn resource_consume_borrow(name: &str, span: Span) -> Self {
        Self::new(
            "T803",
            format!("cannot consume borrowed resource `{}`", name),
            span.start,
            span.end,
        )
    }


    pub fn resource_branch_mismatch(name: &str, span: Span) -> Self {
        Self::new(
            "T804",
            format!(
                "resource `{}` has different ownership states across branches",
                name
            ),
            span.start,
            span.end,
        )
    }


    pub fn resource_not_consumed(name: &str) -> Self {
        Self::new(
            "T805",
            format!("resource `{}` must be consumed before returning", name),
            0,
            0,
        )
    }
}
