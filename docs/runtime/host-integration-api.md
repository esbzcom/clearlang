# Host Integration API (Phase 23.1.1)

## Goal
Enable production hosts to reuse the same automatic runtime package loader/linker path as `clg run`, without manual import wiring.

## Library Surface
`clg-cli` now exposes a library target:
- `clg_cli::commands`
- `clg_cli::logging`
- `clg_cli::signing`

Host integrations can call:
- `clg_cli::commands::run::run(...)`

This path includes:
- runtime-link artifact verification,
- strict lock/trust/signature checks,
- deterministic runtime package link binding,
- availability policy + mirror fallback behavior.

## Usage Pattern
Use the same project root inputs required by `clg run`:
- `clg.runtime-link.json` + hash
- `clg.lock.json`
- `clg.package-signatures.json`
- `clg.trust-policy.json`
- `clg.package-store-index.json`
- optional `clg.runtime-loader.json`

Call `run::run(file, invoke, json_errors, logger)` from the host adapter.

## Contract
- No manual per-import linker wiring is required for runtime package imports.
- Diagnostics remain stable (`R012`-`R017`) across CLI and host adapters using this API path.
