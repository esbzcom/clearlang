# Phase 26.2.0 - Finite-Set Proof Gate (Std Gate C) Design Lock

## Scope
Define the ordered execution and machine-checkable completion criteria for finite-set proof support in Milestone 26.

## Policy Decisions
1. `std::set::subset` is deferred from first-production baseline and is owned by `26.2.1`.
2. Set-algebra APIs (`subset`, `union`, `intersect`, `diff`) are release-disabled until their corresponding Gate C rows are implemented and gated.
3. Theorem-grade claims over set properties require zero assumption boundaries on set-related VCs.

## Assumption-Boundary Policy
- Gate C does not introduce a new permanent release assumption ID for finite-set semantics.
- Temporary development assumptions for set reasoning are allowed only in non-release workflows and MUST be blocked by strict release gates.
- Release/theorem-grade path for set surfaces MUST satisfy:
  - `assumptions.items == []` on set-related VCs,
  - strict compiler mode acceptance (no `C033`),
  - verifier assurance acceptance for required policy.

## Ordered Execution (Must Follow)
1. `26.2.1` subset API implementation (`typed|runtime`) with deterministic behavior/tests.
2. `26.2.2` subset + membership VC/SMT proof reasoning and strict no-assumption gate wiring.
3. `26.2.4` set algebra proof support (`union`, `intersect`, `diff`) after subset is stable.
4. `26.2.5` cardinality-heavy reasoning (`len`, bounds, size relations) last.

## Deterministic Performance Guardrails (26.2.5)
- Guardrail profile input is pinned and versioned (same solver profile as release proof gates).
- CI gate budgets for the finite-set suite:
  - median wall time regression limit: `<= +10%` vs pinned baseline,
  - p95 wall time regression limit: `<= +20%` vs pinned baseline,
  - timeout count: `0` for release-enabled finite-set proof fixtures.
- Any budget breach fails the Gate C performance check.

## Required Evidence Artifacts
- Coverage status updates in:
  - `docs/std/coverage-matrix.md` (`typed|runtime|proved` rows for `std::set` symbols),
  - `docs/proofs/proof-coverage-matrix.{md,json}` (set-related proof surfaces).
- Deterministic regression tests for:
  - subset positive/negative semantic behavior,
  - subset/algebra proof outcomes under strict mode,
  - performance guardrail checks for finite-set fixtures.

## Exit Criteria
Gate C is complete when:
1. `std::set::subset` and approved set-algebra rows are implemented with deterministic typing/runtime behavior.
2. Set-related proof rows required by this phase are `proved` with zero assumption boundaries in strict release workflow.
3. Coverage matrices and CI gates enforce drift detection for set API/proof status.
4. Performance guardrails remain green under pinned deterministic inputs.

## 26.2.2 Evidence Snapshot
- Implemented proof evidence for subset/membership closure:
  - `crates/typer/tests/vc/refinements_and_assumptions.rs` (`set_subset_membership_vcs_carry_no_assumptions`)
  - `crates/cli/tests/cli_it/vc_outputs/assumption_tests.rs` (`build_set_subset_membership_vcs_have_zero_assumptions`)
- Ordered rollout fail-closed evidence:
  - `crates/typer/tests/collections_typing.rs` previously enforced deterministic fail-closed blocking for `std::set::{union,intersect,diff}` until Gate C expansion, then transitioned to typed mismatch conformance once `26.2.4` landed.
- `26.2.4` set-algebra closure evidence:
  - `crates/typer/src/check/expr/calls/collections.rs` and `crates/typer/src/lower/collections_set.rs` implement `std::set::{union,intersect,diff}`.
  - `crates/codegen-wasm/tests/collections_runtime/set_ops.rs` covers deterministic runtime behavior for each operator.
  - `crates/typer/tests/vc/refinements_and_assumptions.rs` and `crates/cli/tests/cli_it/vc_outputs/assumption_tests.rs` assert assumption-free VC behavior.
- `26.2.5` cardinality/perf closure evidence:
  - `crates/typer/tests/vc/refinements_and_assumptions.rs` and `crates/cli/tests/cli_it/vc_outputs/assumption_tests.rs` add `std::set::len` cardinality VC coverage with zero assumptions.
  - `docs/evidence/phase-26.2-finite-set-performance.{md,json}` publish deterministic guardrail thresholds and release-enabled fixture set.
  - `crates/cli/tests/phase26_finite_set_performance_guardrails.rs` enforces guardrail contract drift checks.
- Coverage updates:
  - `docs/proofs/proof-coverage-matrix.{md,json}` marks `feature.finite_set_membership_subset_reasoning` and `intrinsic.std::set::{contains,subset}` as `proved`.
  - `docs/std/coverage-matrix.md` marks `std::set::{contains,subset}` as `proved=yes`.
