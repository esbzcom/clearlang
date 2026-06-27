# Phase 25.1.15: Self-Contained Solver Support Matrix

## Goal
Define and lock the required self-contained solver platform matrix and CI validation strategy.

## Locked Policy
Canonical lock:
- `docs/design/phase-25.1.15-solver-support-matrix.lock.json`

Current matrix:
- release targets:
  - `windows-x64`
  - `linux-x64-glibc-2.39`
  - `macos-x64-15.7.3`

Unsupported target policy:
- release-grade proof execution is fail-closed when bundled solver support is unavailable (`C124` path).

CI validation strategy:
- required job: `milestone3-proof-parity`
- required step: `Solver vendor-path smoke gate`
- required command:
  - `cargo test -p clg-cli --test solver_outcomes bundled_solver_root_is_used_when_solver_env_not_set`

## Follow-up
- keep CI/release-train parity enforcement aligned with this expanded bundle matrix as later
  platform-hardening gates land.
