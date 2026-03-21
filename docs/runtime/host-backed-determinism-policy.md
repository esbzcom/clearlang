# Host-Backed Capability Determinism Policy (Phase 24.0.1)

## Scope
This policy locks runtime behavior for host-backed standard surfaces:
- `std::crypto`
- `std::env`
- `std::wasi`

## Host-Backed Contract
These surfaces remain host imports (not compiled away into app logic):
- `std::crypto::{hash,hmac,verify}` -> `clearlang_crypto::*`
- `std::env::{time,random}` -> `clearlang_env::*`
- `std::wasi::print` -> `wasi_snapshot_preview1::fd_write`

## Determinism Rules
1. `std::crypto`
- Deterministic for identical inputs.
- No host wall-clock or ambient entropy dependency is allowed in crypto operations.
- Unsupported/malformed inputs fail with stable runtime diagnostics.

2. `std::env`
- Local runtime (`clg run`) stubs are deterministic:
  - `env_time()` returns a stable deterministic value.
  - `env_random(n)` returns deterministic bytes for identical `n`.
- Production strict capability policy must explicitly allow/deny env capabilities by host profile.
- No permissive fallback when production profile requires strict runtime checks.

3. `std::wasi`
- `std::wasi::print` remains host-backed IO.
- Output side effects are expected; determinism guarantees apply to loader/linker diagnostics and trust gates, not terminal ordering across environments.

## Enforcement
- Build/runtime profile checks and runtime trust gates must stay fail-closed for production profiles.
- CI covers host-backed import wiring and runtime replay behavior for deterministic cases.
