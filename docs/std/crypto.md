# Namespace: `std::crypto`

## Purpose
Cryptographic primitives for hashing, signature verification, and secure comparisons.

## Sub-Namespaces
- `hash256`
- `signature`
- `public_key`
- `algorithm`
- `verify_result`
- `crypto_error`

## Types
- `Hash256`
- `Signature`
- `PublicKey`
- `CryptoAlgorithm`
- `VerifyResult`
- `CryptoError`

## Type/Function Draft

### `hash256`
Functions:
- `sha256(input: Bytes) -> Hash256`
- `blake2b_256(input: Bytes) -> Hash256`
- `bytes(hash: Hash256) -> Bytes`
- `equals(lhs: Hash256, other: Hash256) -> Bool`

### `signature`
Functions:
- `from_bytes(input: Bytes) -> Result<Signature, CryptoError>`
- `to_bytes(sig: Signature) -> Bytes`
- `algorithm(sig: Signature) -> CryptoAlgorithm`

### `public_key`
Functions:
- `from_bytes(input: Bytes) -> Result<PublicKey, CryptoError>`
- `to_bytes(pk: PublicKey) -> Bytes`
- `algorithm(pk: PublicKey) -> CryptoAlgorithm`

### `algorithm`
Functions:
- `ed25519() -> CryptoAlgorithm`
- `secp256k1() -> CryptoAlgorithm`
- `equals(lhs: CryptoAlgorithm, other: CryptoAlgorithm) -> Bool`

### `verify_result`
Functions:
- `is_valid(value: VerifyResult) -> Bool`
- `error_or_none(value: VerifyResult) -> Option<CryptoError>`
- `valid() -> VerifyResult`
- `invalid(err: CryptoError) -> VerifyResult`

### `crypto_error`
Functions:
- `code(err: CryptoError) -> ErrorCode`
- `equals(a: CryptoError, other: CryptoError) -> Bool`

## Top-Level Functions
- `verify(signature: Signature, message: Bytes, key: PublicKey) -> VerifyResult`
- `hmac_sha256(key: Bytes, message: Bytes) -> Hash256`

## First-Production Cut (recommended)
- Keep `sha256`, `verify`, `hmac_sha256`, `bytes/to_bytes/from_bytes`.
- Keep deterministic `CryptoError` and `VerifyResult`.
- Defer additional algorithms until proof status and policy gates are explicitly green.

## Notes
- All crypto APIs MUST remain deterministic and policy-gated for release claims.
- Constant-time behavior requirements MUST be explicit and testable in conformance suites.
- `VerifyResult` invariant: exactly one state is valid (`valid` with no error, or `invalid` with an error); mixed states MUST be rejected as invalid API behavior.
- Raw integer algorithm ids are not part of the public first-production API surface; callers MUST use `CryptoAlgorithm`.
- `from_bytes` for `Signature`/`PublicKey` MUST enforce canonical encoding and algorithm-specific length constraints.
- `verify(signature, message, key)` MUST NOT perform undocumented implicit transcoding or algorithm substitution.

## Security Considerations
- Message interpretation for `verify` MUST be explicit (raw message bytes, no hidden pre-hash unless API name states it).
- Protocols SHOULD apply domain separation before calling hash/signature APIs.
- Algorithm-specific normalization rules (for example, signature canonical form) MUST be deterministic and fail closed.
- Error surfaces MUST avoid leaking secret material; diagnostics SHOULD expose stable codes rather than secret-derived details.

## Contract Conformance Checklist
- `sha256`/`blake2b_256` outputs MUST be deterministic and byte-stable across platforms.
- `hmac_sha256` MUST use canonical HMAC-SHA256 semantics with deterministic output for identical inputs.
- `verify_result::valid`/`invalid` constructors MUST preserve invariant integrity.
- `verify_result::is_valid` and `error_or_none` MUST be logically consistent with the invariant.
- `CryptoError` code mapping MUST be stable and non-overlapping with other package domains.

## Summary
- Exposes deterministic crypto APIs with explicit assurance-boundary labeling.
- Prioritizes constant-time/security-sensitive surfaces where applicable.
- Must remain compatible with proof/release policy gates for production claims.


