# Runtime ABI Overview

This document defines the stable host interface that ClearLang programs target when compiled to Wasm.

## Goals
- Deterministic and portable across chains and non-chain runtimes.
- Minimal surface area that can be re-implemented by different hosts.
- Clear separation from chain-specific libraries and business logic.

## Entrypoints
- Exports: `init`, `handle`, `query`.
- Signature: `fn(state_ptr: i32, msg_ptr: i32) -> i32`.
- Inputs/return: pointers to `[u32 len][u8 len]` in linear memory.
- Payloads are canonical CBOR envelopes (see Serialization).

## Serialization
- Requests and responses are canonical CBOR (RFC 8949) maps.
- All map keys are text and must use canonical ordering.
- Envelope details live in `docs/design/phase-14-contract-runtime-decoupling.md`.

## Type Layouts (Selected)
- String/Bytes: `[u32 len][u8 len]` in linear memory (see `docs/runtime/strings.md` for `String`).
- Option/Result: canonical 16-byte layout (`{tag, payload_lo, payload_hi, reserved}`) shared across both ADTs; see `docs/design/phase-7.1-option-result-runtime.md`.
- Arrays/Tuples: layout is defined in `docs/runtime/arrays-tuples.md`; values are `i32` pointers to the contiguous layout described there.
- Structs: heap-allocated field layout matching tuple rules in `docs/runtime/arrays-tuples.md`.
- Enums: canonical 16-byte variant layout (`{tag, payload_lo, payload_hi, reserved}`) with tag range `0..N-1` for `N` variants; multi-field payloads are stored via `payload_lo` pointers.

## Host Capabilities (Baseline)
- Storage: read/write raw bytes by key.
- Crypto syscalls: hash and signature verification hooks (host-provided).
- Logging/events: append structured logs for the host to consume.
- Metering: gas/step accounting and deterministic limits.
- ABI entrypoints: `init`, `handle`, and `query` with canonical serialization.
- Host import surface: see `docs/runtime/host-imports.md`.

## Notes
- Chain packages should wrap these primitives with chain-specific types and rules.
- The runtime must document limits and determinism guarantees.
- The pure contract core model and ABI envelopes are defined in
  `docs/design/phase-14-contract-runtime-decoupling.md`.
