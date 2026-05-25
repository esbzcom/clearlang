# Phase 26.1.4.5 - `std::crypto` First-Production API Lock

## Scope
This lock defines the first-production `std::crypto` surface for Gate B execution, preserving current host-backed usage while establishing a cleaner typed path.

## First-Production API (Release-Enabled)

Typed canonical calls:
- `std::crypto::sha256(input: Bytes) -> Hash256`
- `std::crypto::hmac_sha256(key: Bytes, message: Bytes) -> Hash256`
- `std::crypto::verify(signature: Signature, message: Bytes, key: PublicKey) -> VerifyResult`

Current-usage compatibility calls (release-enabled in this phase):
- `std::crypto::hash(algorithm: String, input: Bytes) -> Bytes`
- `std::crypto::hmac(algorithm: String, key: Bytes, message: Bytes) -> Bytes`

Result and error helpers:
- `std::crypto::verify_result::{is_valid,error_or_none,valid,invalid}`
- `std::crypto::crypto_error::{code,equals}`

## Determinism and Assurance Boundary Contracts
- `sha256` and `hmac_sha256` MUST be byte-stable and deterministic across supported targets.
- `hash`/`hmac` compatibility calls MUST accept only explicitly allowed algorithms in first production (`sha256`, `hmac_sha256`) and MUST fail closed for unknown algorithms.
- `verify` MUST be deterministic for identical inputs and MUST NOT perform hidden algorithm substitution.
- `verify_result` invariant MUST hold: `valid` implies no error, `invalid(err)` implies an error, and mixed/invalid states are rejected by conformance tests.
- Release-proof claims MUST stay explicit about remaining crypto assumption boundaries until proof closure tasks are complete.

## Deferred (Non-Release) Symbols
- `std::crypto::hash256::blake2b_256`
- `std::crypto::algorithm::secp256k1`
- Any additional algorithm families, key formats, or advanced crypto primitives.

## Gate B Exit for `std::crypto`
`std::crypto` slice is complete when:
1. API/doc/metadata surface is consistent with this lock.
2. `typed|runtime|proved` statuses are recorded in `docs/std/coverage-matrix.md`.
3. Release profile fails closed for deferred/unknown algorithm surfaces.
4. CI includes deterministic negative tests for `verify_result` mixed/invalid-state rejection.
