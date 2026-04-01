# Phase 25.2.18 - Solver Backend Determinism Parity Gate

## Status
Design lock + implementation record for `25.2.18` in `docs/TODO.md`.

## Goal
Enforce deterministic parity between solver backends so migration to `rust-z3-lib` cannot change proof outcomes for identical strict inputs.

## Parity Gate Contract
Feature-gated integration test:
- `cargo test -p clg-cli --features rust-z3-lib --test solver_backend_parity`

Checks on identical input module:
1. `external-z3-cli` backend run emits VC status vector.
2. `rust-z3-lib` backend run (cutover enabled) emits VC status vector.
3. Status vectors must match exactly.
4. Proof artifact SHA-256 hash must match exactly.

Any mismatch fails closed.

## CI Wiring
- Added to `Proof regression gates` in `.github/workflows/ci.yml`.
- Added CI workflow test assertion in `crates/cli/tests/ci_workflow.rs`.

## Notes
- Gate is intentionally feature-gated and runs with `--features rust-z3-lib`.
- Build environments running this gate must satisfy `rust-z3-lib` build prerequisites (including `cmake` for static-link path).

## References
- `crates/cli/tests/solver_backend_parity.rs`
- `.github/workflows/ci.yml`
- `crates/cli/tests/ci_workflow.rs`
- `docs/release-process.md`
- `README.md`
