# Phase 25.1.4 - Deterministic Solver Profile Lock

## Status
Design lock for `25.1.4` in `docs/TODO.md`.

## Goal
Treat solver configuration as strict, signed input so proof outcomes are reproducible and auditable.

## Locked Profile Source
- Policy lock file: `docs/design/phase-25.1.4-solver-profile.lock.json`.

## Locked Fields
1. Solver identity:
   - family (`z3` baseline),
   - pinned version string.
2. Deterministic options list (ordered key-value entries).
3. Timeout policy:
   - per-VC timeout,
   - optional total timeout budget.
4. Replay policy marker:
   - exact profile hash must match between build evidence and verify checks.

## Release Binding Rules
1. Production release claims must include `solver_profile_hash`.
2. If profile hash is missing or mismatched at verify-time, fail closed.
3. Changing any locked solver field requires policy lock update and regression evidence refresh.

## Non-Goals
1. This slice does not mandate a bundled solver binary strategy.
2. This slice does not define per-platform distribution mechanics.

## References
- `docs/design/phase-25.1.4-solver-profile.lock.json`
- `crates/cli/tests/phase25_solver_profile_lock.rs`
- `docs/TODO.md` (`25.1.12` implementation task)
