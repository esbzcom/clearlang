# Phase 25.0.11 - Release Symbol Allowlist Gate

## Status
Design lock + implementation note for `25.0.11` in `docs/TODO.md`.

## Goal
Fail closed for `release == proved` by allowing release assurance only when all bundle symbols are covered by proved std surfaces in the canonical proof matrix.

## Policy
1. Signature payload includes deterministic `bundle_symbols` extracted from program calls to `std::...` APIs.
2. For `clg verify --require-assurance proved_all`, verification must reject any bundle symbol not present in:
   - `docs/proofs/proof-coverage-matrix.json`
   - where `entries[].status == "proved"`
3. Verification fails with policy error (`V005`) when:
   - `bundle_symbols` payload shape is invalid
   - proof matrix cannot be loaded/parsed
   - any bundle symbol is outside the proved allowlist

## Enforcement Wiring
- Signature payload extension:
  - `crates/cli/src/proofs.rs`
- Verify policy gate:
  - `crates/cli/src/commands/verify.rs`
- Regression tests:
  - `crates/cli/tests/signing/tests.rs`
  - `crates/cli/tests/ci_workflow.rs`
- CI proof regression wiring:
  - `.github/workflows/ci.yml`
- Gate lock metadata:
  - `docs/evidence/milestone_3-proof-gate.lock.json`

## Exit Criteria for 25.0.11
1. New signatures carry deterministic `bundle_symbols`.
2. `--require-assurance proved_all` rejects disallowed symbols.
3. CI proof regression step runs allowlist rejection test.
4. Milestone lock records allowlist source/rule.

## References
- `docs/TODO.md`
- `docs/proofs/proof-coverage-matrix.json`
- `docs/evidence/milestone_3-proof-gate.lock.json`
- `crates/cli/src/proofs.rs`
- `crates/cli/src/commands/verify.rs`
- `crates/cli/tests/signing/tests.rs`
