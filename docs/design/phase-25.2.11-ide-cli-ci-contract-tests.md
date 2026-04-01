# Phase 25.2.11 - IDE CLI CI Contract Tests

## Status
Design lock + implementation record for `25.2.11` in `docs/TODO.md`.

## Goal
Pin IDE/plugin-facing CLI behavior with dedicated CI contract tests so regressions are caught before release tags.

## Contract Coverage
Dedicated test target: `crates/cli/tests/ide_cli_contract.rs`

Covered behavior:
- `check` with `--non-interactive --json-errors`:
  - deterministic failure exit code `1`
  - JSON error payload on `stdout`
  - clean `stderr` when only `--json-errors` is enabled
- `check` with `--non-interactive --json-events`:
  - NDJSON events on `stderr`
  - `schema_version: 1` and `command: check`
- `release` failure with `--non-interactive --json-errors --json-events`:
  - deterministic failure exit code `1`
  - JSON error payload on `stdout`
  - NDJSON events on `stderr` with `schema_version: 1` and `release_lock` stage start
- usage-contract failures remain exit code `2` (`check`/`release` argument errors).

## CI Wiring
`.github/workflows/ci.yml` includes explicit step:
- `IDE CLI contract gates`
- command: `cargo test -p clg-cli --test ide_cli_contract`

This keeps IDE-facing behavior visible and intentionally pinned in the main CI workflow.

## References
- `crates/cli/tests/ide_cli_contract.rs`
- `.github/workflows/ci.yml`
- `docs/design/phase-25.2.8-ide-cli-contract.md`
- `docs/design/phase-25.2.9-non-interactive-cli-contract.md`
- `docs/design/phase-25.2.10-vscode-command-profile.md`
