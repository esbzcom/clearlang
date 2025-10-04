# Phase 8.2 – Linear Typing Rules

## Status
- Draft design note created 2025-10-04; implementation pending.

## Goals
- Enforce single-consume semantics for resources: every value of a resource type must be consumed exactly once or explicitly dropped.
- Support immutable borrows of active resources so read-only access remains ergonomic without violating linearity.
- Produce actionable diagnostics for resource misuse (duplicate consume, use-after-consume, borrow-after-consume).
- Lay groundwork for future phases (aliasing, collections, proofs) without locking in premature runtime behavior.

## Non-Goals
- Implementing mutable borrows or aliasing strategies beyond immutable borrows.
- Allowing resources inside generic containers (tracked in Phase 8.3).
- Emitting final Wasm-level resource runtime hooks—focus remains on typer guarantees.

## Typing Model Overview
1. **Resource Types**
   - Each `resource Foo { ... }` declaration introduces a nominal type `Foo`.
   - Function parameters and locals can reference `Foo`; parameters may be marked `consume` for ownership transfer or default to `borrow`.

2. **Ownership States**
   - Track a `ResourceState` per binding:
     - `ActiveBorrow` – value is live and only immutably borrowed.
     - `Owned` – binding currently owns the value and must consume or drop it.
     - `Consumed` – value is no longer accessible; further use triggers diagnostics.
   - Borrowing an `Owned` value transitions the borrower into `ActiveBorrow` while the owner remains `Owned`; further consumes blocked until borrow scope ends (Phase 8.2 limits scope to block-local immutability).

3. **Operations**
   - **Consume** (`consume` parameters, explicit `drop`, or future APIs) transitions bindings to `Consumed`.
   - **Drop Blocks** execute block bodies that may reference fields; upon exit the resource moves to `Consumed`.
   - **Return** / tail expressions must account for ownership: returning a resource transfers ownership; missing consume/drop is an error.

4. **Diagnostics**
   - `R001`: resource used after consume/drop.
   - `R002`: resource consumed twice.
   - `R003`: borrow after consume in same scope.
   - Include spans for both offending usage and original consume site.

## Block Expressions
- Promote existing `Block` AST blocks to first-class expressions.
- Type checker evaluates statements sequentially, threading environments and ownership maps.
- Drop blocks share the same typing pipeline.
- Return statements end the block early; ensure the surrounding context handles the returned ownership state.

## Implementation Plan
1. **AST & Parser Support**
   - Add `Type::Resource(String)`; parse bare identifiers in `ty_p` as resource references (after keywords/constructors).
   - Include resource types in pretty printer/tests.

2. **Block-Aware Typer**
   - Extend `Expr` with `Block` variant or reuse existing structures with a helper function (e.g., `type_of_block`).
   - Introduce an ownership environment (e.g., `HashMap<&str, ResourceState>`), initialised from parameters (respecting `ParamKind`).
   - Ensure ensure/require contracts reuse borrow semantics.

3. **Diagnostics & Tests**
   - Add unit tests covering consume success, double consume, use-after-consume.
   - Verify drop blocks run to completion and mark owners consumed.

4. **Follow-up Hooks**
   - Phase 8.3 will forbid resource storage in standard collections and design alias-safe containers.
   - Later phases may introduce mutable borrows and formal proofs linking resource states to verification conditions.

## Open Questions
- How to surface borrow scope boundaries in the absence of lexical lifetimes? (Likely block-local enforcement for Phase 8.2.)
- Should `drop` blocks be allowed to move fields out? Initial plan is to forbid consumes inside the drop block; revisit once linear states stabilize.
- Interaction with contracts: ensure `require`/`ensure` expressions cannot consume resources.
