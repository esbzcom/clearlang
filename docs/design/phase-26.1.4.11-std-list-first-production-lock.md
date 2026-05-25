# Phase 26.1.4.11 - `std::list` First-Production API Lock

## Scope
This lock defines the first-production `std::list` API cut, including deterministic bounds behavior and mutable-guard compatibility surfaces.

## First-Production API (Release-Enabled)
- `std::list::{new,len,is_empty,get,push,insert,insert_checked,remove,remove_checked,remove_take,pop}`
- `std::list::{can_mut,push_mut,insert_mut,remove_mut,pop_mut}` (current-usage compatibility, guarded mutable path)

## Determinism and Safety Contracts
- `insert`/`remove` MUST terminate deterministically on out-of-range indices via the standard panic/failure path.
- `insert_checked`/`remove_checked` MUST return deterministic error results and MUST NOT terminate.
- `get`/`remove_take`/`pop` behavior MUST be deterministic for empty and non-empty paths.
- `*_mut` operations MUST require `can_mut` guard contracts and preserve deterministic precondition diagnostics.

## Evolution Contracts
- Future ergonomic syntax (for example dot-call sugar) MUST compile to the same namespace calls.
- Any future in-place optimization must preserve current immutable-return semantics unless explicitly versioned.

## Gate B Exit for `std::list`
`std::list` slice is complete when:
1. API/doc/metadata surface is consistent with this lock.
2. `typed|runtime|proved` statuses are recorded in `docs/std/coverage-matrix.md`.
3. CI includes conformance tests for terminating (`insert/remove`) and non-terminating (`insert_checked/remove_checked`) out-of-range behavior.
4. Release profile fails closed for any deferred list additions.
