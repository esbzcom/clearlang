use crate::check::show_ty;
use lumi_ast::{Effect, Span, Type};

#[derive(Debug, Clone)]
pub struct TyperError {
    pub code: &'static str,
    pub message: String,
    pub start: usize,
    pub end: usize,
}

impl TyperError {
    pub fn new(code: &'static str, message: String, start: usize, end: usize) -> Self {
        TyperError { code, message, start, end }
    }

    pub fn unknown_function(callee: &str, span: Span) -> Self {
        Self::new(
            "T001",
            format!("at {}..{}: unknown function `{}`", span.start, span.end, callee),
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

    pub fn arg_type_mismatch(i: usize, callee: &str, expected: Type, found: Type, span: Span) -> Self {
        Self::new(
            "T003",
            format!(
                "at {}..{}: arg {} type mismatch calling `{}`: expected `{}`, found `{}`",
                span.start, span.end, i, callee, show_ty(expected), show_ty(found)
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
                span.start, span.end, show_ty(declared), show_ty(found)
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
                    sp.start, sp.end, what, show_ty(ty)
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

    pub fn effect_not_supported(effect: Effect) -> Self {
        let eff_str = match effect { Effect::Mut => "mut", Effect::Io => "io", _ => "" };
        Self::new(
            "T009",
            format!("effect `{}` not supported yet; use `pure` or omit", eff_str),
            0,
            0,
        )
    }

    pub fn collections_unavailable(callee: &str, span: Span) -> Self {
        Self::new(
            "T101",
            format!(
                "at {}..{}: collections require generics/ADTs; `{}` is planned in later Phase 4.3 slices",
                span.start, span.end, callee
            ),
            span.start,
            span.end,
        )
    }
}

impl std::fmt::Display for TyperError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for TyperError {}
