# Phase 25.2.17 - `rust-z3-lib` Backend Implementation (Feature + Cutover)

## Status
Design lock + implementation record for `25.2.17` in `docs/TODO.md`.

## Goal
Implement the `rust-z3-lib` solver backend while keeping `external-z3-cli` as the temporary fallback path during migration.

## Backend Activation Contract
- Build feature gate:
  - Cargo feature `rust-z3-lib` enables in-process Z3 backend support.
- Backend selection:
  - `CLG_SOLVER_BACKEND=rust-z3-lib` selects migration backend lane.
- Cutover flag:
  - `CLG_SOLVER_RUST_Z3_CUTOVER` controls effective backend when `rust-z3-lib` is selected:
    - truthy (`1|true|on|yes`) -> use in-process `rust-z3-lib`
    - falsy/unset (`0|false|off|no|empty`) -> temporarily fall back to `external-z3-cli`

This keeps migration deterministic while preserving a safe fallback lane until parity/cutover gates are complete.

## `rust-z3-lib` Runtime Contract
- Uses in-process Z3 (`z3` crate) to evaluate per-VC obligations.
- Current implementation uses static-link build path (`z3` crate `static-link-z3` feature), which requires `cmake` in the build environment when compiling with `rust-z3-lib`.
- Enforces pinned solver version alignment against lock profile (`phase-25.1.4-solver-profile.lock.json`).
- Applies deterministic per-VC timeout and solver options from the locked solver profile.
- Outcome mapping remains stable:
  - `Unsat -> proved`
  - `Sat -> failed`
  - `Unknown(timeout) -> timeout`
  - other `Unknown -> unknown`

## Safety
- Unsupported backend values fail closed (from 25.2.16 policy).
- Selecting `rust-z3-lib` without build feature fails closed.
- During migration, fallback behavior is explicit and deterministic via cutover flag.

## References
- `crates/cli/src/commands/build/solver.rs`
- `crates/cli/src/commands/build/tests.rs`
- `crates/cli/tests/solver_outcomes.rs`
- `crates/cli/Cargo.toml`
- `docs/release-process.md`
- `README.md`
