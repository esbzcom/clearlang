# Phase 25.4.12 - Release Entrypoint Manifest Cutover

## Status
Design lock + implementation record for `25.4.12` in `docs/TODO.md`.

## Goal
Make `clg.project.json` the source of truth for release entrypoint selection and remove positional entry-file arguments from `clg release`.

## Why
- Simple for users: one manifest defines project identity, dependencies, release defaults, and release entrypoint.
- AI-friendly: fewer CLI permutations and less ambiguity around which file is release-authoritative.
- Provably correct: release orchestration becomes deterministic from root + manifest.
- Pre-GA cutover policy: no compatibility shim is required for legacy positional release entry arguments.

## Contract
`clg release` command shape is now:

```text
clg release --key <FILE> --pubkey <FILE> [--root <DIR>]
```

Entrypoint resolution:
1. Load `<root>/clg.project.json`.
2. Require schema v1 `project.entry`.
3. Resolve `<root>/<project.entry>` as the release entry file.
4. Fail closed (`C130`) if manifest is missing, schema v1 metadata is absent, or the resolved entry file is missing/not a file.

## Schema v1 Update
`project.entry` is a required relative non-traversing path field in `clg.project.json`.

## Tooling Alignment
- `clg strict init` template now includes `project.entry`.
- `clg pkg migrate-manifest` now emits `project.entry` in generated schema v1 manifests.
- Example project manifests (`examples/projects/*/clg.project.json`) include `project.entry`.

## References
- `crates/cli/src/main.rs`
- `crates/cli/src/commands/release.rs`
- `crates/cli/src/commands/release_defaults.rs`
- `crates/cli/src/commands/pkg/migrate_manifest.rs`
- `crates/cli/tests/cli_it/diagnostics/release_command.rs`
- `docs/TODO.md`
