# Phase 30.1.1.3.2 - Constructor Simulator Lifecycle Lock

Date: 2026-08-09
Status: Locked
Owner: language-and-proof-owner

## Purpose

Bind the already checked `init(...)` source declaration to the deterministic local simulator
without broadening it into deployment or general target execution.

## Lifecycle Contract

1. A constructor lowers to a private deterministic IR entrypoint associated with exactly one
   declared contract; it is never callable from ClearLang source.
2. `clg simulate --init` selects that entrypoint and requires an empty JSON state object (`{}`).
   Constructor arguments remain an explicit JSON array.
3. On success the simulator validates that every declared state field is present and scalar before
   it writes the initialized state output. Its trace uses `lifecycle: "init"` and has no public
   return value.
4. Any non-empty input state rejects a second initialization before execution and without a state
   output. This is the deterministic local exactly-once boundary; it is not a deployment receipt
   or durable chain registry.
5. Existing `T829` direct-write validation remains the all-path boundary for this narrow target
   profile: nested/control-flow initialization is rejected rather than being speculatively
   simulated. Outbound calls remain rejected by `T830` before lowering.

## Non-Goals

This lock does not add branch/loop execution, constructor re-entry handling, durable deployment
identity, network deployment, or constructor wire encoding. Those require the later target
adapter and receipt work.

## Acceptance Evidence

1. A checked constructor lowers to its private target-neutral entrypoint.
2. Simulation from `{}` writes a fully initialized scalar state and canonical lifecycle trace.
3. Reusing initialized state with `--init` fails before output state is committed.

## References

- `docs/design/phase-30.1.0-state-schema-and-migration-lock.md`
- `docs/design/phase-30.3.2-deterministic-local-simulator-lock.md`
- `docs/todo/milestone_4_roadmap.md`
