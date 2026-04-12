# Phase 25.4.6 - Manifest/Lock Migration and Coexistence Policy

## Status
Design lock + implementation record for `25.4.6` in `docs/TODO.md`.

## Goal
Define deterministic coexistence behavior while moving package workflows from canonical metadata-rooted flows to Gate E manifest/lock flows:
- user-authored: `clg.project.json`
- tool-owned: `clg.lock.json`
- catalog input: `clg.package-metadata.json` (package candidates only)

## Policy
1. Root/dependency intent source-of-truth is `clg.project.json` when schema v1 manifest is present.
2. `clg.package-metadata.json` remains candidate catalog input for solver selection.
3. Legacy `clg-packages.json` is not allowed in `pkg lock`; coexistence with canonical Gate E inputs fails closed.
4. During `clg pkg lock --update`, if manifest v1 exists, existing lockfile roots must match manifest-derived roots exactly.
5. On mismatch, the operator must regenerate/align lockfile roots from manifest intent before update proceeds.

## Deterministic Conflict Diagnostics
- `C109` is emitted for migration/coexistence conflicts:
  - legacy metadata model file present (`clg-packages.json`)
  - manifest-root vs existing-lock-root mismatch on update

Diagnostics are deterministic:
- stable code (`C109`)
- stable stage (`build`)
- deterministic textual conflict context (expected/found root signatures)

## Migration Guidance
1. Remove legacy `clg-packages.json`.
2. Keep package catalog in canonical `clg.package-metadata.json`.
3. Define root intent in `clg.project.json` (`project.name`, `dependencies[]`).
4. Regenerate lock:
   - `clg pkg lock --generate --root <DIR>`
5. For updates:
   - ensure lock roots match manifest roots, then run:
   - `clg pkg lock --update --root <DIR>`

## Evidence
- conflict enforcement (`C109`) in lock command flow:
  - `crates/cli/src/commands/pkg/lock_command.rs`
- integration tests:
  - `pkg_lock_generate_reports_c109_when_legacy_metadata_model_is_present`
  - `pkg_lock_update_reports_c109_when_manifest_roots_conflict_with_existing_lock`
  - `crates/cli/tests/cli_it/pkg_lock/core.rs`
- diagnostics contract update:
  - `docs/diagnostics.md`
