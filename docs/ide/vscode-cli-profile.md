# VSCode CLI Profile (Phase 25.2.10)

This profile defines the stable command contract for VSCode/plugin integrations.

## Command Profile

### Required global flags
- `--non-interactive`
- `--json-errors`
- `--json-events`

### Recommended invocations

Check/preflight:

```powershell
clg --non-interactive --json-errors --json-events check examples/projects/generic/main.clear --root examples/projects/generic
```

Release orchestration:

```powershell
clg --non-interactive --json-errors --json-events release examples/projects/generic/main.clear --key keys/signing.json --pubkey keys/public.json --root examples/projects/generic
```

`clg test` is part of the primary UX contract and must adopt the same flags/channels when it lands in `25.3.2`.

## Machine-Readable Contract

### Error JSON (`--json-errors`, stdout)

```json
{
  "ok": false,
  "errors": [
    {
      "code": "C121",
      "stage": "build",
      "message": "...",
      "file": "path/to/file.clear",
      "start": 0,
      "end": 0
    }
  ]
}
```

Stability rules:
- Top-level keys remain `ok` + `errors`.
- `ok` is always `false` for error payloads.
- `errors` remains an array with stable core fields:
  - `code`, `stage`, `message`, `file`, `start`, `end`
  - optional: `function`

### Event NDJSON (`--json-events`, stderr)
One event per line:

```json
{"schema_version":1,"command":"check","level":"info","event":"start","stage":"check_preflight"}
```

Stability rules:
- `schema_version` is currently pinned to `1`.
- New optional fields may be added without changing the version.
- Breaking changes require a schema-version bump.

## Channel and Exit-Code Contract
- Success payloads: `stdout`.
- `--json-errors` failure payloads: `stdout`.
- Human diagnostics/progress: `stderr`.
- `--json-events` stream: `stderr` only.

Exit codes:
- `0`: success
- `1`: deterministic command failure
- `2`: usage/argument contract failure

## Cancellation and Timeout Behavior
- Commands are non-interactive under `--non-interactive`; plugin invocations must not expect prompts.
- The CLI does not define per-command timeout flags; timeout policy is managed by the plugin/runner.
- On timeout or user cancel, terminate the CLI process and mark the run as cancelled in the plugin.
- If a process is cancelled before a complete success/error payload is emitted, treat the run as cancelled (not as a parsed compiler diagnostic).

Suggested timeout policy (plugin default):
- `check`: 30 seconds
- `release`: 10 minutes

## References
- `docs/design/phase-25.2.8-ide-cli-contract.md`
- `docs/design/phase-25.2.9-non-interactive-cli-contract.md`
- `docs/design/phase-25.2.10-vscode-command-profile.md`
- `docs/diagnostics.md`
