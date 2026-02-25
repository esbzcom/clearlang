use clg_typer::{AssumptionCategory, VerificationCondition};

use super::CompilerMode;

const ASSUMPTION_UNSIGNED_ID: &str = "unsigned.int_model";
const ASSUMPTION_BITWISE_ID: &str = "bitwise.uninterpreted";
const ASSUMPTION_CRYPTO_ID: &str = "crypto.uninterpreted";
const ASSUMPTION_PRIMITIVE_ID: &str = "primitive.unproved";
const ASSUMPTION_EXTERNAL_ID: &str = "external.dependency";

pub(super) fn proof_strict_for_mode(
    mode: CompilerMode,
    value: Option<bool>,
) -> Result<bool, &'static str> {
    match mode {
        CompilerMode::Permissive => Ok(value.unwrap_or(false)),
        CompilerMode::Standard => Ok(value.unwrap_or(true)),
        CompilerMode::Strict => match value {
            Some(false) => {
                Err("`--compiler-mode strict` cannot be combined with `--proof-strict=false`")
            }
            _ => Ok(true),
        },
    }
}

pub(super) fn strict_proof_violation(vcs: &[VerificationCondition]) -> Option<String> {
    for vc in vcs {
        let mut seen_ids = std::collections::BTreeSet::new();
        for assumption in &vc.assumptions {
            if !seen_ids.insert(assumption.id) {
                return Some(format!(
                    "strict proof mode: VC `{}` in function `{}` has duplicate assumption boundary `{}`",
                    vc.vc_id, vc.function, assumption.id
                ));
            }
            let Some(expected_category) = category_for_assumption_id(assumption.id) else {
                return Some(format!(
                    "strict proof mode: VC `{}` in function `{}` has unknown assumption boundary `{}`",
                    vc.vc_id, vc.function, assumption.id
                ));
            };
            if assumption.category != expected_category {
                return Some(format!(
                    "strict proof mode: VC `{}` in function `{}` has mismatched category `{}` for assumption `{}`",
                    vc.vc_id,
                    vc.function,
                    assumption.category.as_str(),
                    assumption.id
                ));
            }
            if assumption.status != "assumed" {
                return Some(format!(
                    "strict proof mode: VC `{}` in function `{}` has invalid status `{}` for assumption `{}`; expected `assumed`",
                    vc.vc_id, vc.function, assumption.status, assumption.id
                ));
            }
        }
    }
    None
}

pub(super) fn strict_l3_claim_violation(vcs: &[VerificationCondition]) -> Option<String> {
    for vc in vcs {
        for assumption in &vc.assumptions {
            if assumption.message.trim().is_empty() {
                return Some(format!(
                    "strict compiler mode L3 claim blocked: VC `{}` in function `{}` has unlabeled assumption boundary `{}` (missing message)",
                    vc.vc_id, vc.function, assumption.id
                ));
            }
            if assumption.symbols.is_empty() {
                return Some(format!(
                    "strict compiler mode L3 claim blocked: VC `{}` in function `{}` has unlabeled assumption boundary `{}` (missing symbols)",
                    vc.vc_id, vc.function, assumption.id
                ));
            }
            if assumption
                .symbols
                .iter()
                .any(|symbol| symbol.trim().is_empty())
            {
                return Some(format!(
                    "strict compiler mode L3 claim blocked: VC `{}` in function `{}` has unlabeled assumption boundary `{}` (empty symbol label)",
                    vc.vc_id, vc.function, assumption.id
                ));
            }
        }
    }
    None
}

pub(super) fn strict_language_profile_violation(vcs: &[VerificationCondition]) -> Option<String> {
    for vc in vcs {
        for assumption in &vc.assumptions {
            let symbols = if assumption.symbols.is_empty() {
                "<none>".to_string()
            } else {
                assumption.symbols.join(", ")
            };
            return Some(format!(
                "strict language profile rejected VC `{}` in function `{}`: assumption boundary `{}` (category `{}`) is a deferred/unchecked surface [{}]",
                vc.vc_id,
                vc.function,
                assumption.id,
                assumption.category.as_str(),
                symbols
            ));
        }
    }
    None
}

fn category_for_assumption_id(id: &str) -> Option<AssumptionCategory> {
    match id {
        ASSUMPTION_UNSIGNED_ID => Some(AssumptionCategory::Unsigned),
        ASSUMPTION_BITWISE_ID => Some(AssumptionCategory::Bitwise),
        ASSUMPTION_CRYPTO_ID => Some(AssumptionCategory::Crypto),
        ASSUMPTION_PRIMITIVE_ID => Some(AssumptionCategory::Primitive),
        ASSUMPTION_EXTERNAL_ID => Some(AssumptionCategory::External),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clg_ast::Span;
    use clg_typer::{AssumptionBoundary, ContractExpr};

    fn sample_vc() -> VerificationCondition {
        VerificationCondition {
            function: "f".to_string(),
            vc_id: "vc:0".to_string(),
            pre: ContractExpr {
                ast: "true".to_string(),
                smt2: "true".to_string(),
                span: Some(Span { start: 0, end: 0 }),
            },
            post: ContractExpr {
                ast: "true".to_string(),
                smt2: "true".to_string(),
                span: Some(Span { start: 0, end: 0 }),
            },
            vc_smt2: "(=> true true)".to_string(),
            status: "generated",
            refinements: Vec::new(),
            assumptions: Vec::new(),
        }
    }

    fn assumption(
        id: &'static str,
        category: AssumptionCategory,
        symbols: &[&str],
    ) -> AssumptionBoundary {
        AssumptionBoundary {
            id,
            category,
            status: "assumed",
            message: "m",
            symbols: symbols.iter().map(|symbol| (*symbol).to_string()).collect(),
        }
    }

    #[test]
    fn strict_mode_accepts_vc_without_assumptions() {
        let mut vc = sample_vc();
        vc.vc_smt2 = "(=> true true)".to_string();
        assert!(strict_proof_violation(&[vc]).is_none());
    }

    #[test]
    fn strict_mode_accepts_valid_assumptions() {
        let mut vc = sample_vc();
        vc.vc_smt2 = "(=> true true)".to_string();
        vc.assumptions = vec![
            assumption(
                ASSUMPTION_UNSIGNED_ID,
                AssumptionCategory::Unsigned,
                &["U64"],
            ),
            assumption(ASSUMPTION_BITWISE_ID, AssumptionCategory::Bitwise, &["&"]),
            assumption(
                ASSUMPTION_CRYPTO_ID,
                AssumptionCategory::Crypto,
                &["std::bytes::eq_ct"],
            ),
            assumption(
                ASSUMPTION_PRIMITIVE_ID,
                AssumptionCategory::Primitive,
                &["std::bytes::eq_ct"],
            ),
            assumption(
                ASSUMPTION_EXTERNAL_ID,
                AssumptionCategory::External,
                &["extpkg::math::add2"],
            ),
        ];
        assert!(
            strict_proof_violation(&[vc]).is_none(),
            "expected strict checks to pass"
        );
    }

    #[test]
    fn strict_mode_rejects_invalid_assumption_status() {
        let mut vc = sample_vc();
        vc.assumptions = vec![AssumptionBoundary {
            id: ASSUMPTION_UNSIGNED_ID,
            category: AssumptionCategory::Unsigned,
            status: "proved",
            message: "m",
            symbols: vec!["U64".to_string()],
        }];
        let msg = strict_proof_violation(&[vc]).expect("expected strict violation");
        assert!(msg.contains("invalid status"));
    }

    #[test]
    fn strict_mode_rejects_unknown_assumption_boundary() {
        let mut vc = sample_vc();
        vc.assumptions = vec![assumption(
            "unknown.assumption",
            AssumptionCategory::Bitwise,
            &["mystery"],
        )];
        let msg = strict_proof_violation(&[vc]).expect("expected strict violation");
        assert!(msg.contains("unknown assumption boundary"));
    }

    #[test]
    fn strict_mode_rejects_mismatched_assumption_category() {
        let mut vc = sample_vc();
        vc.assumptions = vec![assumption(
            ASSUMPTION_BITWISE_ID,
            AssumptionCategory::Crypto,
            &["&"],
        )];
        let msg = strict_proof_violation(&[vc]).expect("expected strict violation");
        assert!(msg.contains("mismatched category"));
    }

    #[test]
    fn strict_l3_claim_accepts_labeled_assumptions() {
        let mut vc = sample_vc();
        vc.assumptions = vec![assumption(
            ASSUMPTION_EXTERNAL_ID,
            AssumptionCategory::External,
            &["extpkg::math::add2"],
        )];
        assert!(strict_l3_claim_violation(&[vc]).is_none());
    }

    #[test]
    fn strict_l3_claim_rejects_assumption_without_symbols() {
        let mut vc = sample_vc();
        vc.assumptions = vec![AssumptionBoundary {
            id: ASSUMPTION_EXTERNAL_ID,
            category: AssumptionCategory::External,
            status: "assumed",
            message: "external dependency assumed",
            symbols: Vec::new(),
        }];
        let msg =
            strict_l3_claim_violation(&[vc]).expect("expected strict L3 claim violation message");
        assert!(msg.contains("missing symbols"));
    }

    #[test]
    fn strict_l3_claim_rejects_assumption_without_message() {
        let mut vc = sample_vc();
        vc.assumptions = vec![AssumptionBoundary {
            id: ASSUMPTION_PRIMITIVE_ID,
            category: AssumptionCategory::Primitive,
            status: "assumed",
            message: "",
            symbols: vec!["std::bytes::eq_ct".to_string()],
        }];
        let msg =
            strict_l3_claim_violation(&[vc]).expect("expected strict L3 claim violation message");
        assert!(msg.contains("missing message"));
    }

    #[test]
    fn strict_language_profile_accepts_vc_without_assumptions() {
        let vc = sample_vc();
        assert!(strict_language_profile_violation(&[vc]).is_none());
    }

    #[test]
    fn strict_language_profile_rejects_external_dependency_assumption() {
        let mut vc = sample_vc();
        vc.assumptions = vec![assumption(
            ASSUMPTION_EXTERNAL_ID,
            AssumptionCategory::External,
            &["extpkg::math::add2"],
        )];
        let msg = strict_language_profile_violation(&[vc])
            .expect("expected strict language profile violation");
        assert!(msg.contains("external.dependency"));
    }

    #[test]
    fn strict_language_profile_rejects_bitwise_assumption() {
        let mut vc = sample_vc();
        vc.assumptions = vec![assumption(
            ASSUMPTION_BITWISE_ID,
            AssumptionCategory::Bitwise,
            &["std::u64::rotl"],
        )];
        let msg = strict_language_profile_violation(&[vc])
            .expect("expected strict language profile violation");
        assert!(msg.contains("bitwise.uninterpreted"));
    }

    #[test]
    fn compiler_mode_permissive_defaults_proof_strict_false() {
        assert_eq!(
            proof_strict_for_mode(CompilerMode::Permissive, None).expect("mode"),
            false
        );
    }

    #[test]
    fn compiler_mode_standard_defaults_proof_strict_true() {
        assert_eq!(
            proof_strict_for_mode(CompilerMode::Standard, None).expect("mode"),
            true
        );
    }

    #[test]
    fn compiler_mode_strict_forces_proof_strict_true() {
        assert_eq!(
            proof_strict_for_mode(CompilerMode::Strict, None).expect("mode"),
            true
        );
        assert_eq!(
            proof_strict_for_mode(CompilerMode::Strict, Some(true)).expect("mode"),
            true
        );
    }

    #[test]
    fn compiler_mode_strict_rejects_proof_strict_false_override() {
        assert!(
            proof_strict_for_mode(CompilerMode::Strict, Some(false)).is_err(),
            "strict mode should reject false override"
        );
    }
}
