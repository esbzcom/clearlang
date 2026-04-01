# Phase 25.2.14 - `clg lint` Deterministic Lints

## Status
Design lock + implementation record for `25.2.14` in `docs/TODO.md`.

## Goal
Add deterministic linting for `.clear` sources with stable diagnostics and fail-closed warning gating.

Command surface:
- `clg lint <PATH>`
- `clg lint <PATH> --deny-warnings`

## Deterministic Lint Rules (v1)
- `lint.trailing_whitespace`
- `lint.crlf_line_endings`
- `lint.missing_final_newline`

Traversal order is deterministic (sorted file paths), and warning output order is stable by `(file, line, column, rule)`.

## `--deny-warnings` Contract
- Default mode: warnings are reported, command exits `0`.
- `--deny-warnings`: any warning fails closed with `C133` (`stage=lint`).
- `--json-errors` + `--deny-warnings`: structured JSON failure on `stdout`.

## Diagnostics
- New code: `C133` (`stage=lint`) for lint contract failures:
  - invalid path / no `.clear` inputs
  - read failure
  - warnings rejected by `--deny-warnings`

## References
- `crates/cli/src/main.rs`
- `crates/cli/src/commands/lint.rs`
- `crates/cli/tests/cli_it/basic.rs`
- `docs/diagnostics.md`
- `docs/release-process.md`
- `README.md`
