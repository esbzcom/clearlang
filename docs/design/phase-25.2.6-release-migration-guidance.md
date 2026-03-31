# Phase 25.2.6 - Legacy Release-Path Migration Guidance

## Status
Design lock + implementation record for `25.2.6` in `docs/TODO.md`.

## Goal
Keep the primary user surface simple (`clg check`, `clg test`, `clg release`) while retaining advanced command access for expert/debug workflows.

## Policy
1. Primary docs/help must treat `clg release` as the production release entrypoint.
2. Flag-heavy legacy release-like paths (`clg build` + `clg verify` orchestration by hand) are expert/debug-only.
3. No compatibility aliases are required in pre-production.

## CLI Behavior Lock
Migration guidance is emitted when users invoke legacy release-like flows:
- `clg build` with release-like intent (`--release-profile production` or signing path).
- `clg verify` with release-gate intent (`--verify-mode compile-time`, `--release-policy`, or `--require-assurance`).

Guidance text points users to:

```text
clg release <FILE> --key <FILE> --pubkey <FILE>
```

This guidance is emitted on `stderr` and does not alter exit-code semantics.

## Help/Docs Lock
- `clg build` and `clg verify` help text explicitly mark these as advanced expert/debug commands.
- Release-process and README docs mark manual build/verify release flows as advanced.

## References
- `crates/cli/src/main.rs`
- `crates/cli/tests/cli_it/basic.rs`
- `docs/release-process.md`
- `README.md`
