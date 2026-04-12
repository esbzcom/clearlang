# Phase 25.4.5 - Manifest/Lock Schema, Migration Notes, and Drift Gate

## Status
Design lock + implementation record for `25.4.5` in `docs/TODO.md`.

## Goal
Provide one explicit contract for:
- `clg.project.json` schema expectations,
- `clg.lock.json` schema expectations, and
- CI fail-closed drift detection when manifest and lock roots diverge.

## Schema Contract
`clg.project.json` (schema v1, user-authored):
- `schema_version: 1`
- `project.name` (root identity)
- `dependencies[]` (`name`, `requirement`)
- `release_defaults` (release UX defaults, non-secret metadata only)

`clg.lock.json` (schema v1, tool-generated):
- `schema_version: 1`
- `resolver_version: 1`
- `roots[]` (`name`, `dependencies[]`)
- `packages[]` exact pinned package identities

Consistency rule:
1. Expected lock root is derived from manifest:
   - root `name == project.name`
   - root `dependencies[] == dependencies[]` (normalized/sorted)
2. Lockfile with mismatched roots is drift and must fail CI.

## Migration Notes
1. Schema v0 manifest compatibility remains temporary for release-default bootstrap paths only.
2. Drift checks in this phase require schema v1 manifest + lock roots.
3. Recommended migration:
   - move metadata/dependencies to `clg.project.json` schema v1,
   - regenerate lock with `clg pkg lock --generate|--update`,
   - keep source imports version-free (`25.4.4`).

## CI Drift Gate
Gate command:
- `cargo run -p xtask -- manifest-lock-drift-check`

Release precheck integration:
- `xtask release-precheck` now includes manifest/lock drift validation.

Tracked fixture:
- `docs/fixtures/phase-25.4/manifest-lock-consistency/`

## Evidence
- gate wiring + consistency check logic:
  - `xtask/src/main/core.rs`
  - `xtask/src/main/artifacts_cli_models.rs`
- gate tests:
  - `xtask/src/main/tests.rs`
- fixture pair:
  - `docs/fixtures/phase-25.4/manifest-lock-consistency/clg.project.json`
  - `docs/fixtures/phase-25.4/manifest-lock-consistency/clg.lock.json`
