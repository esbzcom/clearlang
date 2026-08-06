# Phase 30.1.1.2 - State Transition VC and IR Lock

Date: 2026-08-05
Status: Locked
Owner: language-and-proof-owner

## Purpose

Define the target-neutral transition boundary required to prove stateful contracts. Source-level
state reads and writes exist, but neither may be treated as an ordinary local variable in a proof
or backend. This lock establishes distinct entry and exit state identities before `old(...)` is
introduced.

## Transition Artifact

Each public `mut` contract function has one transition artifact with these canonical fields:

1. contract identifier, source schema version, and canonical state-layout digest;
2. transition function identifier and source digest;
3. declaration-ordered state fields, each with its stable schema field identifier and type;
4. entry symbols `clg.state.pre.<field-id>` and exit symbols `clg.state.post.<field-id>`;
5. ordered state read/write events, including source spans and field identifiers; and
6. an explicit status showing whether a target adapter and solver encoding support the operation.

The artifact uses field identifiers rather than field names for bindings. Renaming source
presentation cannot alter the proof/storage identity.

## Semantics

- A transition begins with `pre` and an unconstrained `post` state that is refined by each ordered
  write. A field with no write has `post(field) = pre(field)`.
- `state.<field>` in a transition body and postcondition denotes the current state at that program
  point; after the final write it denotes `post(field)`.
- Preconditions use `pre` symbols. `old(...)`, added in 30.1.2, is restricted to pure expressions
  rooted in these same `pre` symbols.
- IR represents `state_read(contract-id, field-id)` and `state_write(contract-id, field-id,
  value)` directly. A backend without a state adapter rejects either operation before emission.

## Release and Fail-Closed Policy

Until both the VC encoder and selected target adapter implement these operations, a stateful
transition may be parsed and type checked for diagnostics but cannot obtain a proved VC, strict
release result, or executable target artifact. There is no fallback to host memory, a local
variable, or an uninterpreted proof marked proved.

## Exit Criteria

1. VC and emitted proof artifacts carry deterministic pre/post symbols and ordered state events.
2. Solver encoding proves frame conditions for untouched scalar fields.
3. IR carries target-neutral state read/write operations.
4. Backends lacking the selected adapter reject state operations deterministically.
5. Tests cover deterministic artifact ordering, frame conditions, and release rejection.

## References

- `docs/design/phase-30.1.0-state-schema-and-migration-lock.md`
- `docs/design/phase-30.1.1.1-contract-state-binding-lock.md`
- `docs/todo/milestone_4_roadmap.md`
