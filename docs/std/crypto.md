# Package: `std::crypto`

## Purpose
Cryptographic primitives for hashing, signature verification, and secure comparisons.

## Key Types
- `Hash256`
- `Signature`
- `PublicKey`
- `CryptoAlgorithm`
- `VerifyResult`
- `CryptoError`

## Class/Method Draft

### `Hash256`
Methods:
- `sha256(input: Bytes) -> Hash256`
- `blake2b_256(input: Bytes) -> Hash256`
- `bytes(self) -> Bytes`
- `equals(self, other: Hash256) -> Bool`

### `Signature`
Methods:
- `from_bytes(input: Bytes) -> Result<Signature, CryptoError>`
- `to_bytes(self) -> Bytes`
- `algorithm(self) -> CryptoAlgorithm`

### `PublicKey`
Methods:
- `from_bytes(input: Bytes) -> Result<PublicKey, CryptoError>`
- `to_bytes(self) -> Bytes`
- `algorithm(self) -> CryptoAlgorithm`

### `CryptoAlgorithm`
Methods:
- `ed25519() -> CryptoAlgorithm`
- `secp256k1() -> CryptoAlgorithm`
- `equals(self, other: CryptoAlgorithm) -> Bool`

### `VerifyResult`
Methods:
- `is_valid(self) -> Bool`
- `error_or_none(self) -> Option<CryptoError>`
- `valid() -> VerifyResult`
- `invalid(err: CryptoError) -> VerifyResult`

### `CryptoError`
Methods:
- `code(self) -> ErrorCode`
- `equals(self, other: CryptoError) -> Bool`

## Top-Level Functions
- `verify(signature: Signature, message: Bytes, key: PublicKey) -> VerifyResult`
- `hmac_sha256(key: Bytes, message: Bytes) -> Hash256`

## First-Production Cut (recommended)
- Keep `sha256`, `verify`, `hmac_sha256`, `bytes/to_bytes/from_bytes`.
- Keep deterministic `CryptoError` and `VerifyResult`.
- Defer additional algorithms until proof status and policy gates are explicitly green.

## Notes
- All crypto APIs must remain deterministic and policy-gated for release claims.
- Constant-time behavior requirements should be explicit in implementation notes/tests.
- `VerifyResult` invariant: exactly one state is valid (`valid` with no error, or `invalid` with an error); mixed states are invalid API behavior.
- Raw integer algorithm ids are not part of the public first-production API surface; use `CryptoAlgorithm`.

## Summary
- Exposes deterministic crypto APIs with explicit assurance-boundary labeling.
- Prioritizes constant-time/security-sensitive surfaces where applicable.
- Must remain compatible with proof/release policy gates for production claims.
