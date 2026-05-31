# Phase 26.6.8 - `can_mut` Semantics Lock

## Scope
Resolve long-term `can_mut` semantics for collection mut-guard APIs across `std::list`, `std::set`, and `std::map`.

## Locked Model
Permanent policy model is selected and locked.
- `can_mut` is a compatibility/policy guard predicate only.
- `_mut` APIs are guarded by typing/effect policy and deterministic diagnostics.
- `can_mut` does not imply ownership, uniqueness, alias-freedom, or borrow-style exclusivity.
- Runtime/lowering keeps guard calls deterministic and non-owning; ownership reasoning remains in linear ownership-transfer APIs (`remove_take`/`insert_take` families) and linear VC flows.

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
1. One model is locked with no ambiguity. (`permanent policy model`)
2. Conformance evidence is green for list/set/map guarded mut APIs.
3. Documentation, diagnostics, and runtime behavior are consistent.

## Evidence Snapshot
- Typing/effect diagnostics remain deterministic (`T401`/`T402`/`T403`) and enforce guard presence + variable-target guards.
- Guarded mut API VC obligations are emitted for list/set/map `_mut` surfaces.
- CLI std import metadata includes full `std::unit` assert surface, reducing cross-stage drift risk while Gate G cleanup continues.
