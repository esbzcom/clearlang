# Phase 25.2.7 - `clg check` Strict-Aligned Preflight

## Status
Design lock + implementation record for `25.2.7` in `docs/TODO.md`.

## Goal
Provide a fast deterministic local-iteration command aligned with release policy inputs, without running full release orchestration.

## Command Contract

```text
clg check <FILE> [--root <DIR>]
```

Behavior:
1. Resolve `root` from `--root` or `<FILE>` parent.
2. Validate strict release-preflight inputs under `root`:
   - `clg.lock.json`
   - `clg.trust-policy.json`
   - `clg.host-profile.json`
   - `clg.package-metadata.json`
   - `clg.package-abi.json`
3. Validate release-default inputs:
   - `clg.project.json`
   - release-default trust policy target (typically `trust-policy.json`)
4. Parse + type-check target program and imports.
5. Do not emit release artifacts, do not sign, do not run release-grade orchestration.

## Determinism / Fail-Closed
- Uses the same strict preflight validators as strict build/release policy inputs.
- Fails closed on malformed/missing preflight/default files.
- Emits stable JSON diagnostics via `--json-errors` with stage `check` for preflight/default failures and stage `type` for type-check failures.

## References
- `crates/cli/src/commands/check.rs`
- `crates/cli/src/main.rs`
- `crates/cli/src/commands/build/strict_preflight_api.rs`
- `crates/cli/src/commands/release_defaults.rs`
- `crates/cli/tests/cli_it/basic.rs`
