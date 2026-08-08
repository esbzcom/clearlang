# Phase 30.2.0 - External-Call and Reentrancy Capability Lock

Date: 2026-08-08
Status: Locked
Owner: language-security-owner

## Purpose

Define the only supported future boundary for an outbound contract call before syntax, lowering,
or a target adapter is introduced. An ordinary `io` call, imported function, host capability, or
function value must never silently become an external contract call.

## Capability Model

1. Outbound calls require a dedicated `external_call` capability, available only to a `mut`
   function owned by a contract.
2. The capability is non-transitive: a caller must use the explicit outbound-call form introduced
   by 30.2.1. A local function call does not inherit it merely because its caller has it.
3. The first supported call target is a statically named, ABI-declared interface method with a
   canonical argument and result schema. Dynamic targets, callbacks, function values, fallback
   dispatch, delegate calls, and raw ABI bytes are unsupported.
4. An external call is a target-neutral IR operation with an explicit call-site identifier,
   callee ABI identity, value-transfer field, and result/revert channel. Backends without an
   adapter must reject it before generating an executable artifact.

## State and Reentrancy Protocol

The initial protocol is strict checks-effects-interactions (CEI):

1. All caller-visible preconditions and authorization checks occur before the first outbound
   call.
2. Every state write that establishes the transition's intended guard or balance effect occurs
   before the first outbound call.
3. No state read, write, event emission, or second outbound call may occur after an outbound
   call in the same transition.
4. The contract's declared storage invariants must hold at the call boundary and at transition
   exit. Until state/call target adapters have solver models, this is an explicit release-blocking
   assumption rather than a proved claim.
5. Reentrancy is prevented only for transitions accepted by this protocol. ClearLang makes no
   claim about arbitrary target bytecode, callbacks, proxies, or code that bypasses this model.

## Unsupported Patterns and Stable Diagnostics

The implementation reserves these deterministic typing diagnostics:

| Code | Condition |
|---|---|
| `T830` | External-call syntax or capability used outside a contract-owned `mut` transition. |
| `T831` | Dynamic/untyped target, raw payload, callback, delegate-call, or unsupported value transfer. |
| `T832` | State interaction, event emission, or additional outbound call after an outbound call. |
| `T833` | Call boundary cannot establish the declared invariant/transition proof obligation. |

Before 30.2.1 exists, every attempted external-call spelling is rejected as unsupported; ordinary
imports retain their existing local/package semantics and must not receive any of these effects.

## Evidence and Release Policy

30.2 completion requires deterministic negative fixtures for each diagnostic, adversarial
reentrancy fixtures, call-order proof artifacts, and target-adapter conformance results. Strict
release rejects any outbound-call operation with an unresolved adapter, solver assumption, or
unmodelled callback path.

## Exit Criteria

1. The source capability, static target model, and unsupported surface are unambiguous.
2. CEI ordering and invariant boundaries are defined before implementation.
3. Diagnostics and release-blocking conditions are stable enough for fixtures and automation.
4. 30.2.1 can introduce syntax without changing this security boundary.

## References

- `docs/todo/milestone_4_roadmap.md`
- `docs/design/phase-30.0.1-milestone-4-gate-governance-lock.md`
- `docs/design/phase-30.1.0-state-schema-and-migration-lock.md`
