# Phase 30.5.0 - Contract Runtime Lifecycle Limits

Date: 2026-08-09
Status: Locked
Owner: developer-experience-owner

## Policy

The supported local contract path is bounded and observable. The scalar simulator has an explicit
fuel limit and a per-input/per-output JSON byte limit (`--memory-limit`, default 1 MiB). A limit
failure is deterministic, writes no state output, and reports a failure trace once execution has
started.

The current simulator profile rejects closures, dynamic collections, memory-backed values, and
user-function dispatch before execution. Therefore it creates no unbounded contract heap or
closure lifecycle in the supported profile.

Wasm execution remains a separate application-runtime path. Its existing runtime-limit diagnostic
is not contract target evidence and cannot be used to bypass simulator or target-adapter limits.

## Observability and Non-Goals

Fuel limits are recorded in simulator traces. JSON limit failures name the bounded input/output.
No claim is made about a general EVM memory model, garbage collection, or target gas equivalence.

## References

- `docs/design/phase-30.3.2-deterministic-local-simulator-lock.md`
- `docs/todo/milestone_4_roadmap.md`
