# ClearLang Development Plan

## Current Focus - Phase 16: Crypto Intrinsics + Proofs
- 16.1 Hash/HMAC intrinsics (SHA-256, Keccak, Blake2) with deterministic semantics and test vectors. (Done)
- 16.2 Signature verification intrinsics (ed25519/secp256k1) with strict input validation. (Done)
- 16.3 Constant-time byte equality helper for secret comparisons. (Done)
- 16.4 Deterministic error semantics + diagnostics for crypto intrinsics. (Done)
- 16.5 SMT encoding (or explicit axioms) for crypto/bitwise primitives. (Done)
- 16.6 Document proof limitations for cryptographic primitives in `docs/proofs`. (Done)
- 16.7 On-chain attestation design (EVM registry + IPFS URIs) with a minimal reference implementation and docs.

### Suggested Sequence
1) Lock API surface + error semantics (16.1/16.2/16.4), update docs and diagnostics.
2) Implement intrinsics + runtime stubs + tests (16.1/16.2/16.3).
3) Update SMT/axioms + proof limitations + attestation design/docs (16.5/16.6/16.7).

## Recently Completed
- **Phase 11 - Proof-carrying Wasm verification**: `clg verify` CLI, proof-section hashing/signing, diagnostics, fixtures, and regression tests.
- **Phases 12-15 (per `docs/TODO.md`)**: safety/tooling hardening, DX improvements, runtime decoupling, and unsigned/bitwise expansion.

## Snapshot of Earlier Milestones
- Phase 10: refinements (VC/SMT, fixtures, diagnostics).
- Phase 9: totality & loops.
- Phase 8: resource/linear types.
- Phase 7: Option/Result lowering and proof layout.
- Phase 6: contract syntax + VC generation.
- Phase 1-5: parser/typer/IR + Wasm pipeline foundations.

## Upcoming Phases (High-Level)
- **Phase 17 - Language gaps + collections**: user-defined structs/enums, generics/traits, runtime collections, arrays/slices, module system, linear-aware collections.

### Notes
- `docs/TODO.md` is the canonical checklist; keep this file high-level.
- Historical detail from earlier phases is archived in `docs/rollout/codex-session-history.md`.
