# Phase 25.2.13 - `clg fmt` Deterministic Formatter

## Status
Design lock + implementation record for `25.2.13` in `docs/TODO.md`.

## Goal
Add a deterministic formatter command for `.clear` sources:
- `clg fmt <PATH>`
- `clg fmt <PATH> --check`

## Command Contract
- Input path may be a single `.clear` file or a directory.
- Directory mode discovers `.clear` files recursively in deterministic sorted path order.
- Formatting is deterministic and idempotent:
  - normalize line endings to `\n`
  - trim trailing spaces/tabs on each line
  - enforce newline at end-of-file
- Non-`.clear` files are ignored in directory mode.

## `--check` Behavior
- `--check` does not rewrite files.
- If any file would change, command fails closed with diagnostic `C132` (`stage=fmt`).
- If no file would change, command succeeds.

## Diagnostics
- New code: `C132` (`stage=fmt`) for formatter contract failures:
  - invalid path
  - no `.clear` files found
  - read/write failures
  - check-mode drift (`--check`)

## References
- `crates/cli/src/main.rs`
- `crates/cli/src/commands/fmt.rs`
- `crates/cli/tests/cli_it/basic.rs`
- `docs/diagnostics.md`
- `docs/release-process.md`
- `README.md`
