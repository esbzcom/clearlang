use super::TyperError;
use clg_ast::{Span};

impl TyperError {
    pub fn duplicate_trait(name: &str, span: Span) -> Self {
        Self::new(
            "T230",
            format!("at {}..{}: duplicate trait `{}`", span.start, span.end, name),
            span.start,
            span.end,
        )
    }


    pub fn unknown_trait(name: &str, span: Span) -> Self {
        Self::new(
            "T231",
            format!("at {}..{}: unknown trait `{}`", span.start, span.end, name),
            span.start,
            span.end,
        )
    }


    pub fn duplicate_trait_method(trait_name: &str, method: &str, span: Span) -> Self {
        Self::new(
            "T232",
            format!(
                "at {}..{}: duplicate method `{}` in trait `{}`",
                span.start, span.end, method, trait_name
            ),
            span.start,
            span.end,
        )
    }


    pub fn trait_method_missing(trait_name: &str, method: &str, span: Span) -> Self {
        Self::new(
            "T233",
            format!(
                "at {}..{}: impl for `{}` is missing method `{}`",
                span.start, span.end, trait_name, method
            ),
            span.start,
            span.end,
        )
    }


    pub fn trait_method_extra(trait_name: &str, method: &str, span: Span) -> Self {
        Self::new(
            "T234",
            format!(
                "at {}..{}: impl for `{}` contains extra method `{}`",
                span.start, span.end, trait_name, method
            ),
            span.start,
            span.end,
        )
    }


    pub fn trait_method_signature_mismatch(trait_name: &str, method: &str, span: Span) -> Self {
        Self::new(
            "T235",
            format!(
                "at {}..{}: method `{}` does not match trait `{}` signature",
                span.start, span.end, method, trait_name
            ),
            span.start,
            span.end,
        )
    }


    pub fn overlapping_impl(trait_name: &str, _left: Span, right: Span) -> Self {
        Self::new(
            "T236",
            format!(
                "at {}..{}: overlapping impls for trait `{}`",
                right.start, right.end, trait_name
            ),
            right.start,
            right.end,
        )
    }


    pub fn missing_trait_bound(param: &str, trait_name: &str, span: Span) -> Self {
        Self::new(
            "T237",
            format!(
                "at {}..{}: missing trait `{}` for `{}`",
                span.start, span.end, trait_name, param
            ),
            span.start,
            span.end,
        )
    }


    pub fn ambiguous_impl(
        trait_name: &str,
        param: &str,
        span: Span,
        candidates: &[(String, Span)],
    ) -> Self {
        let mut message = format!(
            "at {}..{}: ambiguous impl for trait `{}` on `{}`",
            span.start, span.end, trait_name, param
        );
        if !candidates.is_empty() {
            let mut parts: Vec<String> = Vec::with_capacity(candidates.len());
            for (ty, sp) in candidates {
                parts.push(format!(
                    "impl for `{}` at {}..{}",
                    ty, sp.start, sp.end
                ));
            }
            message.push_str("; candidates: ");
            message.push_str(&parts.join(", "));
        }
        Self::new("T248", message, span.start, span.end)
    }
}
