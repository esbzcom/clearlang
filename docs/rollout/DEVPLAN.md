# ClearLang Development Plan

## Current Focus - Phase 19: High-assurance ergonomics
- 18.0.0 open questions are now decision-locked (see `docs/design/phase-18.0-open-questions.md`).
- Security gate is complete (`18.1` + `18.3`).
- Reliability gate is complete (`18.2`).
- Ecosystem gate is complete (`18.4`).
- Runtime-ops gate is complete (`18.0.4`) via closure-env host runbook + evidence index.
- Language ergonomics execution is complete (`18.0.5`).
- Proof-model execution is complete (`18.0.6`) including strict-mode default rollout (`18.0.6.4`).
- Next active phase: `19.4` usability-first proof workflow (`19.4.3` next).

### Suggested Sequence
1) Execute `19.4.3` AI-oriented machine-readable proof context bundle (`VC`, assumptions, model snippet, span map).

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
- **Phase 19 compiler-modes lock (`19.0.3`)**: completed with explicit `clg build --compiler-mode {permissive,standard,strict}` semantics and deterministic diagnostics (`C029`, `C030`) (`docs/design/phase-19.0.3-compiler-modes.md`).
- **Phase 19 non-goal clarity lock (`19.0.4`)**: completed with explicit positioning constraints (no universal proof-power superiority claim) and practical bounded-assurance framing (`docs/design/phase-19.0.4-non-goal-clarity.md`).
- **Phase 19 assurance-tier artifacts (`19.1.1`)**: completed with explicit `L0`-`L3` tier metadata embedded in VC JSON/proof/signing artifacts and schema docs (`docs/design/phase-19.1.1-assurance-tiers.md`, `docs/proofs/vc-schema.md`, `docs/proofs/proof-section.md`).
- **Phase 19 assumed-dependency labeling (`19.1.2`)**: completed by labeling non-proved primitive and external dependencies as explicit `assumed` boundaries in VC/proof artifacts (`primitive.unproved`, `external.dependency`) with regression coverage.
- **Phase 19 strict fail-closed L3 gate (`19.1.3`)**: completed by blocking strict-mode `L3` claims when assumption labels are incomplete (`C031`) while preserving deterministic assumption-shape checks (`C014`).
- **Phase 19 compile-time trust anchors (`19.1.4`)**: completed by adding pinned checker versions (`--lean-checker-version`, `--coq-checker-version`) to signed payloads, compile-time verify policy checks (`--verify-mode compile-time --trust-policy`), and deterministic diagnostics (`C032`, `V004`) while keeping runtime verification kernel-free.
- **Phase 19 unsigned/bitwise downgrade closure (`19.2.1`)**: completed by explicitly downgrading bitwise-sensitive `std::u64` intrinsic semantics (`rotl`/`rotr`/`to_bytes_*`/`from_bytes_*`) under `bitwise.uninterpreted` with deterministic VC assumption symbol labeling.
- **Phase 19 crypto modeling policy (`19.2.2`)**: completed by adding deterministic per-intrinsic crypto assurance metadata (`intrinsic_levels`) under `crypto.uninterpreted` in VC JSON and proof-section assumption outputs.
- **Phase 19 refinement ergonomics closure (`19.2.3`)**: completed by enabling inline param/return refinements and generic refinement alias instantiation with deterministic parser normalization into refined-alias artifacts.
- **Phase 19 strict language profile (`19.3.1`)**: completed by making `--compiler-mode strict` fail closed on assumed proof boundaries (`C033`), thereby blocking deferred/unchecked surfaces (including external unchecked dependencies) by default.
- **Phase 19 verified std/core subset publication (`19.3.2`)**: completed by publishing `verified.std_core.v1` (`docs/proofs/verified-std-core-subset.{md,json}`) and adding regression obligations validation (`crates/cli/tests/verified_std_core_subset.rs`) tied to proved coverage rows.
- **Phase 19 profile regression CI gate (`19.3.3`)**: completed by adding verified-profile fixtures (`clearlang-tests/profile/`), fixture manifest (`docs/proofs/verified-profile-fixtures.json`), regression gate test (`crates/cli/tests/profile_regression_gate.rs`), and explicit CI proof-gate command.
- **Phase 19 VC diagnostic hints (`19.4.1`)**: completed by emitting deterministic VC repair hints (`diagnostics.repair_hints`) for minimal `ensure`/`require`/`invariant`/`variant` suggestions with CLI IT coverage and schema/design docs updates.
- **Phase 19 proof-failure slicing + counterexample envelopes (`19.4.2`)**: completed by emitting `diagnostics.failure_slice` and `diagnostics.counterexample` with direct source-span mapping and deterministic model-binding placeholders in VC JSON outputs.

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
