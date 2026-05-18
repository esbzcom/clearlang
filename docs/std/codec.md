# Namespace: `std::codec`

## Purpose
Canonical encoding and decoding primitives for deterministic ABI and hashing workflows.

## Sub-Namespaces
- `encoder`
- `decoder`
- `decode_error`
- `encode_error`

## Types
- `Encoder`
- `Decoder`
- `DecodeError`
- `EncodeError`

## Type/Function Draft

### `encoder`
Functions:
- `new() -> Encoder`
- `write_u64(enc: Encoder, value: U64) -> Result<Encoder, EncodeError>`
- `write_u128(enc: Encoder, value: U128) -> Result<Encoder, EncodeError>`
- `write_u256(enc: Encoder, value: U256) -> Result<Encoder, EncodeError>`
- `write_bool(enc: Encoder, value: Bool) -> Result<Encoder, EncodeError>`
- `write_bytes(enc: Encoder, value: Bytes) -> Result<Encoder, EncodeError>`
- `write_string(enc: Encoder, value: String) -> Result<Encoder, EncodeError>`
- `finish(enc: Encoder) -> Result<Bytes, EncodeError>`

### `decoder`
Functions:
- `new(input: Bytes) -> Decoder`
- `read_u64(dec: Decoder) -> Result<(Decoder, U64), DecodeError>`
- `read_u128(dec: Decoder) -> Result<(Decoder, U128), DecodeError>`
- `read_u256(dec: Decoder) -> Result<(Decoder, U256), DecodeError>`
- `read_bool(dec: Decoder) -> Result<(Decoder, Bool), DecodeError>`
- `read_bytes(dec: Decoder) -> Result<(Decoder, Bytes), DecodeError>`
- `read_string(dec: Decoder) -> Result<(Decoder, String), DecodeError>`
- `read_fixed(dec: Decoder, len: Int) -> Result<(Decoder, Bytes), DecodeError>`
- `position(dec: Decoder) -> Int`
- `remaining(dec: Decoder) -> Int`
- `is_eof(dec: Decoder) -> Bool`

### `decode_error`
Functions:
- `code(err: DecodeError) -> ErrorCode`
- `offset(err: DecodeError) -> Option<Int>`
- `equals(a: DecodeError, other: DecodeError) -> Bool`

### `encode_error`
Functions:
- `code(err: EncodeError) -> ErrorCode`
- `equals(a: EncodeError, other: EncodeError) -> Bool`

## First-Production Cut (recommended)
- Keep canonical primitives: `write/read_u64`, `write/read_bool`, `write/read_bytes`, `finish`, `position`, `remaining`, `is_eof`.
- Keep deterministic `DecodeError`/`EncodeError`.
- Defer broader type coverage until ABI proof gates are complete.

## Notes
- Encoding rules MUST be canonical and versioned.
- Decode failures MUST remain deterministic and machine-readable.
- Integer wire endianness MUST be explicitly locked and consistent with `std::int` conversion contracts.
- Length prefixes and container framing MUST be deterministic and bounded by explicit limits.

## Security Considerations
- Canonical encoding MUST reject alternate valid-but-non-canonical forms.
- `Decoder` MUST fail closed on truncated, oversized, or malformed payloads.
- `position`/`remaining` MUST be deterministic and safe for defensive parsing logic.

## Contract Conformance Checklist
- Encoder/decoder round-trips MUST be deterministic for canonical payloads.
- Non-canonical or malformed payloads MUST be rejected with stable `DecodeError` codes/offsets.
- `read_fixed` MUST enforce exact-length semantics with deterministic failure behavior.
- Encoding schema version changes MUST preserve explicit backward/forward compatibility policy.

## Summary
- Guarantees canonical serialization outputs for signature/hash stability.
- Produces deterministic decode failures with stable machine-readable diagnostics.
- Serves release/replay workflows that require byte-for-byte reproducibility.


