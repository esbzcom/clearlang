# Phase 25.4.9 - Release Legacy Parameter Retirement (Pre-GA)

## Status
Design lock + implementation record for `25.4.9` in `docs/TODO.md`.

## Goal
Retire legacy non-secret `clg release` parameters and make `clg.project.json` the canonical release configuration source before GA.

## Design Principles Check
- Simple for users: default release invocation should be short (`clg release <FILE> --root <DIR>`).
- AI-friendly: one manifest source-of-truth reduces parameter ambiguity in tool-generated flows.
- Provably correct: release inputs stay explicit, deterministic, and fail closed when missing.
- Crypto-focused: release verification/signing gates remain strict and unchanged.

## Parameters To Retire
Remove these options from primary `clg release` CLI surface:
- `--advisory-as-of`
- `--key-id`
- `--out-dir`
- `--trust-policy`

These values must resolve from `clg.project.json` `release_defaults`.

## Parameters To Keep Explicit
Keep key-material parameters explicit:
- `--key`
- `--pubkey`

Rationale: sensitive key-path handling should stay operator-controlled and avoid accidental repository coupling.

## Post-Cutover Command Contract
Primary release command:

```powershell
clg release <FILE> --key <FILE> --pubkey <FILE> --root <DIR>
```

Required manifest defaults under `<DIR>/clg.project.json`:
- `release_defaults.advisory_as_of`
- `release_defaults.key_id`
- `release_defaults.out_dir`
- `release_defaults.trust_policy`

If required manifest values are missing/placeholder, command fails closed with deterministic diagnostics.

## Pre-GA Compatibility Policy
Because product is pre-GA, no long-term compatibility commitment is required for removed non-secret release flags.
Cutover is immediate once `25.4.9` ships and Gate E acceptance tests pass.

## Acceptance Criteria
1. `clg release --help` no longer lists retired non-secret parameters.
2. Passing retired parameters fails as usage/contract error.
3. Release succeeds when manifest defaults are valid and key/public-key flags are provided.
4. Release fails closed when required manifest defaults are unresolved.
5. Docs and examples use manifest-first release command shape.

## References
- `docs/design/phase-25.4.1-project-manifest-v1.md`
- `crates/cli/src/main.rs`
- `crates/cli/src/commands/release.rs`
- `crates/cli/tests/cli_it/diagnostics/release_command.rs`
- `crates/cli/tests/cli_it/basic/build_release.rs`
- `docs/release-process.md`
- `docs/TODO.md`
