# Phase 17.6 - Linear-Aware Collections

## Status
Design note covering 17.6.1.1 through 17.6.1.4.
Implementation tracking lives in `docs/TODO.md` under Phase 17.6.

## Goals
- Allow storing resource values in collections without violating linear typing.
- Keep ownership behavior explicit at collection boundaries (move in, move out, borrow-only reads).
- Preserve deterministic behavior across typer, lowering, runtime checks, and diagnostics.
- Keep the model small and AI-friendly so diagnostics and repair loops remain predictable.
- Reuse existing effect and proof workflows (`pure`/`mut`/`io`, VC emission) instead of inventing a parallel system.

## Design Principles Check
- Simple for users: one ownership story for resources, including when nested in collections.
- AI-friendly: explicit, stable rules for moves/borrows and deterministic diagnostics.
- Provably correct: linear invariants are enforced by default and represented in typing/VC obligations.
- Crypto-focused: no implicit duplication or loss of resource-backed assets.

## Linear Ownership Guideline
- Global rule: any type that transitively contains a `Resource` value is linear-owned.
- This includes wrappers and composites such as `Option<R>`, `Result<R,E>`, `List<R>`, `Map<K,R>`, and tuples like `(List<R>, Option<R>)`.
- Ownership transfer stays explicit at API boundaries (`push`/`insert` move-in, `*_take` move-out).
- Phase-gated unsupported forms are still rejected with `T806` (currently `Set<Resource>` and array/slice forms containing resources).

## Non-Goals
- Hash-table performance work or collection API expansion.
- Mutable borrow/lifetime inference beyond current linear model.
- Garbage collection or deallocation redesign.
- General alias analysis for non-resource values.
- Closure/capture interactions (Phase 17.7).
- Trait-surface redesign (Phase 17.8 follow-ups).

## Safety Invariants
1. Resource values are never implicitly copied.
2. Every resource value has exactly one owner at each program point.
3. Inserting a resource into a collection transfers ownership into that collection.
4. Extracting a resource from a collection transfers ownership to the result binding and removes ownership from the collection slot.
5. Read-only queries over resource collections cannot produce owned resource aliases.
6. No operation may observe the same resource as owned in two places (binding + collection, or two collections).
7. Control-flow joins must preserve linear consistency for resource values inside and outside collections.
8. Effect gates remain sound: operations that mutate linear collection state are `mut`-only unless proven pure by construction.
9. Runtime trap behavior for invalid linear states must stay deterministic and mapped to stable diagnostics.

## 17.6.1.2 Ownership and Alias Rules

### Linear Container Classification
- `List<R>`, `Set<R>`, and `Map<K, R>` are linear containers when `R` is a resource type.
- A linear container binding is itself linear: it cannot be implicitly copied, duplicated, or dropped.
- Assignment/rebinding of linear containers follows the same ownership-state transitions as standalone resources.
- Tuple wrappers around linear values are linear as well; all owned outputs must be consumed.

### Operation Classes
- Move-in: consumes a resource argument and transfers ownership into the container.
- Borrow-read: observes container state without transferring resource ownership.
- Move-out: transfers ownership of a contained resource to the caller and updates container ownership.
- Iterate: repeated borrow-read over elements/values; iteration never duplicates or implicitly consumes resources.

### List<Resource>
- Insert (`push`/`insert`): move-in.
  - Input ownership: consumes `l: List<R>` and `x: R`.
  - Output ownership: returns a new owned `List<R>`.
- Get (`get`): borrow-read only.
  - Ownership transfer is forbidden.
  - Because the language currently has no first-class borrow return type, direct `get` for `List<Resource>` is gated in the prototype surface until borrow views are added.
- Remove (`remove`): move-out.
  - Must not drop removed resources.
  - Linear form returns updated collection plus extracted value, e.g. `remove_take(l, i) -> (List<R>, Option<R>)`.
- Iterate:
  - Iteration over `List<Resource>` is borrow-read only.
  - Consuming an iterated element is forbidden unless a dedicated draining API is used.

### Set<Resource>
- Phase 17.6 prototype keeps `Set<Resource>` disabled.
- Rationale:
  - Set operations require an equality/identity model for lookup/removal.
  - Resource values are non-copyable and currently non-equatable in the type system.
- Rule:
  - `Set<Resource>` construction and all related operations (`insert`, lookup/get/contains, `remove`, iterate) remain rejected until identity semantics are specified in a follow-up slice.

### Map<K, Resource>
- Constraint: `K` remains non-resource and equatable (existing `Map` key rules still apply).
- Insert (`insert`): move-in with explicit replacement ownership.
  - Input ownership: consumes `m: Map<K, R>` and `v: R`.
  - If key is new: returns updated map with no extracted value.
  - If key exists: old value ownership must be returned to caller, never dropped implicitly.
  - Linear form: `insert_take(m, k, v) -> (Map<K, R>, Option<R>)`.
- Get (`get`): borrow-read only.
  - No ownership transfer from map to caller.
  - As with lists, direct owned `get` is gated until borrow-return forms are available.
- Remove (`remove`): move-out.
  - Removal must return the removed resource ownership when present.
  - Linear form: `remove_take(m, k) -> (Map<K, R>, Option<R>)`.
- Iterate:
  - Iteration over map values is borrow-read only.
  - Key iteration stays value semantics; value iteration cannot clone or consume resources without explicit draining APIs.

### Alias and Control-Flow Rules
- A linear container cannot be consumed more than once (`T802`-class behavior).
- Using a consumed linear container is invalid (`T801`-class behavior).
- Borrow-read views block move-out/move-in operations on the same container in that scope (`T803`-class behavior).
- Branches must rejoin with identical ownership states for linear containers and extracted resources (`T804`-class behavior).

## 17.6.1.3 Consume/Borrow Boundary Semantics

### Function Boundary Rules
- Parameter mode remains explicit:
  - `consume c: List<R>` / `consume m: Map<K, R>` transfers ownership of the full container and all contained resources into the callee.
  - `c: List<R>` / `m: Map<K, R>` is borrow-only; callee may read/inspect but cannot move-out resources from that binding.
- Returning a linear container transfers ownership to the caller.
- Returning an extracted resource (`R` or `Option<R>`) transfers ownership of that resource to the caller.
- A borrowed linear container cannot escape as an owned return value.

### Collection Call Boundary Rules
- Move-in APIs consume the resource argument at the call site and consume/replace the container binding with the returned container.
  - Example shape: `l = std::list::push(l, x)` consumes old `l` and `x`, then binds new `l`.
- Move-out APIs consume the container argument and return both updated container ownership and extracted ownership.
  - Example shape: `(l2, out) = std::list::remove_take(l, i)`.
  - The extracted resource in `out` becomes caller-owned when `Some`.
- Borrow-read APIs do not transfer resource ownership out of the container and do not consume element/value ownership.
- Iteration over linear containers is borrow-only unless a dedicated draining API is used.
- Wrapper constructors/literals preserve linearity:
  - `Some(x)`, `Ok(x)`, `Err(x)`, and tuple literals move owned resource paths from inputs into the wrapper value.
  - Returning these wrappers counts as consuming the moved ownership paths.

### Forbidden Copy Paths
- Implicit copies are disallowed for:
  - linear containers (`List<R>`, `Map<K, R>`),
  - resources extracted from linear containers.
- Invalid paths include:
  - passing the same owned linear value into two consume positions,
  - using a value after it was consumed by a move-in or move-out call,
  - replacing a map entry without explicitly returning prior ownership,
  - materializing owned results from borrow-only reads.
- Branch joins that duplicate ownership or drop ownership without consume/drop are rejected.

### Boundary State Transitions (Canonical)
- Move-in: `(OwnedContainer, OwnedResource) -> OwnedContainer`
- Move-out: `OwnedContainer -> (OwnedContainer, Option<OwnedResource>)`
- Borrow-read: `BorrowContainer -> BorrowContainer` (no owned resource returned)
- Borrowed bindings can be read multiple times, but cannot be consumed while borrowed.

## 17.6.1.4 Deterministic Diagnostics and Trap Mapping

### Type-Check Boundary (Static)
Linear-aware collection diagnostics reuse the existing linear/resource family so tooling remains stable.

| Case | Stage | Code | Deterministic Rule |
| --- | --- | --- | --- |
| Use after move/consume (container or extracted resource) | type | T801 | Report at use site; include original consume span when available. |
| Double consume (same owner path consumed twice) | type | T802 | Report second consume site. |
| Consume while borrow is active | type | T803 | Report consume site and active borrow origin when available. |
| Ownership state diverges across branches/match arms | type | T804 | Report join point with conflicting branch spans. |
| Owned linear value reaches function/block end unconsumed | type | T805 | Report declaration or last owning binding span. |
| Unsupported container form for resources (current: `Set<Resource>` and array/slice forms containing resources) | type | T806 | Report type location/call site with explicit unsupported-kind message. |

Notes:
- For Phase 17.6, `T806` narrows from blanket rejection to unsupported forms only.
- `List<Resource>` and `Map<K, Resource>` errors should prefer `T801`-`T805` when the form is supported but ownership rules are violated.

### Runtime Boundary (Defensive)
Well-typed programs should not hit linear ownership traps at runtime. Runtime mapping is defensive for malformed handles or corrupted state crossing host/ABI boundaries.

| Case | Stage | Code | Deterministic Rule |
| --- | --- | --- | --- |
| Index/bounds violation during linear move-out (`remove_take`, etc.) | runtime | R009 | Same bounds semantics as existing collection runtime (`0 <= i < len` or API-specific bound). |
| Invalid linear collection handle/header/pointer metadata | runtime | R010 | Includes null/misaligned/out-of-range pointers and inconsistent header fields. |
| Unknown/unclassified runtime failure | runtime | R999 | Fallback only; should not be emitted in planned linear paths. |

Runtime detail policy:
- `R009` and `R010` remain stable and reusable across non-linear and linear collection operations.
- When runtime detail fields are present, they must be deterministic enums/labels (no nondeterministic strings).

### JSON Diagnostics Contract
- `--json-errors` output shape is unchanged.
- Linear collection diagnostics must keep:
  - stable `code` values,
  - stable `stage` (`type` or `runtime`),
  - source span on type errors and best-effort span on runtime errors.

## 17.6.3.1 VC Prototype (Control-Flow Linearity)
- VC generation now emits prototype control-flow obligations for ownership-sensitive collection operations:
  - `linear:branch:N`: symbolic branch-state agreement for tracked linear collection owners used in `if`/`match`.
  - `linear:loop:N`: symbolic loop-state preservation for tracked linear collection owners used in `while`.
- These VCs are additive and do not replace existing typer diagnostics (`T801`-`T805`); they provide a proof-facing hook for later 17.6.3.x strengthening.

## 17.6.3.2 Effect Gate Alignment + Proof/Runtime Split
- Effect gate policy for linear collections:
  - Ownership-transfer APIs (`std::list::push`, `std::list::insert`, `std::list::remove_take`, `std::map::insert_take`, `std::map::remove_take`) are treated as `pure` by construction.
  - `_mut` aliases remain `mut`-only and require existing `can_mut` guard obligations.
- Rationale:
  - Linear ownership transitions are enforced statically (resource tracker + `T801`-`T805`) and by linear VC obligations (`linear:branch:*`, `linear:loop:*`).
  - Runtime continues to execute the same deterministic collection helpers/traps (`R009`, `R010`) regardless of whether a call originated from a pure ownership API or a `_mut` alias.
- Proof/runtime split:
  - Proof layer: effect gating and VC obligations capture alias/ownership discipline.
  - Runtime layer: defensive memory/header/bounds checks enforce deterministic safety for malformed states.

## 17.6.3.3 Prototype Fixtures + Runtime Workflow Tests
- Added `--emit-vcs` fixtures for linear collection control-flow obligations:
  - `docs/proofs/fixtures/linear-collections-branch.vc.json`
  - `docs/proofs/fixtures/linear-collections-branch-inline.vc.json`
  - `docs/proofs/fixtures/linear-collections-loop.vc.json`
- Snapshot coverage is wired through `crates/cli/tests/vc_snapshots.rs`.
- Added representative runtime workflow tests (collection-helper semantics used by linear APIs):
  - `crates/codegen-wasm/tests/collections_runtime/list_ops.rs`
  - `crates/codegen-wasm/tests/collections_runtime/map_ops.rs`

## Scope Boundary for 17.6.1.x
- This document locks the safety contract, ownership/alias rules, consume/borrow boundary semantics, and deterministic diagnostics/trap mapping for linear containers.

## References
- `docs/design/phase-8.2-linear-typing.md`
- `docs/design/phase-17.3-collections-runtime.md`
- `docs/typing.md`
- `docs/resource-guide.md`
