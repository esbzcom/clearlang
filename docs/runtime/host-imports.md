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
- Outputs are returned directly (e.g., `Bytes` pointer or `Bool`). Failures are signaled via the runtime error globals/traps (R00x) rather than out-of-band status wrappers.
- Concrete pointer/length layouts are part of the ABI spec and will be pinned
  alongside implementation.

## Capability Interfaces (v1)

Storage (io-only)
- `storage_get(key: Bytes) -> Option<Bytes>`
- `storage_set(key: Bytes, value: Bytes) -> Result<(), Error>`
- `storage_delete(key: Bytes) -> Result<(), Error>`

Crypto (io-only, module: `clearlang_crypto`)
- `crypto_hash(alg: String, data: Bytes) -> Bytes`
- `crypto_hmac(alg: String, key: Bytes, data: Bytes) -> Bytes`
- `crypto_verify(alg: String, msg: Bytes, sig: Bytes, pk: Bytes) -> Bool`

Algorithms (v1)
- Hash/HMAC `alg` values: `sha256`, `keccak256`, `blake2b256`, `blake2s256` (lowercase).
- Signature `alg` values: `ed25519`, `secp256k1` (lowercase).
- `crypto_hash`/`crypto_hmac` return fixed-length outputs:
  - `sha256`/`keccak256`/`blake2b256`/`blake2s256` -> 32 bytes.
- `crypto_verify` returns `false` on signature mismatch; it traps on invalid inputs.
- Invalid algorithm identifiers trap with runtime error `R006`.
- Invalid key/signature lengths trap with runtime error `R007`.
- Malformed signature encoding traps with runtime error `R008`.

Input length rules (v1)
- `ed25519`: public key 32 bytes, signature 64 bytes.
- `secp256k1`: public key 33 (compressed) or 65 (uncompressed) bytes; signature 64 bytes (`r || s`).

Logging/events (io-only)
- `emit_event(kind: String, data: Bytes) -> Result<(), Error>`

WASI stdout (io-only)
- `std::wasi::print(b: Bytes) -> Int` writes bytes to stdout via `wasi_snapshot_preview1::fd_write`.

Environment (io-only, optional, module: `clearlang_env`)
- `env_time() -> Int`
- `env_random(len: Int) -> Bytes`
- `env_chain_id() -> String`

## Ownership Lock (Phase 21.0.4.1)
- Runtime capability ownership remains in the host boundary (`std::host` package model).
- Canonical user-facing ownership in v1:
  - `std::env::{time,random}` are direct host-backed calls.
  - `chain_id`, storage, and events are host capabilities but are surfaced through chain wrappers (`std::<chain>::...`) for chain-specific policy and encoding.
- This lock is an ownership boundary definition, not a promise that every wrapper/API is already implemented in Phase 21.

## Strict Policy Lock for `std::env::{time,random}` (Phase 21.0.4.2)
- In `--compiler-mode strict`, both `std::env::time` and `std::env::random` are denied for Phase 21.
- Denials are deterministic and fail closed with `C106`.
- Profile matrix in Phase 21:
  - `contract_static`: deny `time`, deny `random`.
  - `shared_app`: deny `time`, deny `random`.
- `clg run` stubs (`time=0`, zero-filled random bytes) remain dev/test behavior and do not relax strict build policy.

## Host-Backed Determinism Lock (Phase 24.0.1)
- `std::crypto`, `std::env`, and `std::wasi` remain host-backed surfaces and are not compiled away into local app logic.
- Determinism and replay policy for these surfaces is locked in `docs/runtime/host-backed-determinism-policy.md`.
- Production profile policy for `contract_static` vs `shared_app` is expanded in `docs/runtime/host-profiles-production-policy.md`.

## Effect Gating
- `pure` code must not call any host import.
- `mut` remains local-only; host interaction is always `io`.
- Nondeterministic calls (time/random) are disabled by default and must be
  explicitly enabled by the host.

## Notes
- `clg run` provides deterministic stubs for `env_time` (`0`), `env_random` (zero-filled bytes), and `env_chain_id` (`"dev-chain"`).
- `clg run` also implements `clearlang_crypto` locally for dev/test; production hosts must supply real crypto imports.
- Storage/crypto/logging interfaces remain design targets; hosts must provide their own imports.

## Relationship to the Pure Core
- The pure core state transition (`apply`/`query`) should not depend on host
  imports. Any required data must be included in the request envelope or state.
- Chain packages may wrap host capabilities, but only through `io` functions.
