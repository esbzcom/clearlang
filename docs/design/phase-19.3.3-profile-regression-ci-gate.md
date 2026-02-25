# Phase 19.3.3 - Profile regression CI gate

## Status
Design lock for `19.3.3` in `docs/TODO.md`.

## Goal
Add an explicit CI gate that fails when assurance level for existing verified-profile fixtures regresses.

## Design Principles Check
- Simple for users: verified-profile fixtures and expected minimum tiers are declared in one manifest.
- AI-friendly: gate is deterministic and machine-readable (`verified-profile-fixtures.json` + stable test).
- Provably correct: assurance regressions (`L1` -> `L0`) fail CI before release.
- Crypto-focused: strict profile fixture checks ensure deferred/unchecked proof surfaces do not silently re-enter verified paths.

## Scope
1. Publish verified-profile fixture manifest with minimum tier requirements.
2. Add regression test that:
   - builds each fixture in `standard` mode and checks VC `assurance.tier >= min_tier`,
   - rejects unexpected assumptions for tiers at/above `L1`,
   - builds each fixture in `strict` mode to enforce fail-closed profile compatibility.
3. Wire the regression test into CI proof gates.

## Locked Behavior
1. Fixture manifest lives at `docs/proofs/verified-profile-fixtures.json`.
2. Regression gate test lives at `crates/cli/tests/profile_regression_gate.rs`.
3. CI proof-regression step must run:
   - `cargo test -p clg-cli --test profile_regression_gate`

## Non-Goals
1. No new assurance-tier semantics or schema changes.
2. No automatic fixture generation in this slice.
3. No changes to trust-anchor verification workflow.

## Exit Criteria for 19.3.3
1. Existing verified-profile fixtures are enforced by CI with minimum tier checks.
2. CI fails on assurance regressions for those fixtures.
3. TODO/rollout focus advances to `19.4.1`.

## References
- `docs/TODO.md`
- `docs/rollout/DEVPLAN.md`
- `docs/proofs/verified-std-core-subset.json`
- `docs/proofs/verified-profile-fixtures.json`
