# ClearLang Development Plan

## Current Focus - Phase 19: High-assurance ergonomics
- 18.0.0 open questions are now decision-locked (see `docs/design/phase-18.0-open-questions.md`).
- Security gate is complete (`18.1` + `18.3`).
- Reliability gate is complete (`18.2`).
- Ecosystem gate is complete (`18.4`).
- Runtime-ops gate is complete (`18.0.4`) via closure-env host runbook + evidence index.
- Language ergonomics execution is complete (`18.0.5`).
- Proof-model execution is complete (`18.0.6`) including strict-mode default rollout (`18.0.6.4`).
- Next active phase: `19.0` target/guardrails for high-assurance ergonomics.

### Suggested Sequence
1) Execute `19.0.3`-`19.0.4` guardrails before opening 19.1 tier/runtime policy work.

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
- **Phase 18 proof-model assumption boundaries (`18.0.6.1`)**: completed with explicit VC/proof artifact `assumptions.items` for unsigned/bitwise/crypto modeling limits.
- **Phase 18 proof-regression CI suites (`18.0.6.2`)**: completed with explicit CI proof gates (`vc_snapshots`, VC assumption-boundary artifact checks, typer proof-model checks) and snapshot fixture expansion.
- **Phase 18 proof-coverage matrix (`18.0.6.3`)**: completed with published feature/intrinsic matrix (`docs/proofs/proof-coverage-matrix.{md,json}`) and CI validation (`proof_coverage_matrix` test).
- **Phase 18 strict-mode defaults (`18.0.6.4`)**: completed by defaulting `clg build --emit-vcs` to strict assumption-boundary validation (`--proof-strict=true`) with deterministic build diagnostics (`C014`).
- **Phase 19 success-target lock (`19.0.1`)**: completed with bounded theorem-prover-grade assurance scope and measurable criteria (`docs/design/phase-19.0.1-success-target.md`).
- **Phase 19 design-principles gate lock (`19.0.2`)**: completed with explicit policy + regression test for all `docs/design/phase-19*.md` slices (`docs/design/phase-19.0.2-design-principles-gate.md`, `crates/cli/tests/phase19_design_principles.rs`).

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
- **Phase 19 - High-assurance ergonomics**: trust tiers, strict verify profile, and explainable assurance artifacts.

### Notes
- `docs/TODO.md` is the canonical checklist; keep this file high-level.
- Historical detail from earlier phases is archived in `docs/rollout/codex-session-history.md`.
- Attestation production checklist + migration path: `docs/rollout/attestation-production.md`.
