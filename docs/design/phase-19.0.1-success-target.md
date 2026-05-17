# Phase 19.0.1 - Success Target Lock

## Status
Design lock for `19.0.1` in `docs/TODO.md`.
This defines the Phase 19 success target before guardrail and tier/runtime policy work.

## Goal
Set an explicit, auditable target: theorem-prover-grade assurance for bounded program classes, with lower user complexity than direct theorem-prover workflows.

## Design Principles Check
- Simple for users: proof workflows stay in ClearLang syntax (`require`/`ensure`, refinements, invariants), not external proof scripts.
- AI-friendly: diagnostics/artifacts stay deterministic and machine-readable for generation/repair loops.
- Provably correct: claims are bounded and explicit; assumptions are surfaced, not hidden.
- Crypto-focused: target profile prioritizes deterministic, audit-grade contract/system behavior.

## Bounded Program Classes (In Scope for the Target)
1. Deterministic first-order modules with explicit contracts.
2. Pure and mut flows whose obligations can be expressed with current ClearLang contracts/refinements/loop invariants.
3. Resource/linearity-preserving state transitions under existing effect rules and diagnostics.
4. Verified package profiles that pass strict assurance gates with explicit trust/assumption labeling.

## Out of Scope for the Target
1. Universal theorem proving over arbitrary higher-order math/programs.
2. Claims that every external/runtime dependency is formally proved.
3. Any claim of blanket superiority over Coq/Agda/Lean/F* for general proof power.

## Target Definition
ClearLang succeeds when a production user can obtain audit-grade assurance artifacts for the bounded classes above using standard ClearLang tooling, while every non-proved area is explicitly labeled and policy-gated.

Equivalent operational statement:
- "High assurance with explicit boundaries" rather than "universal proof completeness."

## Success Criteria (Phase-19 Program Target)
1. Assurance transparency:
   - Build outputs classify proved vs assumed boundaries per module/package.
   - No hidden assumptions in strict assurance claims.
2. Deterministic policy gating:
   - Release policies can reject artifacts below required assurance tier.
   - Tier downgrades/regressions are CI-detectable.
3. Practical user complexity:
   - Core proof obligations are expressible in language-level contracts/refinements/invariants.
   - Users are not required to write theorem-prover scripts for standard L0-L2 workflows.
4. Crypto readiness:
   - Determinism, linear/resource safety, and trust-boundary reporting are first-class in assurance artifacts.

## Mapping to Existing Baseline
- Preserve Phase `18.0.0` assurance baseline (`L0`-`L3`) from `docs/design/phase-18.0-open-questions.md`.
- Phase `19.1+` extends this baseline with explicit artifact/reporting and policy enforcement mechanics.

## Non-Goals (19.0.1 Lock)
- No new language syntax.
- No immediate solver/model expansion.
- No trust-anchor integration changes yet (handled in later 19.x tasks).

## Exit Criteria for 19.0.1
1. This success-target lock is published and referenced from roadmap docs.
2. Bounded in-scope/out-of-scope claims are explicit.
3. Measurable success criteria are defined for follow-on 19.x execution.

## References
- `docs/TODO.md`
- `docs/TODO.md`
- `docs/design/phase-18.0-open-questions.md`
- `README.md`
