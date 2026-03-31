# Phase 25.2.4 - `clg release` Proved-Only Default

## Status
Design note + implementation lock for `25.2.4` in `docs/TODO.md`.

## Goal
Ensure `clg release` enforces theorem-grade proof by default with no optional downgrade path for release artifacts.

## Policy
`clg release` hardcodes:
1. strict production build (`--compiler-mode strict --release-profile production` behavior),
2. theorem-grade verify gate (`--require-assurance proved_all` behavior),
3. fail-closed execution on any non-`proved_all` outcome.

No release-command flags exist to relax these requirements.

## Enforcement
- Build stage fails with `C121` when theorem-grade proof status is not met.
- Verify stage is always compile-time and always requires `proved_all`.
- CLI help for `clg release` intentionally excludes downgrade controls (`--compiler-mode`, `--release-profile`, `--proof-strict`, `--require-assurance`).

## References
- `crates/cli/src/commands/release.rs`
- `crates/cli/tests/cli_it/basic.rs`
- `crates/cli/tests/cli_it/diagnostics/release_command.rs`
- `docs/release-process.md`
