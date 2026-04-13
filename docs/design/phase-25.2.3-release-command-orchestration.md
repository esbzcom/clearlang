# Phase 25.2.3 - `clg release` One-Command Orchestration

## Status
Design note + implementation lock for `25.2.3` in `docs/TODO.md`.

## Goal
Implement one-command release orchestration with fail-closed behavior:
- `lock -> build/prove -> sign -> verify -> bundle`

## Design Gap Resolved
Two trust-policy contracts are required and intentionally separate:
1. **Strict preflight policy** (build input): `clg.trust-policy.json` (schema v0).
2. **Compile-time verify trust-anchor policy**: `trust-policy.json` (schema v1 with `trust_anchors`).

`clg release` resolves this by:
- using module-root strict preflight inputs for strict build (`clg.*` files);
- taking `release_defaults.trust_policy` (default value in `<root>/clg.project.json`) for verify-mode policy;
- reading `trust_anchors.lean_checker` and `trust_anchors.coq_checker` from that verify policy and passing them to build/sign so compile-time verification is deterministic and aligned.

## Command Behavior
`clg release` now performs stages in order:
1. `clg pkg lock` (`--generate` or `--update` auto-selected by lockfile presence) in strict mode with required advisory timestamp.
   - Lock roots are sourced from `clg.project.json` schema v1 `dependencies[]` when present.
   - Release entrypoint is sourced from `clg.project.json` schema v1 `project.entry`.
2. strict production build + proof + sign with deterministic output paths.
3. compile-time verify with `--require-assurance proved_all`.
4. bundle manifest emission with deterministic artifact hashes.

If any stage fails, command fails immediately (fail-closed).

## Artifacts
For `project.entry` file `<stem>.clear` and output directory `<out>`:
- `<out>/<stem>.wasm`
- `<out>/<stem>.vc.json`
- `<out>/<stem>.proof.json`
- `<out>/<stem>.sig.json`
- `<out>/<stem>.assurance.json`
- `<out>/<stem>.release-bundle.json` (manifest with SHA-256 hashes for produced artifacts)

## References
- `crates/cli/src/commands/release.rs`
- `crates/cli/tests/cli_it/diagnostics/release_command.rs`
- `docs/design/phase-25.2.2-release-ux-design-lock.md`
- `docs/release-process.md`
