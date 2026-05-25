# Phase 26.1.4.12 - `std::set` First-Production API Lock

## Scope
This lock defines the first-production `std::set` surface for deterministic membership and mutation semantics.

## First-Production API (Release-Enabled)
- `std::set::{new,len,is_empty,contains,insert,remove}`
- `std::set::{can_mut,insert_mut,remove_mut}` (current-usage compatibility, guarded mutable path)

## Determinism Contracts
- `contains` MUST be deterministic for identical logical set content.
- `insert` MUST be idempotent for existing elements and deterministic for new elements.
- `remove` MUST preserve deterministic no-op behavior when value is absent.
- `*_mut` paths MUST enforce `can_mut` precondition behavior with deterministic diagnostics.

## Deferred (Non-Release) Symbols
- `std::set::{subset,union,intersect,diff}` remain deferred until Gate C ordered proof rollout enables them.

## Gate B Exit for `std::set`
`std::set` slice is complete when:
1. API/doc/metadata surface is consistent with this lock.
2. `typed|runtime|proved` statuses are recorded in `docs/std/coverage-matrix.md`.
3. CI conformance covers mutation and non-mutation paths for `contains/insert/remove`.
4. Release profile fails closed for deferred set-algebra symbols.
