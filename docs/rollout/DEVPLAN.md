# ClearLang Development Plan

## Current Focus - Phase 18: Production hardening + attestation
- 18.0.0 open questions are now decision-locked (see `docs/design/phase-18.0-open-questions.md`).
- Security gate is complete (`18.1` + `18.3`).
- Reliability gate is complete (`18.2`).
- Ecosystem gate is complete (`18.4`).
- Runtime-ops gate is complete (`18.0.4`) via closure-env host runbook + evidence index.
- Language ergonomics execution is complete (`18.0.5`).
- Next active work is proof-model execution: `18.0.6`.

### Suggested Sequence
1) Execute `18.0.5` language ergonomics follow-through from the `18.0.0` decision lock.
2) Execute `18.0.6` proof-model follow-through from the `18.0.0` decision lock.

## Recently Completed
- **Phase 17 - Language gaps + collections**: completed through 17.9 closure/module/resource follow-ups.
- **Phase 18 kickoff decision gate (`18.0.0`)**: resolved and documented in `docs/design/phase-18.0-open-questions.md`.
- **Phase 18 security gate (`18.0.1`)**: completed via `18.1` hardening and `18.3` review/fuzzing.
- **Phase 18 reliability gate (`18.0.2`)**: completed via DA policy + backup/restore drill runbook (`18.2`).
- **Phase 18 runtime-ops gate (`18.0.4`)**: completed via closure-env no-free runbook (`docs/rollout/closure-env-ops-runbook.md`) and evidence index (`docs/evidence/closure-env/README.md`).
- **Phase 18 ergonomics design lock (`18.0.5.1`)**: published in `docs/design/phase-18.0.5-language-ergonomics.md`.
- **Phase 18 ergonomics diagnostics/docs (`18.0.5.2`)**: completed with explicit deferred-form diagnostics (`P013`, `T244`, `T245`, `T246`, `T806`) and updated rationale docs.
- **Phase 18 ergonomics deterministic migration coverage (`18.0.5.3`)**: completed with migration fixtures (`clearlang-tests/migration/`) and stable JSON-code CLI IT assertions.
- **Phase 18 ergonomics SDK usability gates (`18.0.5.4`)**: completed with CLI IT metrics checks (`crates/cli/tests/cli_it/sdk_usability.rs`) and explicit CI gate step (`.github/workflows/ci.yml`).

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
