# Phase 25.2.16 - Solver Backend Abstraction and Selection Policy

## Status
Design lock + implementation record for `25.2.16` in `docs/TODO.md`.

## Goal
Introduce an explicit solver backend abstraction that supports:
- `external-z3-cli`
- `rust-z3-lib`

with deterministic backend selection rules for CLI and IDE automation.

## Deterministic Selection Policy (v1)
- New env var: `CLG_SOLVER_BACKEND`
- Supported values:
  - `external-z3-cli`
  - `rust-z3-lib`
- Selection behavior:
  - unset / empty / whitespace: default to `external-z3-cli`
  - explicit `external-z3-cli`: use external binary resolution path
  - explicit `rust-z3-lib`:
    - allowed only when build enables Cargo feature `rust-z3-lib`
    - otherwise fail closed with deterministic error
  - any other value: fail closed with deterministic error listing supported values

## Migration Staging
- This slice introduces the abstraction boundary and policy only.
- `rust-z3-lib` implementation is intentionally deferred to `25.2.17`.
- Current default runtime behavior remains unchanged (`external-z3-cli`).

## Safety and Determinism
- Backend selection is explicit and machine-readable (single env var).
- Unsupported/disabled backend requests are rejected instead of silently falling back.
- Default path is stable and backward-compatible for current release workflows.

## References
- `crates/cli/src/commands/build/solver.rs`
- `crates/cli/src/commands/build/tests.rs`
- `docs/release-process.md`
- `README.md`
