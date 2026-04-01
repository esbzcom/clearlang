# Phase 25.2.8 - IDE/VSCode CLI Contract

## Status
Design lock + implementation record for `25.2.8` in `docs/TODO.md`.

## Goal
Provide stable machine-readable CLI behavior for IDE/plugin integration across primary workflows:
- `clg check`
- `clg test` (contract reserved; command lands in `25.3.2`)
- `clg release`

## Machine-Readable Outputs

### 1) Deterministic error channel (`--json-errors`)
- Failure diagnostics are emitted as JSON to `stdout`.
- Human/migration/progress logs stay on `stderr`.
- Error payload shape is stable:
  - top-level: `{ "ok": false, "errors": [...] }`
  - item fields: `code`, `stage`, `message`, `file`, `start`, `end`, optional `function`.

### 2) Structured progress channel (`--json-events`)
- Progress/stage events are emitted as NDJSON on `stderr`.
- Event schema is stable (`schema_version: 1`):
  - `schema_version`
  - `command`
  - `level`
  - `event`
  - `stage`
  - optional `fields` map

## Exit-Code Contract
- `0`: success.
- `1`: deterministic command failure (diagnostic/internal command error).
- `2`: CLI usage/argument contract failure (clap parsing/required flags).

## Command Coverage
- Implemented in this task:
  - `check`: `--json-errors`, `--json-events`, deterministic exit-code behavior.
  - `release`: `--json-errors`, `--json-events`, deterministic exit-code behavior.
- Reserved for `25.3.2`:
  - `test` command must implement the same `--json-errors`/`--json-events`/exit-code contract exactly.

## References
- `crates/cli/src/main.rs`
- `crates/cli/src/logging.rs`
- `crates/cli/tests/cli_it/basic.rs`
- `crates/cli/tests/cli_it/diagnostics/release_command.rs`
- `docs/diagnostics.md`
