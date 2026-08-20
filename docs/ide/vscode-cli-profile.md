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
clg --non-interactive --json-errors --json-events release --key keys/signing.json --pubkey keys/public.json --root examples/projects/generic
```

Test runner:

```powershell
clg --non-interactive --json-errors --json-events test examples/projects/testing --report json
```

Contract simulation:

```powershell
clg --non-interactive --json-errors --json-events simulate contract.clear --function transfer --state state.json --args args.json --state-out state-out.json --trace-out trace.json
```

`clg test` uses the same stdout/stderr and event-channel contract as `check`/`release`.

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

### Contract simulation failures

When a supported contract transition fails during execution, `simulate --json-errors` returns
the same error envelope with `code: "C142"`, `stage: "simulate"`, and a `function` field. The
`file`, `start`, and `end` fields identify the failing guard when the lowered instruction retains
a source span; otherwise they identify the selected function body. VS Code/LSP clients should use
these fields as the primary diagnostic range.

The `--trace-out` artifact is `clg.contract-simulation-trace.v1`. On execution failure it adds:

```json
{
  "failure_location": { "file": "contract.clear", "start": 0, "end": 1, "function": "transfer" },
  "stack": [
    { "file": "contract.clear", "start": 0, "end": 1, "function": "transfer", "instruction": 4 }
  ]
}
```

`stack` is ordered caller-to-failing frame and currently contains the selected simulator entry
frame only; unsupported calls fail closed rather than fabricating cross-contract frames. The
optional trace fields are additive and do not change the CLI profile schema version.

### Event NDJSON (`--json-events`, stderr)
One event per line:

```json
{"schema_version":1,"command":"check","level":"info","event":"start","stage":"check_preflight"}
```

Stability rules:
- `schema_version` is currently pinned to `1`.
- New optional fields may be added without changing the version.
- Breaking changes require a schema-version bump.

`clg test` report contract:
- pass/fail summaries are emitted by `--report` (`human|json|junit`) on `stdout`.
- JSON report (`--report json`) keeps `schema_version: 1` and stable per-test fields, including:
  - `failure_kind` / `failure_code` (`C137|C138|C139|C141`)
  - `failure_id` deterministic taxonomy string
  - optional `expected_outcome` and `assertion_diff` for advanced assertion/expectation flows
  - `captured_stdout` / `captured_stderr`
  - `replay.argv` for single-test replay flow.
  - runtime safety reasons for `C138` (`fuel_exhausted`, `memory_limit`, `worker_crash`) when applicable.

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
