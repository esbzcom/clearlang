# Phase 25.2.2 - Gate C Release UX Design Lock

## Status
Design lock for `25.2.2` in `docs/TODO.md`.

## Goal
Lock a simplified primary CLI surface for end users and AI tools while keeping release policy fail-closed (`release == proved`).

## Primary Command Surface
Shipped primary commands are:
- `clg check`
- `clg release`

Planned primary command (reserved contract):
- `clg test` (command implementation lands in `25.3.2`)

Advanced/expert flows (`build`, `verify`, `pkg lock`, and detailed flags) remain available but are not the primary UX surface.

## `clg release` Contract (Gate C Shape)
`clg release` is the single production release entrypoint. The concrete CLI shape is:

```text
clg release <FILE> \
  --advisory-as-of <RFC3339_UTC> \
  --key <FILE> \
  --key-id <ID> \
  --pubkey <FILE> \
  [--root <DIR>] \
  [--out-dir <DIR>] \
  [--trust-policy <FILE>]
```

Command contract requirements:
1. Release policy is fixed to theorem-grade (`proved_all`) for production artifacts.
2. Orchestration stages are fixed in order:
   - `lock`
   - `build/prove`
   - `sign`
   - `verify(require-assurance=proved_all)`
   - `bundle`
3. Output artifacts are deterministic path contracts rooted under `--out-dir`:
   - `<stem>.wasm`
   - `<stem>.vc.json`
   - `<stem>.proof.json`
   - `<stem>.sig.json`
   - `<stem>.assurance.json`
   - `<stem>.release-bundle.json`

## Non-Goals (This Task)
- Implementing full stage execution is tracked by `25.2.3`.
- `clg strict init <root>` prefill generation is tracked by `25.2.5`.
- IDE JSON/progress/exit-code contracts are tracked by `25.2.8` to `25.2.11`.

## References
- `docs/TODO.md`
- `docs/release-process.md`
- `README.md`
- `crates/cli/src/main.rs`
- `crates/cli/src/commands/release.rs`
