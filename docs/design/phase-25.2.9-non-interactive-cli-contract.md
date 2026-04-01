# Phase 25.2.9 - Non-Interactive Primary Command Contract

## Status
Design lock + implementation record for `25.2.9` in `docs/TODO.md`.

## Goal
Guarantee non-interactive, plugin-safe behavior for primary commands:
- `clg check`
- `clg test` (reserved contract; command implementation in `25.3.2`)
- `clg release`

## Contract

### 1) Non-Interactive Mode
- Global flag: `--non-interactive`.
- Primary commands do not prompt for user input.
- Invocation is deterministic and safe for IDE/background execution.

### 2) Deterministic stdout/stderr Separation
- Success payloads remain on `stdout`.
- Diagnostic failures:
  - `--json-errors` => JSON payload on `stdout`.
  - non-JSON failures => human-readable text on `stderr`.
- Stage/progress stream:
  - `--json-events` => NDJSON events on `stderr`.

### 3) Plugin-Safe Logging
- Machine event stream is schema-versioned (`schema_version: 1`) and line-delimited JSON.
- Primary release path avoids extra lock-step human summary lines on `stdout`; `clg release` stdout remains deterministic release payload output.

## Notes
- `clg test` must adopt the same non-interactive and channel-separation contract when implemented.

## References
- `crates/cli/src/main.rs`
- `crates/cli/src/logging.rs`
- `crates/cli/src/commands/pkg/lock_command.rs`
- `crates/cli/src/commands/release.rs`
- `crates/cli/tests/cli_it/basic.rs`
- `crates/cli/tests/cli_it/diagnostics/release_command.rs`
