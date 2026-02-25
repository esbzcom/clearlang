# Phase 19.3.2 - Verified std/core subset publication

## Status
Design lock for `19.3.2` in `docs/TODO.md`.

## Goal
Publish a deterministic, machine-readable verified std/core subset profile that includes only proof-backed surfaces and ties each surface to regression obligations.

## Design Principles Check
- Simple for users: one published profile (`verified.std_core.v1`) defines what is currently verified without assumption boundaries.
- AI-friendly: profile metadata is machine-readable and validated against proof-coverage matrix entries.
- Provably correct: profile membership is restricted to `proved` coverage entries with no assumption boundaries.
- Crypto-focused: deferred/unchecked crypto/bitwise/unsigned/external surfaces remain outside the verified subset until formally proved.

## Scope
1. Publish profile artifacts:
   - human-readable subset doc,
   - machine-readable subset JSON.
2. Include proof-backed contract/core surfaces only (from coverage matrix `proved` entries).
3. Attach regression obligations per subset entry with concrete test references.
4. Add regression test to enforce:
   - subset-to-coverage consistency (`proved`, no assumptions),
   - obligation references point to real tests.

## Locked Behavior
1. `verified.std_core.v1` includes only current `proved` std/core feature rows.
2. Any coverage regression from `proved` -> `assumed` for subset entries fails subset regression tests.
3. Any missing or renamed obligation test references fail subset regression tests.

## Non-Goals
1. No new proof semantics or assumption-boundary IDs.
2. No CI policy gate for assurance downgrade detection (owned by `19.3.3`).
3. No expansion to assumed intrinsics in this slice.

## Exit Criteria for 19.3.2
1. Published subset artifacts exist in `docs/proofs/`.
2. Subset regression test verifies profile consistency and obligation references.
3. TODO/rollout docs move next execution to `19.3.3`.

## References
- `docs/TODO.md`
- `docs/rollout/DEVPLAN.md`
- `docs/proofs/proof-coverage-matrix.md`
- `docs/proofs/proof-coverage-matrix.json`
