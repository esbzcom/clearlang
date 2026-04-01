# Phase 25.2.19 - Rust Cutover Packaging Gate

## Status
Design lock + implementation record for `25.2.19` in `docs/TODO.md`.

## Goal
Add a release packaging gate that enforces no runtime dependency on `tools/proof/z3` for supported release targets once `rust-z3-lib` cutover is enabled.

## Gate Contract
Feature-gated packaging regression:
- `cargo test -p clg-cli --features rust-z3-lib --test solver_rust_cutover_packaging`

Test conditions:
- `CLG_SOLVER_BACKEND=rust-z3-lib`
- `CLG_SOLVER_RUST_Z3_CUTOVER=1`
- no `CLG_SOLVER_BIN`
- no `CLG_SOLVER_BUNDLE_ROOT`
- isolated temp working directory without `tools/proof/z3`

Required outcomes:
1. Build succeeds.
2. VC statuses must not contain `generated`.
3. Proof summary `generated_count` must be `0`.

This fails closed if runtime behavior still depends on external solver bundle paths.

## CI Wiring
- Added to `Proof regression gates` in `.github/workflows/ci.yml`.
- Added CI workflow assertion in `crates/cli/tests/ci_workflow.rs`.

## References
- `crates/cli/tests/solver_rust_cutover_packaging.rs`
- `.github/workflows/ci.yml`
- `crates/cli/tests/ci_workflow.rs`
- `docs/release-process.md`
- `README.md`
