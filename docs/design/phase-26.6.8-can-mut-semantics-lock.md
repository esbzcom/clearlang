# Phase 26.6.8 - `can_mut` Semantics Lock

## Scope
Resolve long-term `can_mut` semantics for collection mut-guard APIs across `std::list`, `std::set`, and `std::map`.

## Current State
- `can_mut` is currently a compatibility-oriented guard predicate.
- `_mut` APIs are guarded by typing/effect policy and deterministic diagnostics.
- Current behavior does not prove ownership/uniqueness semantics.

## Decision Requirement
Choose one explicit model and lock it:
1. Ownership/uniqueness model: implement and enforce uniqueness-aware mutability semantics.
2. Permanent policy model: keep `can_mut` as a policy guard and explicitly disallow stronger aliasing claims.

## Required Deliverables
- Normative semantics doc update in `docs/typing.md`.
- Collection docs update in `docs/std/collections.md`.
- Coverage matrix notes update in `docs/std/coverage-matrix.md`.
- Conformance tests for all guarded mut APIs:
  - `std::list::{can_mut,push_mut,insert_mut,remove_mut,pop_mut}`
  - `std::set::{can_mut,insert_mut,remove_mut}`
  - `std::map::{can_mut,insert_mut,remove_mut}`
- Migration/deprecation notes if user-visible behavior changes.

## Fail-Closed Rules
- If semantics remain ambiguous across docs/typing/runtime, fail closed.
- If `_mut` guard diagnostics diverge from locked semantics, fail closed.
- If release claims imply ownership guarantees without proof/evidence, fail closed.

## Exit Criteria
1. One model is locked with no ambiguity.
2. Conformance evidence is green for list/set/map guarded mut APIs.
3. Documentation, diagnostics, and runtime behavior are consistent.
