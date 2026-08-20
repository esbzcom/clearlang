# Phase 30.5.2 - Simulation Diagnostics and VS Code CLI Profile

Date: 2026-08-20
Status: Implemented and locked
Owner: developer-experience-owner

## Decision

Contract simulation diagnostics extend the existing stable VS Code CLI profile; this phase does
not introduce a second editor protocol or an unversioned language-server transport. Editor and
LSP clients invoke `clg` with `--non-interactive --json-errors --json-events` and consume the
already-pinned diagnostic envelope on stdout.

For an execution failure in a supported simulated transition, `clg simulate` writes a
`clg.contract-simulation-trace.v1` trace before returning failure. The trace contains:

- `failure_location`: `file`, byte `start`/`end`, and `function`.
- `stack`: caller-to-failing frames, each with the same source fields plus a zero-based lowered
  `instruction` index.

The current scalar simulator has one executable entry frame. A preserved guard IR span is used
for that frame and failure location; other instructions fall back to the selected function body.
Calls that the simulator cannot model remain fail-closed and never synthesize a cross-contract
stack.

With `--json-errors`, the same failure returns `C142` at stage `simulate`. Its stable core
`file`, `start`, and `end` fields exactly match `failure_location`, and its optional `function`
matches the selected entrypoint. This is an additive use of the profile's optional field, not a
schema change.

## Compatibility and support boundary

- The stdout error envelope remains `{ "ok": false, "errors": [...] }`.
- The NDJSON event stream remains on stderr at schema version 1.
- `failure_location` and `stack` are optional trace fields; existing trace consumers remain
  compatible.
- VS Code integrations use the documented CLI process contract in
  `docs/ide/vscode-cli-profile.md`; no bundled extension or LSP server is claimed by this phase.

## Verification

The CLI integration suite proves a failing `require` produces C142 JSON, matching source location
in the trace, a deterministic entry-frame stack, and a stderr-only JSON event stream.

## References

- `docs/ide/vscode-cli-profile.md`
- `docs/design/phase-25.2.10-vscode-command-profile.md`
- `docs/diagnostics.md`
