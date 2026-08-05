# Phase 30.0.0 - Milestone 4 Production Contract Platform Lock

Date: 2026-08-05
Status: Locked
Owner: product-and-runtime-owner

## Purpose

Milestone 4 turns ClearLang's proved-Wasm and distribution foundation into a usable production
contract platform. The milestone is intentionally narrow: it closes one end-to-end contract
workflow on one chain target before any multi-chain, web, or mobile platform expansion.

## Product Decision

The first supported target is EVM-compatible chains. Milestone 4 must enable a developer to
author a stateful ClearLang contract, prove its supported business invariants, test it against a
deterministic EVM-compatible environment, package the target artifact and ABI, deploy or invoke
it through an explicit target adapter, and verify its release evidence.

The production claim is limited to the locked target profile and the proof-covered contract
surface. No result in this milestone claims universal cryptographic, side-channel, or arbitrary
chain-runtime correctness.

## Design Principles Check

- Simple for users: one target profile and one primary workflow (`clg contract init|test|release`)
  avoid asking developers to compose raw Wasm, ABI, host-profile, and package commands.
- AI-friendly: contract state schemas, ABI artifacts, diagnostics, simulator traces, and proof
  evidence must be versioned, deterministic, and machine-readable.
- Provably correct: state transitions, storage schemas, snapshots, external-call boundaries, and
  release-enabled crypto assumptions must be explicit; unsupported assumptions fail closed.
- Crypto-focused: target integration prioritizes authorization, value/accounting invariants,
  deterministic signing/verification, event evidence, and supply-chain integrity.

## Required Product Capabilities

1. Stateful contract semantics
   - persistent typed storage with a versioned schema;
   - `old(...)` snapshots in contract postconditions;
   - state-transition and storage-invariant VCs;
   - explicit event emission and deterministic event ABI projection.
2. External-call safety
   - a typed external-call capability distinct from local state mutation;
   - an enforceable checks-effects-interactions rule or an equally strong documented alternative;
   - reentrancy-safe state-transition rules with deterministic diagnostics.
3. EVM-compatible target adapter
   - contract ABI generation, deploy/call encoding, revert/error decoding, event decoding, and
     target configuration;
   - deterministic local simulation with storage, caller, value, block context, gas/resource
     limits, and trace artifacts;
   - explicit compatibility/version policy for supported EVM environments.
4. Release-grade crypto policy
   - select the minimal target crypto surface required for the first contract profile;
   - either prove the selected semantic properties without hidden assumptions or require a
     separately verified, versioned attestation boundary that is visible in release evidence;
   - keep unsupported cryptographic and side-channel claims blocked from theorem-grade release.
5. Operational and developer readiness
   - bounded runtime memory/lifecycle policy suitable for long-running simulator and host use;
   - target-aware property/fuzz testing, source locations/stack traces, and an editor integration
     that consumes the existing structured CLI contract;
   - deployment, rollback, incident, and key-management runbooks.

## Non-Goals

Milestone 4 does not authorize:

1. a second production chain target or a generic multi-chain abstraction;
2. web, mobile, UI, HTTP, database, or general application frameworks;
3. a default switch from embedded to shared std delivery;
4. universal proofs of cryptographic hardness, constant-time behavior, or target-runtime
   implementation correctness;
5. broad language-feature expansion unrelated to the contract workflow.

## Release Rules

1. Existing Milestone 3 release and provenance gates remain mandatory.
2. Contract release is fail closed for unsupported target profiles, unversioned state schemas,
   unresolved storage/external-call obligations, invalid ABI artifacts, or unlabelled crypto
   assumptions.
3. Only the exact EVM-compatible profile and explicitly proved/attested contract surfaces may
   carry the Milestone 4 production-contract claim.
4. Simulator success alone is never deployment assurance; the release bundle must identify the
   target profile, ABI, state-schema version, proof/attestation evidence, and toolchain version.

## Exit Criterion

Milestone 4 is complete only when a non-trivial stateful reference contract can be created,
proved within the supported claim boundary, property/fuzz tested, simulated, released with ABI
and evidence, deployed/invoked on the supported EVM-compatible target, and independently
verified from its release bundle. The complete workflow must be covered by deterministic CI and
documented for developers and operators.

## References

- `README.md`
- `docs/todo/milestone_3_roadmap.md`
- `docs/design/phase-25.0.2-theorem-grade-certification-policy.md`
- `docs/design/phase-25.1.19-crypto-release-closure.md`
- `docs/design/phase-29.15.0-delivery-mode-simplification-decision-lock.md`
