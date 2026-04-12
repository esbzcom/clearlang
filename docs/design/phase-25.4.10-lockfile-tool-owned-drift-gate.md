# Phase 25.4.10 - Lockfile Tool-Owned Drift Gate

## Status
Design lock + implementation record for `25.4.10` in `docs/TODO.md`.

## Goal
Enforce fail-closed CI drift checks so `clg.lock.json` remains tool-owned and consistent with manifest + resolved-graph evidence.

## Policy
`xtask manifest-lock-drift-check` must fail closed when any of the following are true:
1. `clg.project.json` root/dependency intent diverges from `clg.lock.json` roots.
2. `clg.lock.json` bytes are not canonical tool serializer output.
3. `clg.resolved-graph.json` roots/packages diverge from lock identity.
4. `clg.resolved-graph.sha256` does not match canonical resolved-graph bytes.

## Why This Covers Manual Edits
Manual lock edits are rejected unless they exactly preserve:
- manifest-aligned root intent,
- canonical lock serialization shape,
- resolved-graph identity parity, and
- graph hash sidecar integrity.

This keeps lock ownership on `clg pkg lock`/`clg release` workflows and turns ad-hoc edits into deterministic CI failures.

## Evidence
- Gate implementation:
  - `xtask/src/main/core.rs`
  - `xtask/src/main/artifacts_cli_models.rs`
- Gate tests:
  - `xtask/src/main/tests.rs`
- CI fixture updates:
  - `docs/fixtures/phase-25.4/manifest-lock-consistency/`
- Task tracking:
  - `docs/TODO.md`
