# Package: `std::codec`

## Purpose
Canonical encoding and decoding primitives for deterministic ABI and hashing workflows.

## Key Types
- `Encoder`
- `Decoder`
- `DecodeError`
- `EncodeError`

## Class/Method Draft

### `Encoder`
Methods:
- `new() -> Encoder`
- `write_u64(self, value: U64) -> Result<Encoder, EncodeError>`
- `write_u128(self, value: U128) -> Result<Encoder, EncodeError>`
- `write_u256(self, value: U256) -> Result<Encoder, EncodeError>`
- `write_bool(self, value: Bool) -> Result<Encoder, EncodeError>`
- `write_bytes(self, value: Bytes) -> Result<Encoder, EncodeError>`
- `write_string(self, value: String) -> Result<Encoder, EncodeError>`
- `finish(self) -> Result<Bytes, EncodeError>`

### `Decoder`
Methods:
- `new(input: Bytes) -> Decoder`
- `read_u64(self) -> Result<(Decoder, U64), DecodeError>`
- `read_u128(self) -> Result<(Decoder, U128), DecodeError>`
- `read_u256(self) -> Result<(Decoder, U256), DecodeError>`
- `read_bool(self) -> Result<(Decoder, Bool), DecodeError>`
- `read_bytes(self) -> Result<(Decoder, Bytes), DecodeError>`
- `read_string(self) -> Result<(Decoder, String), DecodeError>`
- `is_eof(self) -> Bool`

### `DecodeError`
Methods:
- `code(self) -> ErrorCode`
- `offset(self) -> Option<Int>`
- `equals(self, other: DecodeError) -> Bool`

### `EncodeError`
Methods:
- `code(self) -> ErrorCode`
- `equals(self, other: EncodeError) -> Bool`

## First-Production Cut (recommended)
- Keep canonical primitives: `write/read_u64`, `write/read_bool`, `write/read_bytes`, `finish`, `is_eof`.
- Keep deterministic `DecodeError`/`EncodeError`.
- Defer broader type coverage until ABI proof gates are complete.

## Notes
- Encoding rules must be canonical and versioned.
- Decode failures must remain deterministic and machine-readable.

## Summary
- Guarantees canonical serialization outputs for signature/hash stability.
- Produces deterministic decode failures with stable machine-readable diagnostics.
- Serves release/replay workflows that require byte-for-byte reproducibility.
