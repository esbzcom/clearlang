# Phase 25.4.11 - Trust-Policy UX Clarification

## Status
Design lock + implementation record for `25.4.11` in `docs/TODO.md`.

## Goal
Make trust-policy naming unambiguous across CLI UX, manifest defaults, diagnostics, and docs.

## Canonical Contract
Two policy files are intentionally separate:

1. Strict package trust policy (strict preflight/build/runtime package trust gate):
- Filename: `clg.trust-policy.json`
- Scope: strict package signature/trust enforcement (`C103`, `C110`, runtime trust gates)

2. Compile-time verify trust-anchor policy:
- Filename (default/expected): `trust-policy.json`
- Scope: `clg verify --verify-mode compile-time --trust-policy <FILE>` and release/check trust-anchor checks (`V004`)

`clg.project.json` `release_defaults.trust_policy` always points to the compile-time verify trust-anchor policy path.

## Cutover Requirements
1. Docs/examples must use `trust-policy.json` for compile-time verify commands.
2. Diagnostics text must explicitly distinguish compile-time trust-anchor policy from strict package trust policy.
3. CLI error wording must avoid ambiguous "trust policy" references when compile-time verify policy is intended.

## Evidence
- Implementation:
  - `crates/cli/src/commands/verify/core.rs`
  - `crates/cli/src/signing.rs`
  - `crates/cli/src/commands/release_defaults.rs`
  - `crates/cli/src/commands/check.rs`
- Docs:
  - `docs/diagnostics.md`
  - `docs/release-process.md`
  - `examples/projects/README.md`
- Task tracking:
  - `docs/TODO.md`
