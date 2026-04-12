# Phase 25.4.7 - Pre-GA Fail-Closed Cutover Policy

## Status
Design lock + implementation record for `25.4.7` in `docs/TODO.md`.

## Goal
Lock Gate E cutover behavior before GA:
- remove legacy inputs/flags as soon as manifest-first replacements are ready,
- fail closed on incompatible legacy usage,
- make no backward-compatibility commitment before GA.

## Policy
1. Pre-GA compatibility is migration debt, not a product guarantee.
2. Legacy inputs/flags are removed by policy, not preserved with aliases.
3. Cutover changes must fail closed with deterministic diagnostics (no silent fallback).
4. Source of truth after Gate E cutover:
   - user-authored: `clg.project.json`
   - tool-owned: `clg.lock.json`
5. Legacy coexistence conflicts remain deterministic `C109` failures.

## Cutover Inventory (Gate E)
1. Manifest migration tooling and docs:
   - `25.4.8` (`clg pkg migrate-manifest`)
2. Non-secret release flag retirement:
   - `25.4.9` (`--advisory-as-of`, `--key-id`, `--out-dir`, `--trust-policy`)
3. Tool-owned lock contract enforcement:
   - `25.4.10` (drift gate/manual-edit mismatch fail closed)
4. Trust-policy naming clarification:
   - `25.4.11` (`trust-policy.json` vs `clg.trust-policy.json`)

## Fail-Closed Requirements
1. Removed legacy flags/options must return deterministic usage/contract errors.
2. Deprecated schema/input paths must be rejected with stable diagnostics.
3. Docs/help/CI must reflect only supported post-cutover surfaces.

## Non-Goals
1. No compatibility alias shims for removed pre-GA legacy release surfaces.
2. No deferred "best effort" fallback to legacy paths after cutover.

## References
- `docs/design/phase-25.4.1-project-manifest-v1.md`
- `docs/design/phase-25.4.6-manifest-lock-migration-coexistence-policy.md`
- `docs/design/phase-25.4.9-release-legacy-parameter-retirement.md`
- `docs/TODO.md`
