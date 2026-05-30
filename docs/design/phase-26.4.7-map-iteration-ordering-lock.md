# Phase 26.4.7 - Map Iteration and Ordering Expansion Lock

## Scope
Define release requirements for adding `std::map` iteration/export APIs (`keys`, `values`, `entries`, iterators) after Gate E.

## Policy Decisions
1. No map iteration/export API is release-enabled until canonical ordering is defined and tested.
2. Canonical ordering rules MUST be stable across identical logical map content.
3. Serialization/hash/signing workflows MUST use only canonical ordering outputs.
4. Expansion is additive-only and must preserve existing map API behavior.

## Required Contracts
- Deterministic ordering specification for all added iteration/export APIs.
- Deterministic behavior for empty/singleton/multi-entry maps.
- Deterministic behavior when keys are structurally equal and values differ.
- No regression to existing `contains/get/insert/remove/insert_take/remove_take` semantics.

## Required Evidence
- Coverage updates in:
  - `docs/std/coverage-matrix.md`
  - `docs/proofs/proof-coverage-matrix.{md,json}` (if proof-enabled)
- Runtime conformance tests for canonical ordering outputs.
- Negative tests for non-canonical or unstable ordering regressions.
- Profile/performance checks for representative iteration workloads.

## Fail-Closed Rules
- If ordering is unspecified for any release-enabled iteration/export API, fail closed.
- If conformance tests do not prove deterministic canonical ordering, fail closed.
- If enabling iteration/export APIs regresses existing map semantics, fail closed.

## Exit Criteria
1. Canonical ordering lock is encoded in docs and tests.
2. New map iteration/export rows are tracked in coverage matrices.
3. Conformance and performance gates are green under pinned deterministic inputs.
