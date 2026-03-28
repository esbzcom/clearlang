# Phase 25.0.13 - Crypto Proof Boundary Release Gate

## Status
Design lock + implementation note for `25.0.13` in `docs/TODO.md`.

## Goal
Enforce explicit crypto-boundary fail-closed behavior in production release builds so unresolved crypto semantics cannot pass release-grade/theorem-grade policy.

## Policy
1. In strict production builds (`--compiler-mode strict --release-profile production`), any VC assumption boundary with id `crypto.uninterpreted` is an immediate release-policy failure.
2. Failure is reported with deterministic build diagnostic `C123`.
3. This gate is explicit and runs before generic strict language profile rejection (`C033`) to preserve stable crypto-specific release diagnostics.

## Enforcement Wiring
- Build-side release gate:
  - `crates/cli/src/commands/build/run.rs`
  - `crates/cli/src/commands/build/strict.rs`
- CLI integration regression:
  - `crates/cli/tests/cli_it/diagnostics/type_and_mode_basics.rs`
- CI proof regression wiring:
  - `.github/workflows/ci.yml`
  - `crates/cli/tests/ci_workflow.rs`
- Milestone lock + gate assertions:
  - `docs/evidence/milestone_3-proof-gate.lock.json`
  - `crates/cli/tests/milestone3_release_gate.rs`

## Exit Criteria for 25.0.13
1. Production builds fail with `C123` when `crypto.uninterpreted` is present.
2. CI includes deterministic regression test for `C123`.
3. Milestone lock records crypto release-boundary policy metadata.
4. TODO marks `25.0.13` complete with linked implementation and tests.

## References
- `docs/TODO.md`
- `docs/evidence/milestone_3-proof-gate.lock.json`
- `docs/release-process.md`
- `crates/cli/src/commands/build/strict.rs`
- `crates/cli/src/commands/build/run.rs`
