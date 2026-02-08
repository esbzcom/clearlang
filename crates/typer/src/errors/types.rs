use super::TyperError;
use crate::check::show_ty;
use clg_ast::{Span, Type};

impl TyperError {
    pub fn duplicate_type(name: &str, span: Span) -> Self {
        Self::new(
            "T701",
            format!("at {}..{}: duplicate type `{}`", span.start, span.end, name),
            span.start,
            span.end,
        )
    }

    // Phase 17.2 - Generics and traits

    pub fn type_conflicts_with_resource(name: &str, span: Span) -> Self {
        Self::new(
            "T702",
            format!(
                "at {}..{}: type `{}` conflicts with an existing resource name",
                span.start, span.end, name
            ),
            span.start,
            span.end,
        )
    }

    pub fn unknown_type(name: &str, span: Option<Span>) -> Self {
        if let Some(sp) = span {
            Self::new(
                "T210",
                format!("at {}..{}: unknown type `{}`", sp.start, sp.end, name),
                sp.start,
                sp.end,
            )
        } else {
            Self::new("T210", format!("unknown type `{}`", name), 0, 0)
        }
    }

    pub fn unknown_struct_field(struct_name: &str, field: &str, span: Span) -> Self {
        Self::new(
            "T211",
            format!(
                "at {}..{}: unknown field `{}` on struct `{}`",
                span.start, span.end, field, struct_name
            ),
            span.start,
            span.end,
        )
    }

    pub fn missing_struct_field(struct_name: &str, field: &str, span: Span) -> Self {
        Self::new(
            "T212",
            format!(
                "at {}..{}: missing field `{}` in struct `{}` literal",
                span.start, span.end, field, struct_name
            ),
            span.start,
            span.end,
        )
    }

    pub fn duplicate_struct_field(field: &str, span: Span) -> Self {
        Self::new(
            "T213",
            format!(
                "at {}..{}: duplicate struct field `{}`",
                span.start, span.end, field
            ),
            span.start,
            span.end,
        )
    }

    pub fn struct_field_type_mismatch(
        struct_name: &str,
        field: &str,
        expected: Type,
        found: Type,
        span: Span,
    ) -> Self {
        Self::new(
            "T214",
            format!(
                "at {}..{}: struct `{}` field `{}` type mismatch: expected `{}`, found `{}`",
                span.start,
                span.end,
                struct_name,
                field,
                show_ty(expected),
                show_ty(found)
            ),
            span.start,
            span.end,
        )
    }

    pub fn unknown_enum_variant(label: &str, span: Span) -> Self {
        Self::new(
            "T215",
            format!(
                "at {}..{}: unknown enum variant `{}`",
                span.start, span.end, label
            ),
            span.start,
            span.end,
        )
    }

    pub fn enum_variant_arity_mismatch(
        label: &str,
        expected: usize,
        found: usize,
        span: Span,
    ) -> Self {
        Self::new(
            "T216",
            format!(
                "at {}..{}: enum variant `{}` expects {} fields, found {}",
                span.start, span.end, label, expected, found
            ),
            span.start,
            span.end,
        )
    }

    pub fn expected_struct(found: &str, span: Span) -> Self {
        Self::new(
            "T217",
            format!(
                "at {}..{}: expected struct type, found `{}`",
                span.start, span.end, found
            ),
            span.start,
            span.end,
        )
    }

    pub fn resource_field_not_supported(kind: &str, name: &str, span: Span) -> Self {
        Self::new(
            "T218",
            format!(
                "at {}..{}: resources are not allowed in {} `{}` fields yet",
                span.start, span.end, kind, name
            ),
            span.start,
            span.end,
        )
    }

    pub fn duplicate_enum_variant(variant: &str, span: Span) -> Self {
        Self::new(
            "T219",
            format!(
                "at {}..{}: duplicate enum variant `{}`",
                span.start, span.end, variant
            ),
            span.start,
            span.end,
        )
    }

    // Collections typing (Phase 4.6)
}
