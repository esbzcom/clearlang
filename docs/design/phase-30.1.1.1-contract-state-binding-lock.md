# Phase 30.1.1.1 - Contract State Binding and Lowering Lock

Date: 2026-08-05
Status: Locked
Owner: language-and-proof-owner

## Purpose

Define the bridge from a declared state schema to executable and provable contract transitions.
The schema-only implementation does not make `state.<field>` a valid expression or give a
function a pre/post-state boundary. This bridge must land before `old(...)`, invariants, or target
storage adapters are implemented.

## Source and Ownership Model

1. A deployable contract owns its state declaration, initializer, invariants, and transition
   functions. Contract members are parsed as members of that contract, not as unrelated top-level
   declarations.
2. `state` is a reserved implicit value only inside a contract member. It cannot be shadowed,
   passed as a normal value, captured by a closure, exported, or referenced by ordinary module
   functions.
3. `state.<field>` is resolved against the contract's declared fields. Unknown fields and a bare
   `state` value are deterministic type errors.
4. A `pure` contract function has an explicit read-only state capability. A `mut` transition has
   read/write state capability. An `io` function and all non-contract functions have none for this
   milestone slice.
5. A write target must be rooted at `state.<declared-field>`. Assignments to local variables,
   parameters, or a field reached through a non-state value remain unsupported.

## Typed Transition Representation

The AST records state field access and state assignment explicitly rather than treating either as
a normal variable or a generic field mutation. Type checking carries a `ContractStateContext`
containing the contract identifier, ordered field table, and access mode. This avoids accidental
visibility through a global name and gives diagnostics the exact field span.

The lowering boundary carries target-neutral operations:

- `state_read(contract_id, field_id)`;
- `state_write(contract_id, field_id, value)`; and
- an explicit transition entry snapshot token for proof generation.

Field identifiers are the canonical identifiers emitted in the state-schema artifact. Physical
storage keys, serialization, and EVM host calls remain the responsibility of the target adapter in
30.4. Until that adapter exists, executable target builds that contain state operations fail
closed with a dedicated unsupported-target diagnostic; they must never silently lower to local
variables or no-ops.

## Proof Boundary

Each public `mut` transition receives one immutable entry-state snapshot before its body executes.
`require` is evaluated against that entry state; normal `state.<field>` references in `ensure` are
the exit state. The snapshot is not source-addressable until 30.1.2 introduces `old(...)`, but its
identity is carried through the typed and VC representations now.

## Non-Goals

This lock does not add `old(...)`, invariants, constructor definite-initialization, migration
execution, collection-rooted mutation, target storage serialization, or an EVM adapter. It only
adds the necessary state-binding and lowering boundary for those features.

## Exit Criteria

1. Contract members own their executable functions and use an explicit state context.
2. Reads and scalar writes of declared state fields are parsed and type checked with capability
   enforcement.
3. Lowering contains deterministic target-neutral state operations and transition snapshot
   identity.
4. Backends without a state adapter reject stateful lowering fail-closed.
5. Parser, typer, VC, lowering, and rejection tests cover the boundary.

## References

- `docs/design/phase-30.1.0-state-schema-and-migration-lock.md`
- `docs/todo/milestone_4_roadmap.md`
