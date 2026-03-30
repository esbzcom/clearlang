# Phase 25.1.13: CI Replay Gates for Release Target Platforms

## Goal
Close TODO item `25.1.13` by enforcing deterministic solver replay outcomes in CI for the current release target platform scope.

## Decision
- Milestone 3 release-target platform scope is **Windows-only** for now.
- Multi-platform parity compare (Linux/macOS) is deferred until:
  - self-contained solver shipping is complete (`25.1.14`),
  - support matrix policy is locked (`25.1.15`).

## Determinism Contract for This Phase
- Replay determinism on identical inputs is required and enforced by CI:
  - per-VC solver statuses must be stable (`proved|failed|unknown|timeout`),
  - emitted proof artifacts must be stable for identical runs.
- Release-train gating for `milestone_3` tags must wait for:
  - core checks,
  - Windows proof-parity replay job.

## Implementation
- CI workflow now runs `milestone3-proof-parity` on `windows-latest` and uploads:
  - `milestone3-proof-parity-windows`
- `milestone3-release-train-gate` depends on:
  - `checks`
  - `milestone3-proof-parity`
- Cross-platform compare job was removed for current scope.

## Lock + Evidence Updates
- `docs/evidence/milestone_3-proof-gate.lock.json`
  - `cross_platform_proof_parity_targets` is now `["windows"]`.
- `docs/evidence/milestone_3-proof-gate.md`
  - release-target parity section updated to Windows-only scope.
- `docs/release-process.md`
  - milestone_3 parity note updated to Windows-only release target gating.

## Validation
- CI structure assertions:
  - `crates/cli/tests/ci_workflow.rs`
  - `crates/cli/tests/milestone3_release_gate.rs`
- Replay outcome/artifact stability tests:
  - `crates/cli/tests/solver_outcomes.rs`
  - `crates/cli/tests/solver_replay_stability.rs`
