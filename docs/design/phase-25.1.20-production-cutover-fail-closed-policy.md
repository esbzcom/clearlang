# Phase 25.1.20: Production Cutover Fail-Closed Policy

## Goal
Define and lock strict production cutover from placeholder solver states to release-grade fail-closed behavior.

## Policy
- `--release-profile production` requires:
  - `--compiler-mode strict`
  - theorem-grade status `proved_all`
- Placeholder/non-proved VC status vectors are blocked for release:
  - `generated`, `failed`, `unknown`, `timeout`
- Enforced by `C121`.

## Lock
- `docs/design/phase-25.1.20-production-cutover-fail-closed.lock.json`

## CI validation
- Production cutover diagnostic test (`C121`) must be in CI proof regression gates.
- Missing-solver generated-status path remains covered to ensure fail-closed cutover semantics are preserved.
