# ClearLang Development Plan

## Current Focus - Phase 18: Production hardening + attestation
- 18.0.0 open questions are now decision-locked (see `docs/design/phase-18.0-open-questions.md`).
- Next active work is execution against that lock: 18.1, 18.2, 18.3, 18.4.

### Suggested Sequence
1) Complete 18.1 attestation registry hardening (authorization, key rotation, revocation, schema versioning).
2) Complete 18.3 security review and fuzzing for contract/payload validation.
3) Complete 18.2 data availability policy and operational drills.
4) Complete 18.4 compiled module/package import support.
5) Execute 18.0.5 and 18.0.6 implementation follow-through from the 18.0.0 decision lock.

## Recently Completed
- **Phase 17 - Language gaps + collections**: completed through 17.9 closure/module/resource follow-ups.
- **Phase 18 kickoff decision gate (`18.0.0`)**: resolved and documented in `docs/design/phase-18.0-open-questions.md`.

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
