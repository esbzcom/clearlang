# Phase 25.2.12 - Release Precheck Gate

## Status
Design lock + implementation record for `25.2.12` in `docs/TODO.md`.

## Sequencing Gap and Lock
`25.2.12` requires `fmt + lint + tests` gating before strict signed publish flow, but dedicated `clg fmt` / `clg lint` commands land in `25.2.13` / `25.2.14`.

Locked decision for this slice:
- Enforce a fail-closed release-precheck gate now for local + CI workflows using existing deterministic toolchain checks.
- Keep the gate explicit and reusable via `xtask`.
- Replace/extend this gate with `clg fmt`/`clg lint` command surfaces as they land.

## Gate Contract (Current)
New command:
- `cargo run -p xtask -- release-precheck`

Behavior:
- runs `cargo fmt --all -- --check`
- runs `cargo clippy --workspace --all-targets -- -D warnings` (lint gate)
- runs `cargo test --workspace`

Any failure blocks the release-precheck stage (fail-closed).

## CI and Local Wiring
- CI `checks` job now runs:
  - `Release precheck gate (fmt + lint + tests)` -> `cargo run -p xtask -- release-precheck`
- Local publish/release docs require the same command before strict signed release flow.

## References
- `xtask/src/main/core.rs`
- `xtask/src/main/artifacts_cli_models.rs`
- `.github/workflows/ci.yml`
- `docs/release-process.md`
- `README.md`
