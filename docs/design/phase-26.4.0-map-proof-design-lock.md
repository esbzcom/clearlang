# Phase 26.4.0 - Map Proof Gate (Std Gate E) Design Lock

## Scope
Define the ordered execution and machine-checkable completion criteria for map proof support in Milestone 26.

## Policy Decisions
1. Read-only map proof closure (`new`, `len`, `is_empty`, `contains`, `get`) is mandatory before mutating map proof surfaces are release-enabled.
2. Mutating map proof surfaces (`insert`, `insert_take`, `remove`, `remove_take`) are release-disabled until their corresponding Gate E rows are implemented and gated.
3. Theorem-grade claims over map properties require zero assumption boundaries on map-related VCs.
4. Deterministic present/absent-key behavior and overwrite semantics remain part of the first-production compatibility contract and must be preserved while proofs are expanded.
5. Guarded mutable compatibility APIs (`can_mut`, `insert_mut`, `remove_mut`) remain release-scoped compatibility rows and MUST stay conformance-locked while Gate E closes.
6. Iteration/export map APIs (`keys`, `values`, `entries`, iterators) remain release-disabled in Gate E and require a follow-up canonical-ordering lock before enablement.

## Assumption-Boundary Policy
- Gate E does not introduce a new permanent release assumption ID for map semantics.
- Temporary development assumptions for map reasoning are allowed only in non-release workflows and MUST be blocked by strict release gates.
- Release/theorem-grade path for map surfaces MUST satisfy:
  - `assumptions.items == []` on map-related VCs,
  - strict compiler mode acceptance (no `C033`),
  - verifier assurance acceptance for required policy.

## Ordered Execution (Must Follow)
1. `26.4.1` close read-only map proofs (`new`, `len`, `is_empty`, `contains`, `get`) with deterministic diagnostics/tests.
2. `26.4.2` add VC/SMT reasoning for key-membership/value-consistency invariants and overwrite semantics.
3. `26.4.3` close mutating map proof semantics (`insert`, `insert_take`, `remove`, `remove_take`) after `26.4.2`, plus alias-conformance checks for `insert_mut`/`remove_mut`.
4. `26.4.4` enforce theorem-grade strict no-assumption gate for release-enabled map proof surfaces.
5. `26.4.5` update compatibility row evidence for `can_mut` + mut aliases and fail closed on drift.
6. `26.4.6` run deterministic performance guardrail checks last after semantic/proof closure.
7. `26.4.7` publish post-Gate-E iteration/ordering lock before enabling any map iteration/export API.

## Fail-Closed Sequencing Rules
- If mutating map proof rows are enabled before read-only closure, Gate E fails closed.
- If release enables mutating map proofs before `26.4.2` invariant closure, Gate E fails closed.
- Any regression from `proved` to non-proved status for release-enabled map rows fails closed.
- If `can_mut`/`insert_mut`/`remove_mut` compatibility rows drift from conformance evidence, Gate E fails closed.
- If any map iteration/export API is release-enabled without the `26.4.7` canonical-ordering lock and conformance tests, Gate E fails closed.

## Deterministic Performance Guardrails (26.4.6)
- Guardrail profile input is pinned and versioned (same solver profile as release proof gates).
- CI gate budgets for the map-proof suite:
  - median wall time regression limit: `<= +10%` vs pinned baseline,
  - p95 wall time regression limit: `<= +20%` vs pinned baseline,
  - timeout count: `0` for release-enabled map proof fixtures.
- Any budget breach fails the Gate E performance check.

## Required Evidence Artifacts
- Coverage status updates in:
  - `docs/std/coverage-matrix.md` (`typed|runtime|proved` rows for `std::map` symbols),
  - `docs/proofs/proof-coverage-matrix.{md,json}` (map-related proof surfaces).
- Compatibility row maintenance when enabled:
  - `can_mut`,
  - `insert_mut`,
  - `remove_mut`.
- Deterministic regression tests for:
  - empty/non-empty behavior for `is_empty`,
  - key-present/key-absent behavior for `contains/get`,
  - deterministic `can_mut` guard diagnostics + mut-alias conformance behavior,
  - mutation and take-variant proof outcomes under strict mode,
  - performance guardrail checks for map fixtures.
- Performance evidence publication:
  - `docs/evidence/phase-26.4-map-performance.{md,json}`.

## Exit Criteria
Gate E is complete when:
1. Required map API rows for this phase are implemented with deterministic typing/runtime behavior in the prescribed order.
2. Map-related proof rows required by this phase are `proved` with zero assumption boundaries in strict release workflow.
3. Coverage matrices and CI gates enforce drift detection for map API/proof status.
4. Deterministic present/absent-key and overwrite semantics remain conformant.
5. Performance guardrails remain green under pinned deterministic inputs.
