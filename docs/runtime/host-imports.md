# Runtime Host Imports (Phase 14)

This document describes the capability interfaces exposed by the runtime to
Wasm modules. It is a design target; exact import names and calling conventions
will be finalized during implementation.

## Goals
- Deterministic, chain-agnostic host boundary.
- Minimal surface area that can be re-implemented by different hosts.
- Explicit effect gating for any host interaction.

## Conventions
- Inputs are byte slices (`Bytes`) referencing linear memory.
- Outputs are `Bytes` plus a status code, represented by an ABI-level wrapper.
- Concrete pointer/length layouts are part of the ABI spec and will be pinned
  alongside implementation.

## Capability Interfaces (v1)

Storage (io-only)
- `storage_get(key: Bytes) -> Option<Bytes>`
- `storage_set(key: Bytes, value: Bytes) -> Result<(), Error>`
- `storage_delete(key: Bytes) -> Result<(), Error>`

Crypto (io-only)
- `crypto_hash(alg: String, data: Bytes) -> Bytes`
- `crypto_verify(alg: String, msg: Bytes, sig: Bytes, pk: Bytes) -> Bool`

Logging/events (io-only)
- `emit_event(kind: String, data: Bytes) -> Result<(), Error>`

Environment (io-only, optional)
- `env_time() -> Int`
- `env_random(len: Int) -> Bytes`
- `env_chain_id() -> String`

## Effect Gating
- `pure` code must not call any host import.
- `mut` remains local-only; host interaction is always `io`.
- Nondeterministic calls (time/random) are disabled by default and must be
  explicitly enabled by the host.

## Relationship to the Pure Core
- The pure core state transition (`apply`/`query`) should not depend on host
  imports. Any required data must be included in the request envelope or state.
- Chain packages may wrap host capabilities, but only through `io` functions.
