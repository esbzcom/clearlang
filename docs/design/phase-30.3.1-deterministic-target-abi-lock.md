# Phase 30.3.1 - Deterministic Target ABI Lock

Date: 2026-08-09
Status: Locked
Owner: target-runtime-owner

## Purpose

Define the first target-facing contract ABI artifact without claiming EVM bytecode, selector
derivation, calldata encoding, deployment, or network compatibility. The artifact is a canonical
descriptor consumed by the simulator, deploy adapter, and release bundle in later phases.

## Artifact Contract

`clg build --emit-contract-abi <FILE>` accepts exactly one contract and emits canonical JSON with
format `clg.contract-abi.v1`. It contains:

1. compiler identity and the complete loaded source-graph identity;
2. contract name and source version;
3. declaration-ordered function descriptors, each with a domain-separated SHA-256 identifier,
   effect, canonical signature, parameters, and return type;
4. declaration-ordered event descriptors and the canonical event-ABI digest already bound by the
   state-schema artifact;
5. an explicit error surface. It is an empty ordered list with status `unsupported` until the
   language has canonical contract-error declarations;
6. the canonical state-schema, layout, and event-ABI digests; and
7. target metadata identifying the descriptor profile and every deferred wire/runtime boundary.

Function identifiers are `sha256` digests over the ABI domain, contract name, source version,
and canonical function signature. Function declaration order is preserved for tooling; identity
does not depend on that order.

## Target Boundary

The ABI descriptor profile is `clg.evm-compatible.abi.v1`. It is target-facing metadata, not an
assertion that the artifact can yet be deployed to an EVM-compatible environment.

EVM method selectors, event topics, ABI tuple encoding, revert/error decoding, value transfer,
and calldata are deliberately omitted. They depend on the target crypto and transport decisions
that remain outside this lock. The descriptor marks those boundaries as `deferred` so no caller
can mistake it for executable wire ABI.

## Fail-Closed Rules

- ABI emission fails unless the loaded program has exactly one contract.
- The ABI source graph, compiler identity, and referenced schema/layout/event digests are all
  canonical JSON inputs. A later simulator, target receipt, or release bundle must reject drift.
- No build may label this descriptor as deployed, wire-compatible, or release-grade by itself.

## Acceptance Evidence

1. Repeated emission for identical sources is byte-identical.
2. Function and event declaration order is preserved.
3. Changing any loaded source file changes the artifact source-graph digest.
4. The artifact links to the same state-schema, layout, and event-ABI identities as the canonical
   state-schema artifact.

## References

- `docs/design/phase-30.3.0-target-profile-and-state-solver-lock.md`
- `docs/design/phase-30.1.0-state-schema-and-migration-lock.md`
- `docs/todo/milestone_4_roadmap.md`
