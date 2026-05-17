# Phase 19.0.4 - Non-Goal Clarity Lock

## Status
Design lock for `19.0.4` in `docs/TODO.md`.
This makes the Phase 19 non-goal explicit and durable across docs/planning.

## Goal
Document a clear non-goal:
- ClearLang does not claim universal proof-power superiority over Coq/Agda/Lean/F*.
- ClearLang targets practical production assurance with simpler UX for bounded program classes.

## Design Principles Check
- Simple for users: set realistic expectations and avoid theorem-prover-level onboarding requirements for standard workflows.
- AI-friendly: keep claims precise and machine-checkable, preventing ambiguous marketing language in artifacts/docs.
- Provably correct: scope assurance claims to bounded, explicit surfaces with visible assumptions and policy gates.
- Crypto-focused: prioritize production reliability, determinism, and auditable guarantees over universal proof expressiveness claims.

## Non-Goal Statement (Normative)
1. ClearLang is not positioned as a universal replacement for Coq/Agda/Lean/F* proof power.
2. ClearLang is positioned as an easier, production-oriented assurance system for bounded classes of programs and contracts.

## Scope
1. Planning/docs clarity only for this slice.
2. Preserve this statement in Phase 19 roadmap/design artifacts.

## Required Wording Constraints
1. Allowed framing:
   - "practical production assurance"
   - "bounded program classes"
   - "simpler UX"
2. Disallowed framing:
   - absolute "better than theorem provers in all cases"
   - universal proof-power superiority claims

## Evidence of Adoption
1. `docs/TODO.md` marks `19.0.4` complete with this lock reference.
2. `docs/TODO.md` records `19.0.4` completion and moves execution focus to `19.1`.

## Non-Goals (for this slice)
1. No language/runtime/solver changes.
2. No alteration of assurance-tier semantics (`19.1` owns that work).
3. No trust-anchor integration changes.

## Exit Criteria for 19.0.4
1. Explicit non-goal statement is published and referenced in roadmap docs.
2. Phase `19.0` guardrails are fully closed.
3. Next active execution sequence advances to `19.1`.

## References
- `docs/TODO.md`
- `docs/TODO.md`
- `docs/design/phase-19.0.1-success-target.md`
- `docs/design/phase-18.0-open-questions.md`
