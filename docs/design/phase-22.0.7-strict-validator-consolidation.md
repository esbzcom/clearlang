# Phase 22.0.7 - Strict Validator Consolidation

## Status
Design lock target for `22.0.7` in `docs/TODO.md`.

## Goal
Prevent drift across strict preflight validators by using one shared validation module for:
- package identifiers,
- exact semver pins,
- sha256 digests,
- UTC RFC3339 timestamp parsing.

## Locked Implementation
1. Shared module:
   - `crates/cli/src/commands/build/strict_validation.rs`
2. Consumers migrated to shared validators:
   - `strict_lockfile.rs`
   - `strict_package_contract.rs`
   - `strict_package_signatures.rs`
   - `strict_trust_policy.rs` (timestamp parsing)

## Determinism Contract
1. Validation rules and error wording remain stable for equivalent invalid inputs.
2. Ordering and duplicate-detection behavior remains deterministic.
3. Future strict schema additions must reuse shared validators unless explicitly justified.

## Regression Gates
- Shared validator unit tests in `strict_validation.rs`.
- Existing strict loader/signature test suites remain green after migration.

## References
- `docs/TODO.md` (`22.0.7`)
- `docs/design/phase-22.0.4-package-metadata-abi-compatibility-policy.md`
