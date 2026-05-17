# Phase 19.5.2 - `clg verify --explain` Human Summary

## Status
Design lock for `19.5.2` in `docs/TODO.md`.

## Goal
Add a deterministic human-readable explanation mode for verification that reports what is checked, what remains assumed, and why assumptions exist.

## Design Principles Check
- Simple for users: one flag (`--explain`) prints a concise assurance summary after successful verify.
- AI-friendly: output uses stable line keys (`assurance`, `vcs_*`, `assumed_boundaries`, `dependency_trust_labels`) for tooling extraction.
- Provably correct: explanation is derived from signed payload + embedded proof section already verified in the verify flow.
- Crypto-focused: assumption boundaries and trust labels for crypto/external surfaces are explicit and visible in audit output.

## Scope
1. Extend `clg verify` with `--explain`.
2. After successful verification, print summary including:
   - mode, scope, artifact hashes, assurance tier/label,
   - VC totals and checked-vs-assumed counts,
   - assumed boundary list with reason text,
   - dependency trust labels (`primitive`/`external`) when present,
   - trust-anchor versions/policy path when available.
3. Keep existing verify success/failure behavior unchanged when `--explain` is not used.
4. Add integration coverage for:
   - checked-core summary output,
   - assumed-boundary reasoning output.

## Locked Behavior
1. `--explain` is informational only; it does not alter verification decisions.
2. Explanation output is emitted only on successful verify.
3. Output ordering is deterministic via sorted boundary/dependency aggregation.

## Non-Goals
1. No release-policy enforcement in verify output (`19.5.3`).
2. No machine JSON schema for explain mode in this slice.
3. No additional theorem-prover execution beyond existing verify-mode behavior.

## Exit Criteria for 19.5.2
1. `clg verify --explain` prints the required human summary fields.
2. Regression tests cover checked-core and assumed-boundary summary paths.
3. TODO focus advances to `19.5.3`.

## References
- `docs/TODO.md`
- `docs/TODO.md`
- `crates/cli/src/main.rs`
- `crates/cli/src/commands/verify.rs`
- `crates/cli/tests/signing.rs`
