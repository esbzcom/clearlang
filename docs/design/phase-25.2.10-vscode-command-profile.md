# Phase 25.2.10 - VSCode Plugin Command Profile

## Status
Design lock + documentation record for `25.2.10` in `docs/TODO.md`.

## Goal
Publish a stable VSCode plugin-facing CLI profile for primary commands so integrations can rely on deterministic invocation, output schemas, and cancellation handling.

## Required Invocation Profile
For plugin execution, invoke primary commands with:
- `--non-interactive`
- `--json-errors`
- `--json-events`

Canonical command shapes:
- `clg --non-interactive --json-errors --json-events check <FILE> --root <DIR>`
- `clg --non-interactive --json-errors --json-events release --key <FILE> --pubkey <FILE> --root <DIR>`

## Output Contract (Pinned)
- Error payload: JSON on `stdout` with stable shape `{ "ok": false, "errors": [...] }`.
- Event stream: NDJSON on `stderr` with `schema_version: 1`.
- Exit codes:
  - `0` success
  - `1` deterministic command failure
  - `2` usage contract failure

## Cancellation and Timeout Contract
- CLI remains non-interactive and does not prompt.
- Timeout policy is owned by the plugin/IDE runner, not by per-command timeout flags.
- Plugin runners should cancel by terminating the process and treating the run as cancelled when no complete success/error payload is produced.
- Suggested default timeout budgets:
  - `check`: short budget (for example 30s)
  - `release`: longer budget (for example 10m)

## References
- `docs/ide/vscode-cli-profile.md`
- `docs/diagnostics.md`
- `docs/release-process.md`
- `README.md`
