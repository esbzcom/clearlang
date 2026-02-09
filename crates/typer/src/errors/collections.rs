use super::render_type;
use super::TyperError;
use crate::check::show_ty;
use clg_ast::{Span, Type};

impl TyperError {
    pub fn array_index_out_of_bounds(span: Span) -> Self {
        Self::new(
            "T114",
            format!(
                "at {}..{}: array index is out of bounds",
                span.start, span.end
            ),
            span.start,
            span.end,
        )
    }

    pub fn array_length_mismatch(expected: u32, found: u32, span: Span) -> Self {
        Self::new(
            "T116",
            format!(
                "at {}..{}: array length mismatch (expected {}, found {})",
                span.start, span.end, expected, found
            ),
            span.start,
            span.end,
        )
    }

    pub fn tuple_index_requires_constant(span: Span) -> Self {
        Self::new(
            "T115",
            format!(
                "at {}..{}: tuple index must be a constant integer",
                span.start, span.end
            ),
            span.start,
            span.end,
        )
    }

    pub fn element_type_mismatch(expected: Type, found: Type, span: Span) -> Self {
        Self::new(
            "T208",
            format!(
                "at {}..{}: element type mismatch: expected `{}`, found `{}`",
                span.start,
                span.end,
                show_ty(expected),
                show_ty(found)
            ),
            span.start,
            span.end,
        )
    }

    pub fn non_equatable_key(found: Type, span: Option<Span>) -> Self {
        let rendered = render_type(&found);
        match span {
            Some(sp) => Self::new(
                "T220",
                format!(
                    "at {}..{}: non-equatable key type for Map/Set: `{}`",
                    sp.start, sp.end, rendered
                ),
                sp.start,
                sp.end,
            ),
            None => Self::new(
                "T220",
                format!("non-equatable key type for Map/Set: `{}`", rendered),
                0,
                0,
            ),
        }
    }

    pub fn cannot_infer_collection(span: Span, kind: &str) -> Self {
        Self::new(
            "T206",
            format!(
                "at {}..{}: cannot infer element type for {}::new()",
                span.start, span.end, kind
            ),
            span.start,
            span.end,
        )
    }

    pub fn expected_collection(kind: &str, found: Type, span: Span) -> Self {
        Self::new(
            "T207",
            format!(
                "at {}..{}: expected {} argument, found `{}`",
                span.start,
                span.end,
                kind,
                show_ty(found)
            ),
            span.start,
            span.end,
        )
    }

    pub fn collections_unavailable(callee: &str, span: Span) -> Self {
        Self::new(
            "T101",
            format!(
                "at {}..{}: collections require generics/ADTs; `{}` is planned in later Phase 4.x slices",
                span.start, span.end, callee
            ),
            span.start,
            span.end,
        )
    }

    // Phase 4.5 — Match typing diagnostics

    pub fn resource_in_collection(found: Type, span: Option<Span>) -> Self {
        let rendered = render_type(&found);
        match span {
            Some(sp) => Self::new(
                "T806",
                format!(
                    "at {}..{}: resources cannot be stored inside collections or tuples yet; found `{}`",
                    sp.start, sp.end, rendered
                ),
                sp.start,
                sp.end,
            ),
            None => Self::new(
                "T806",
                format!(
                    "resources cannot be stored inside collections or tuples yet; found `{}`",
                    rendered
                ),
                0,
                0,
            ),
        }
    }

    pub fn resource_get_requires_move_out(callee: &str, value_ty: Type, span: Span) -> Self {
        Self::new(
            "T806",
            format!(
                "at {}..{}: `{}` cannot return owned `{}` from a resource collection; use a move-out method (e.g. `move_out`) instead",
                span.start,
                span.end,
                callee,
                render_type(&value_ty)
            ),
            span.start,
            span.end,
        )
    }

    pub fn resource_collection_op_requires_ownership_api(callee: &str, span: Span) -> Self {
        Self::new(
            "T806",
            format!(
                "at {}..{}: `{}` is not supported for resource collections yet; use an ownership API (e.g. `move_out`) instead",
                span.start, span.end, callee
            ),
            span.start,
            span.end,
        )
    }

    // Phase 6.6 - ADT sugar diagnostics
}
