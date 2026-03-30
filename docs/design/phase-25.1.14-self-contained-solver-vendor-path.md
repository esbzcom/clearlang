# Phase 25.1.14: Self-Contained Solver Vendor Path

## Goal
Close TODO item `25.1.14` by removing the requirement for system-wide solver installation in the default proof flow.

## Scope
- Current release target is Windows-first.
- Solver execution remains deterministic and pinned by `phase-25.1.4-solver-profile.lock.json`.

## Runtime Resolution Order
`clg build` solver execution now resolves binaries in this order:
1. `CLG_SOLVER_BIN` (explicit override, highest priority).
2. `CLG_SOLVER_BUNDLE_ROOT` + platform-relative solver path.
3. Default vendored bundle search:
   - `<repo-or-ancestor>/tools/proof/z3/<platform>/z3(.exe)`
   - `<clg-exe-dir>/solver/<platform>/z3(.exe)` for packaged layouts.

If nothing resolves, behavior remains non-fatal for non-release/dev flows (VCs stay `generated`).

## Rust-Managed Vendor Path
Added xtask command:
- `cargo run -p xtask -- solver-vendor-stage --from <PATH> [--platform windows|linux|macos]`

This stages the solver binary into:
- `tools/proof/z3/<platform>/z3(.exe)`

No system install is required after staging.

## Validation Coverage
- `crates/cli/tests/solver_outcomes.rs`
  - bundled solver root fallback works when `CLG_SOLVER_BIN` is unset.
  - explicit `CLG_SOLVER_BIN` override still takes precedence.
- `xtask/src/main/tests.rs`
  - argument parsing coverage for `solver-vendor-stage`.
