# Phase 25.1.8 - Z3 VC Outcome Integration

## Status
Implementation lock for `25.1.8` in `docs/TODO.md`.

## Goal
Integrate theorem-prover execution (Z3 baseline) into VC processing and record deterministic per-VC outcomes:
- `proved`
- `failed`
- `unknown`
- `timeout`

## Delivered
1. Added solver execution path in build pipeline:
   - `crates/cli/src/commands/build/solver.rs`
   - wired into `build` flow before proof status/release gating.
2. Z3 profile lock is used for deterministic options/timeouts:
   - sourced via `docs/design/phase-25.1.4-solver-profile.lock.json`.
3. Outcome mapping:
   - `unsat -> proved`
   - `sat -> failed`
   - `unknown + reason timeout -> timeout`
   - `unknown + non-timeout reason -> unknown`
4. Added deterministic total/per-VC timeout handling.
5. Added fallback policy:
   - if solver binary is unavailable, VC statuses remain existing `generated` (current fail-open compatibility path before strict cutover).

## Activation
Solver execution is opt-in for this slice:
- set `CLG_SOLVER_BIN` to a Z3-compatible executable path.

## Tests
Added:
- `crates/cli/tests/solver_outcomes.rs`
  - validates mapping for `proved|failed|unknown|timeout`
  - validates `--emit-proof` summary counts
  - validates unavailable-solver fallback to `generated`

## CI/Gate Wiring
Added proof-regression command:
- `cargo test -p clg-cli --test solver_outcomes`

Updated:
- `.github/workflows/ci.yml`
- `crates/cli/tests/ci_workflow.rs`
- `docs/evidence/milestone_3-proof-gate.lock.json`
