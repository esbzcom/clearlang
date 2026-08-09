# Phase 30.3.2 - Deterministic Local Simulator Lock

Date: 2026-08-09
Status: Locked
Owner: target-runtime-owner

## Purpose

Define a local contract simulator for the supported state-transition profile. It is a deterministic
target-model interpreter, not a fallback Wasm backend and not an EVM node.

## Command Contract

The simulator accepts one typed contract transition plus explicit JSON inputs for state and
arguments. Every execution records caller, value, block context, fuel limit, ordered state
changes, ordered event receipts, result or failure, and pre/post state in a versioned canonical
trace. The state and trace outputs are written explicitly; no ambient chain or filesystem state is
consulted after the supplied inputs are loaded.

## Initial Supported Profile

The initial interpreter executes declaration-ordered, straight-line scalar state transitions:

- `Bool`, signed integers, and in-range `U64` values;
- state reads/writes, scalar arithmetic/comparison/boolean operations, and event emission;
- explicit caller, value, block number, timestamp, and gas/fuel inputs recorded in the trace;
- deterministic instruction fuel accounting with failure before an instruction would exceed the
  supplied limit.

Control-flow state transitions, collections, memory-backed values, user-function dispatch,
external calls, target imports, and target-specific error/revert encoding fail closed. This mirrors
the current minimal state solver profile rather than extending its claim boundary.

## State and Trace Rules

1. Input state must be a JSON object containing every declared state field and no undeclared
   field. Values must match the initial scalar profile.
2. The simulator starts from a copy of the input state; a failed run emits a trace but never
   writes a state output.
3. Event receipt ordering follows IR instruction order. External calls are rejected before any
   target interaction occurs.
4. Trace format is `clg.contract-simulation-trace.v1`; canonical JSON bytes make equal inputs and
   toolchains reproduce equal traces.
5. Caller, value, and block context are evidence fields only until the language exposes those
   values to contract source under a separate lock.

## Non-Goals

This lock does not implement EVM bytecode, ABI calldata, RPC, deployment, concurrency, callbacks,
or generic contract execution. Simulator success is never a deployment or release claim.

## Acceptance Evidence

1. Identical source and inputs yield byte-identical state and trace outputs.
2. A scalar state write and event produce ordered, typed trace entries.
3. Missing/extra state fields, unsupported instructions, external calls, and fuel exhaustion fail
   deterministically without committing output state.

## References

- `docs/design/phase-30.3.0-target-profile-and-state-solver-lock.md`
- `docs/design/phase-30.3.1-deterministic-target-abi-lock.md`
- `docs/todo/milestone_4_roadmap.md`
