# Phase 25.0.12 - Strict Release Surface Policy

## Status
Design lock + implementation note for `25.0.12` in `docs/TODO.md`.

## Goal
Enforce release-time surface policy for `release == proved`: production builds must reject non-proved std surfaces before publish/sign workflows.

## Policy
1. `clg build --release-profile production` evaluates used `std::...` bundle symbols.
2. Used symbols must be a subset of `docs/proofs/proof-coverage-matrix.json` entries where `status == "proved"`.
3. Violations fail closed with build diagnostic `C122`.
4. Unproved/non-allowlisted surfaces remain dev-only and cannot pass production release build policy.

## Enforcement Wiring
- Production build gate:
  - `crates/cli/src/commands/build/run.rs`
- Shared matrix allowlist loader + symbol extraction:
  - `crates/cli/src/proofs.rs`
- Diagnostics regression:
  - `crates/cli/tests/cli_it/diagnostics/type_and_mode_basics.rs`
- CI proof regression wiring:
  - `.github/workflows/ci.yml`
  - `crates/cli/tests/ci_workflow.rs`
- Milestone lock metadata:
  - `docs/evidence/milestone_3-proof-gate.lock.json`
  - `crates/cli/tests/milestone3_release_gate.rs`

## Exit Criteria for 25.0.12
1. Production build fails with `C122` on non-proved std surfaces.
2. CI executes deterministic regression coverage for the `C122` gate.
3. Milestone lock records production release-surface policy metadata.
4. TODO marks `25.0.12` complete with linked implementation and tests.

## References
- `docs/TODO.md`
- `docs/proofs/proof-coverage-matrix.json`
- `docs/evidence/milestone_3-proof-gate.lock.json`
- `docs/release-process.md`
- `crates/cli/src/commands/build/run.rs`
