# Phase 30.3.0 - Minimal Target Profile and State Solver Lock

Date: 2026-08-08
Status: Locked
Owner: target-runtime-owner

## Purpose

Define the smallest executable target slice needed to close Contract Gate A without claiming
deployment, RPC, ABI transport, or arbitrary EVM execution. The slice makes the existing
target-neutral state IR meaningful to the VC encoder and solver.

## Profile

The profile identifier is `clg.contract-state-solver.v1`. It consumes only declaration-ordered
`StateRead`, `StateWrite`, and `EventEmit` operations for a single contract transition.

1. Each state field has one entry (`pre`) and one exit (`post`) solver symbol, named from its
   canonical state-field identifier.
2. A scalar write updates the corresponding exit symbol in source order. Fields not written are
   constrained to retain their entry value.
3. `Bool`, signed/unsigned integer, string, and bytes scalar fields use the existing canonical
   SMT sort mapping. Composite physical-layout encoding remains a later target-adapter task.
4. Event emissions are ordered, typed transition receipts. They do not mutate solver state and
   therefore do not change a state-invariant formula.
5. Branching state transitions, loops that mutate state, and `ExternalCall` are outside this
   minimal profile. They retain an explicit assumption boundary and cannot support a strict
   production proof claim.

## Solver and Counterexample Contract

1. A state transition VC supplies the transition relation as a premise and proves frame
   conditions plus each declared exit invariant from that relation.
2. Failed VCs request and record the deterministic solver model in the existing
   `clg.counterexample.v1` proof artifact. The model is evidence, not an executable receipt.
3. The adapter removes the former unresolved state-transition assumption only for fully modelled
   straight-line scalar transitions and migrations.
4. Any unsupported transition shape, unavailable solver, unknown result, timeout, or malformed
   model remains fail-closed for strict builds.

## Non-Goals

This lock does not enable Wasm state execution, EVM bytecode generation, deployment, network
access, ABI calls, storage collections, or external-call execution. Backends continue to reject
unadapted state/event/external-call IR deterministically.

## Acceptance Evidence

1. State-invariant VCs prove a valid scalar transition and fail with a solver model for an
   invalid one.
2. Migration-invariant VCs use the same entry/exit relation.
3. Proof artifacts retain deterministic counterexample bytes across identical runs.
4. Unsupported transition flow and external calls retain an explicit release-blocking boundary.

## References

- `docs/design/phase-30.1.0-state-schema-and-migration-lock.md`
- `docs/design/phase-30.1.1.2-state-transition-vc-ir-lock.md`
- `docs/todo/milestone_4_roadmap.md`
