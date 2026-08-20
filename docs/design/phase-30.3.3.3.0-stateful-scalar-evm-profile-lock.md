# Phase 30.3.3.3.0 - Stateful Scalar EVM Profile Lock

Date: 2026-08-20
Status: Locked
Owner: target-runtime-owner

## Purpose

Define the first executable EVM profile for ClearLang stateful contracts without broadening the
target claim to arbitrary EVM execution. The profile gives every emitted storage transition a
deterministic correspondence to the existing state VC model.

## Profile

The profile identifier is `clg.evm-stateful-scalar.v1`, targeting EVM opcodes available since
London. It supports one contract, direct `init`, `pure` reads, and `mut` straight-line
transitions over `Bool`, `U8`, `U64`, `U128`, and `Int` state fields and parameters.

- A field's storage slot is `keccak256("clg.evm-stateful-scalar-slot.v1\0" || field_id)`, where
  `field_id` is the canonical state-schema identifier. The artifact records every field ID and
  derived 32-byte slot.
- `StateRead(field)` is `SLOAD(slot(field))`; `StateWrite(field, value)` is
  `SSTORE(slot(field), value)`. Values use one 256-bit EVM word and ClearLang scalar range checks
  remain compiler obligations.
- Constructor arguments are standard ABI-v2 words appended to creation bytecode. `init` executes
  once during creation; the compiled constructor must contain the checked direct-write profile
  accepted by `T829`.
- Function selectors and scalar calldata/results use `clg.evm-wire-abi.v1`. Unknown selectors,
  malformed/short calldata, unsupported instructions, and failed guards revert.
- A non-indexed event has `topic0 = keccak256(event signature)` and ABI-v2 word data, emitted by
  `LOG1`. Indexed event fields are outside the first profile.

## VC Correspondence

The artifact records the state schema digest, field-ID/slot map, and ordered supported IR
transition operations. For every emitted `StateRead`/`StateWrite`, the mapping names the same
canonical field ID used by the VC entry/exit symbols. Any function containing an IR instruction
without a listed EVM semantics is rejected before artifact emission.

## Explicit Exclusions

Collections, dynamic strings/bytes, `U256`, branches or loops that mutate state, external calls,
indexed events, delegates/proxies, fallback dispatch, value transfer, and arbitrary inline EVM are
not supported. No artifact may silently substitute simulator semantics for EVM execution.

## Acceptance Evidence

1. Repeated builds produce byte-identical artifacts, slots, and transition mappings.
2. Constructor and transition fixtures show EVM storage matching the VC field mapping.
3. Unsupported types, instructions, or control flow fail before bytecode is emitted.

