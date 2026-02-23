# ClearLang Development Plan

## Current Focus - Phase 18: Production hardening + attestation
- 18.0.0 open questions are now decision-locked (see `docs/design/phase-18.0-open-questions.md`).
- Security gate is complete (`18.1` + `18.3`).
- Reliability gate is complete (`18.2`).
- Ecosystem gate is complete (`18.4`).
- Next active work is runtime-ops execution: `18.0.4` (closure-env no-free host runbook).

### Suggested Sequence
1) Publish `18.0.4` host runbook for closure-env no-free policy (worker recycle, memory budgets, monitoring alerts).
2) Execute `18.0.5` language ergonomics follow-through from the `18.0.0` decision lock.
3) Execute `18.0.6` proof-model follow-through from the `18.0.0` decision lock.

## Recently Completed
- **Phase 17 - Language gaps + collections**: completed through 17.9 closure/module/resource follow-ups.
- **Phase 18 kickoff decision gate (`18.0.0`)**: resolved and documented in `docs/design/phase-18.0-open-questions.md`.
- **Phase 18 security gate (`18.0.1`)**: completed via `18.1` hardening and `18.3` review/fuzzing.
- **Phase 18 reliability gate (`18.0.2`)**: completed via DA policy + backup/restore drill runbook (`18.2`).

## Historical Highlights
- **Phase 11 - Proof-carrying Wasm verification**: `clg verify` CLI, proof-section hashing/signing, diagnostics, fixtures, and regression tests.
- **Phases 12-15 (per `docs/TODO.md`)**: safety/tooling hardening, DX improvements, runtime decoupling, and unsigned/bitwise expansion.

## Snapshot of Earlier Milestones
- Phase 10: refinements (VC/SMT, fixtures, diagnostics).
- Phase 9: totality & loops.
- Phase 8: resource/linear types.
- Phase 7: Option/Result lowering and proof layout.
- Phase 6: contract syntax + VC generation.
- Phase 1-5: parser/typer/IR + Wasm pipeline foundations.

## Upcoming Phases (High-Level)
- **Phase 18 - Production hardening + attestation**: security/reliability gates, package imports, and proof-model execution follow-through.
- **Phase 19 - High-assurance ergonomics**: trust tiers, strict verify profile, and explainable assurance artifacts.

### Notes
- `docs/TODO.md` is the canonical checklist; keep this file high-level.
- Historical detail from earlier phases is archived in `docs/rollout/codex-session-history.md`.
- Attestation production checklist + migration path: `docs/rollout/attestation-production.md`.
