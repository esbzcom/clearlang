use super::TyperError;
use clg_ast::{Effect, Span};

impl TyperError {
    pub fn effect_required(callee: &str, effect: &str, span: Span) -> Self {
        Self::new(
            "T401",
            format!(
                "at {}..{}: calling `{}` requires `{}` effect",
                span.start, span.end, callee, effect
            ),
            span.start,
            span.end,
        )
    }

    pub fn mut_guard_missing(callee: &str, guard: &str, arg: &str, span: Span) -> Self {
        Self::new(
            "T402",
            format!(
                "at {}..{}: calling `{}` requires `{}` guard for `{}`",
                span.start, span.end, callee, guard, arg
            ),
            span.start,
            span.end,
        )
    }

    pub fn mut_guard_requires_variable(callee: &str, guard: &str, span: Span) -> Self {
        Self::new(
            "T403",
            format!(
                "at {}..{}: calling `{}` requires its first argument to be a variable guarded by `{}`",
                span.start, span.end, callee, guard
            ),
            span.start,
            span.end,
        )
    }

    pub fn effect_not_supported(effect: Effect) -> Self {
        let eff_str = match effect {
            Effect::Mut => "mut",
            Effect::Io => "io",
            _ => "",
        };
        Self::new(
            "T009",
            format!("effect `{}` not supported yet; use `pure` or omit", eff_str),
            0,
            0,
        )
    }
}
