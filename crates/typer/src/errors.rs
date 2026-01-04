use crate::check::show_ty;
use clg_ast::{Effect, Span, Type};

#[derive(Debug, Clone)]
pub struct TyperError {
    pub code: &'static str,
    pub message: String,
    pub start: usize,
    pub end: usize,
}

impl TyperError {
    pub fn new(code: &'static str, message: String, start: usize, end: usize) -> Self {
        TyperError {
            code,
            message,
            start,
            end,
        }
    }

    pub fn unknown_function(callee: &str, span: Span) -> Self {
        Self::new(
            "T001",
            format!(
                "at {}..{}: unknown function `{}`",
                span.start, span.end, callee
            ),
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

    pub fn arg_type_mismatch(
        i: usize,
        callee: &str,
        expected: Type,
        found: Type,
        span: Span,
    ) -> Self {
        Self::new(
            "T003",
            format!(
                "at {}..{}: arg {} type mismatch calling `{}`: expected `{}`, found `{}`",
                span.start,
                span.end,
                i,
                callee,
                show_ty(expected),
                show_ty(found)
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
                span.start,
                span.end,
                show_ty(declared),
                show_ty(found)
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
                    sp.start,
                    sp.end,
                    what,
                    show_ty(ty)
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

    pub fn bool_operand(what: &str, ty: Type, span: Option<Span>) -> Self {
        if let Some(sp) = span {
            Self::new(
                "T012",
                format!(
                    "at {}..{}: {} must be Bool, found `{}`",
                    sp.start,
                    sp.end,
                    what,
                    show_ty(ty)
                ),
                sp.start,
                sp.end,
            )
        } else {
            Self::new(
                "T012",
                format!("{} must be Bool, found `{}`", what, show_ty(ty)),
                0,
                0,
            )
        }
    }

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

    pub fn binary_operands_mismatch(op: &str, left: Type, right: Type, span: Span) -> Self {
        Self::new(
            "T013",
            format!(
                "at {}..{}: operands of `{}` must have the same type, found `{}` and `{}`",
                span.start,
                span.end,
                op,
                show_ty(left),
                show_ty(right)
            ),
            span.start,
            span.end,
        )
    }

    pub fn contract_not_bool(kind: &str, ty: Type, span: Span) -> Self {
        Self::new(
            "T014",
            format!(
                "at {}..{}: `{}` must be Bool, found `{}`",
                span.start,
                span.end,
                kind,
                show_ty(ty)
            ),
            span.start,
            span.end,
        )
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

    pub fn duplicate_type(name: &str, span: Span) -> Self {
        Self::new(
            "T701",
            format!("at {}..{}: duplicate type `{}`", span.start, span.end, name),
            span.start,
            span.end,
        )
    }

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
                "at {}..{}: invalid match scrutinee: expected `Option` or `Result`, found `{}`",
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

    // Collections typing (Phase 4.6)
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

    // Phase 6.6 - ADT sugar diagnostics
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

impl std::fmt::Display for TyperError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for TyperError {}

fn render_type(ty: &Type) -> String {
    match ty {
        Type::Int => "Int".to_string(),
        Type::Bool => "Bool".to_string(),
        Type::String => "String".to_string(),
        Type::Resource(name) => name.to_string(),
        Type::Option(inner) => format!("Option<{}>", render_type(inner)),
        Type::Result(ok, err) => format!("Result<{}, {}>", render_type(ok), render_type(err)),
        Type::List(inner) => format!("List<{}>", render_type(inner)),
        Type::Set(inner) => format!("Set<{}>", render_type(inner)),
        Type::Map(key, val) => format!("Map<{}, {}>", render_type(key), render_type(val)),
    }
}
