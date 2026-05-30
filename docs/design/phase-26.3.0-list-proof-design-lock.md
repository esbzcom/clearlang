# Phase 26.3.0 - List Proof Gate (Std Gate D) Design Lock

## Scope
Define the ordered execution and machine-checkable completion criteria for list proof support in Milestone 26.

## Policy Decisions
1. Read-only list proof closure (`new`, `len`, `is_empty`, `get`) is mandatory before any mutating list proof surfaces are release-enabled.
2. Mutating list proof surfaces (`push`, `pop`, `insert`, `remove`, `remove_take`) are release-disabled until their corresponding Gate D rows are implemented and gated.
3. Theorem-grade claims over list properties require zero assumption boundaries on list-related VCs.
4. Deterministic out-of-range list behavior remains part of the first-production compatibility contract and must be preserved while proofs are expanded.

## Assumption-Boundary Policy
- Gate D does not introduce a new permanent release assumption ID for list semantics.
- Temporary development assumptions for list reasoning are allowed only in non-release workflows and MUST be blocked by strict release gates.
- Release/theorem-grade path for list surfaces MUST satisfy:
  - `assumptions.items == []` on list-related VCs,
  - strict compiler mode acceptance (no `C033`),
  - verifier assurance acceptance for required policy.

## Ordered Execution (Must Follow)
1. `26.3.1` close read-only list proofs (`new`, `len`, `is_empty`, `get`) with deterministic diagnostics/tests.
2. `26.3.2` add VC/SMT reasoning for index safety (`0 <= i < len`) and `get` value-preservation under unchanged list state.
3. `26.3.3` close append/pop proof semantics (`push`, `pop`) with deterministic length/emptiness invariants.
4. `26.3.4` close indexed mutation proof semantics (`insert`, `remove`, `remove_take`) after `26.3.2` and `26.3.3`.
5. `26.3.5` enforce theorem-grade strict no-assumption gate for release-enabled list proof surfaces.
6. `26.3.7` run deterministic performance guardrail checks last after semantic/proof closure.

## Fail-Closed Sequencing Rules
- If `push/pop/insert/remove/remove_take` proof rows are enabled before read-only closure, Gate D fails closed.
- If `insert/remove/remove_take` proof rows are enabled before append/pop closure, Gate D fails closed.
- Any regression from `proved` to non-proved status for release-enabled list rows fails closed.

## Deterministic Performance Guardrails (26.3.7)
- Guardrail profile input is pinned and versioned (same solver profile as release proof gates).
- CI gate budgets for the list-proof suite:
  - median wall time regression limit: `<= +10%` vs pinned baseline,
  - p95 wall time regression limit: `<= +20%` vs pinned baseline,
  - timeout count: `0` for release-enabled list proof fixtures.
- Any budget breach fails the Gate D performance check.

## Required Evidence Artifacts
- Coverage status updates in:
  - `docs/std/coverage-matrix.md` (`typed|runtime|proved` rows for `std::list` symbols),
  - `docs/proofs/proof-coverage-matrix.{md,json}` (list-related proof surfaces).
- Compatibility row maintenance when enabled:
  - `insert_checked`,
  - `remove_checked`.
- Deterministic regression tests for:
  - bounds/in-range/out-of-range behavior,
  - read-only and mutating proof outcomes under strict mode,
  - performance guardrail checks for list fixtures.
- Performance evidence publication:
  - `docs/evidence/phase-26.3-list-performance.{md,json}`.

## Exit Criteria
Gate D is complete when:
1. Required list API rows for this phase are implemented with deterministic typing/runtime behavior in the prescribed order.
2. List-related proof rows required by this phase are `proved` with zero assumption boundaries in strict release workflow.
3. Coverage matrices and CI gates enforce drift detection for list API/proof status.
4. Deterministic out-of-range behavior and compatibility surfaces remain conformant.
5. Performance guardrails remain green under pinned deterministic inputs.
