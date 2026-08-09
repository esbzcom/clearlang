# Phase 30.3.3 - Explicit Target Adapter and Receipt Lock

Date: 2026-08-09
Status: Locked
Owner: target-runtime-owner

## Purpose

Define the boundary for deploy, call, and invoke commands for the first EVM-compatible target.
The boundary prevents a local simulation, ABI descriptor, or ambient endpoint from being mistaken
for a submitted target transaction.

## Command Contract

The eventual adapter commands are `clg target deploy`, `clg target call`, and `clg target invoke`.
Each command requires all of the following explicit inputs:

1. target profile identifier (`clg.evm-compatible.v1`);
2. RPC endpoint URL and numeric chain identifier;
3. an input artifact whose target bytecode and ABI wire format are declared compatible with that
   profile;
4. for `call`/`invoke`, an explicit contract address and ABI function identifier;
5. for `invoke`, an explicit signing/sender configuration and value/gas policy.

No command reads a default RPC URL, chain ID, account, wallet, or network from the environment,
working directory, or a tool-specific configuration file. `call` never signs or submits; `invoke`
never silently simulates. An RPC response is accepted only after its requested chain identifier
matches the supplied identifier.

## Receipt Contract

Successful target work emits canonical `clg.target-receipt.v1` JSON. It records the command kind,
target profile, RPC endpoint identity (without credentials), chain ID, artifact and ABI digests,
contract/function identity, request digest, transaction or call identifier, block reference, and
result/revert status. A receipt is evidence of one explicit request, not a release attestation.

## Current Fail-Closed Boundary

The repository does not yet produce EVM bytecode or selector/calldata encoding: 30.3.1 deliberately
emits a descriptor rather than a wire ABI. Therefore no adapter command may be presented as
operational until both prerequisites are implemented and independently tested. Until then, any
target submission path must fail before network access and explain the missing bytecode/wire ABI
boundary. Local simulator traces cannot substitute for receipts.

## Acceptance Evidence

1. Every operational adapter invocation rejects omitted or ambiguous target/RPC/chain inputs.
2. Deterministic receipt bytes bind the request, target profile, ABI, and bytecode identities.
3. Chain-ID mismatch, missing bytecode/wire ABI, malformed receipt, failed RPC response, and
   reverted transaction fail closed without a success receipt.

## Non-Goals

This lock does not select an EVM client library, support wallet discovery, implement calldata,
or claim public-network compatibility. It does not make the simulator a deployment adapter.

## References

- `docs/design/phase-30.3.1-deterministic-target-abi-lock.md`
- `docs/design/phase-30.3.2-deterministic-local-simulator-lock.md`
- `docs/todo/milestone_4_roadmap.md`
