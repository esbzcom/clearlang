# Phase 26.1.4.13 - `std::map` First-Production API Lock

## Scope
This lock defines the first-production `std::map` surface for deterministic key lookup/mutation behavior.

## First-Production API (Release-Enabled)
- `std::map::{new,len,is_empty,contains,get,insert,insert_take,remove,remove_take}`
- `std::map::{can_mut,insert_mut,remove_mut}` (current-usage compatibility, guarded mutable path)

## Determinism and Safety Contracts
- `contains`/`get` MUST be deterministic for identical logical map content.
- `insert` MUST deterministically replace existing value when key already exists.
- `insert_take`/`remove_take` MUST preserve deterministic returned prior-value semantics.
- `remove` MUST preserve deterministic no-op behavior for absent keys.
- `*_mut` paths MUST enforce `can_mut` precondition behavior with deterministic diagnostics.
- Current `can_mut` semantics are compatibility-oriented (guard predicate) rather than ownership/uniqueness proof; production claims MUST NOT imply stronger aliasing guarantees until the dedicated `can_mut` semantics lock lands.

## Evolution Contracts
- Any future map iteration/ordering APIs MUST declare canonical ordering rules before release enablement.
- Future batched mutation APIs must be additive and preserve existing key equality semantics.

## Gate B Exit for `std::map`
`std::map` slice is complete when:
1. API/doc/metadata surface is consistent with this lock.
2. `typed|runtime|proved` statuses are recorded in `docs/std/coverage-matrix.md`.
3. CI conformance covers `contains/get/insert/remove` and take/mut variants with stable behavior.
4. Release profile fails closed for any deferred map additions.
