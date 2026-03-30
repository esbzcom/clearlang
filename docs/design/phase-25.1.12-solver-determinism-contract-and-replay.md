# Phase 25.1.12 - Solver Determinism Contract and Replay

## Status
Implementation lock for `25.1.12` in `docs/TODO.md`.

## Goal
Harden deterministic solver execution by pinning solver identity/profile inputs and adding replay-stability gates.

## Delivered
1. Enforced solver identity pin when solver execution is enabled:
   - configured solver binary must report pinned `solver_version` from
     `docs/design/phase-25.1.4-solver-profile.lock.json`.
   - enforced in `crates/cli/src/commands/build/solver.rs`.
2. Kept deterministic profile inputs locked:
   - `solver_family`, `solver_version`, `options[]`, and `timeouts_ms.{per_vc,total}` loaded from lock profile.
3. Added solver-enabled replay stability regression:
   - `crates/cli/tests/solver_replay_stability.rs`
   - runs identical builds twice with solver enabled and checks:
     - VC JSON equality
     - proof artifact byte equality
4. Added CI proof-gate wiring for replay test:
   - `cargo test -p clg-cli --test solver_replay_stability`

## References
- `crates/cli/src/commands/build/solver.rs`
- `crates/cli/tests/solver_outcomes.rs`
- `crates/cli/tests/solver_replay_stability.rs`
- `.github/workflows/ci.yml`
- `crates/cli/tests/ci_workflow.rs`
- `docs/evidence/milestone_3-proof-gate.lock.json`
