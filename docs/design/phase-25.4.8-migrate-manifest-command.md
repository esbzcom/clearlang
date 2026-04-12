# Phase 25.4.8 - `clg pkg migrate-manifest`

## Status
Design lock + implementation record for `25.4.8` in `docs/TODO.md`.

## Goal
Add deterministic migration tooling to bootstrap `clg.project.json` from existing canonical package inputs.

## Command
```text
clg pkg migrate-manifest --root <DIR>
```

## Behavior
1. Reads canonical package metadata from `<DIR>/clg.package-metadata.json`.
2. Uses `<DIR>/clg.lock.json` roots when present to preserve existing dependency intent.
3. If lockfile is absent, derives roots deterministically from canonical metadata (same deterministic resolver contract used by `pkg lock`).
4. Writes `<DIR>/clg.project.json` schema v1 with:
   - project metadata defaults,
   - migrated `dependencies[]`,
   - `release_defaults` placeholders.

## Fail-Closed Migration Conflicts
- Emits `C109` for deterministic migration/coexistence conflicts:
  - target `clg.project.json` already exists,
  - legacy `clg-packages.json` is present,
  - incompatible/ambiguous roots for manifest-v1 projection.

## Notes
- Migration command is pre-GA tooling for Gate E cutover and does not preserve legacy compatibility aliases.
- Post-migration lock regeneration/update remains:
  - `clg pkg lock --generate|--update --root <DIR>`

## Evidence
- command surface:
  - `crates/cli/src/main.rs`
- implementation:
  - `crates/cli/src/commands/pkg/migrate_manifest.rs`
- integration tests:
  - `crates/cli/tests/cli_it/pkg_lock/core.rs`
