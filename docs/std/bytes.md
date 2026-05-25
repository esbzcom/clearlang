# Namespace: `std::bytes`

## Purpose
Byte-level primitives for deterministic binary processing and crypto-oriented workflows.

## Sub-Namespaces
- `bytes`
- `bytes_error`

## Types
- `Bytes` (built-in)
- `BytesError`

## Type/Function Draft

### `bytes`
Functions:
- `len(value: Bytes) -> Int`
- `is_empty(value: Bytes) -> Bool`
- `concat(lhs: Bytes, other: Bytes) -> Bytes`
- `slice(value: Bytes, start: Int, end: Int) -> Result<Bytes, BytesError>`
- `equals(lhs: Bytes, other: Bytes) -> Bool`
- `equals_ct(lhs: Bytes, other: Bytes) -> Bool`
- `from_string(value: String) -> Bytes`
- `to_string(value: Bytes) -> Result<String, BytesError>`
- `to_hex(value: Bytes) -> String`
- `from_hex(input: String) -> Result<Bytes, BytesError>`

### `bytes_error`
Functions:
- `code(err: BytesError) -> ErrorCode`
- `offset(err: BytesError) -> Option<Int>`
- `equals(a: BytesError, other: BytesError) -> Bool`

## First-Production Cut (recommended)
- Keep `Bytes`: `len`, `is_empty`, `concat`, `equals`, `equals_ct`, `from_string`.
- Defer `slice`, `to_string`, `to_hex`, `from_hex` until full deterministic error mapping is enabled.
- Defer `BytesError` method surface until at least one error-producing helper is release-enabled.

## Notes
- `equals_ct` MUST be the default contract for secret comparisons.
- `equals_ct` MUST execute without secret-dependent branches or memory access patterns.
- `equals_ct` MAY reveal public metadata such as input length; it MUST NOT reveal content via timing behavior.
- For length mismatch, `equals_ct` MUST return `false` deterministically; length checks MAY short-circuit because length is public metadata.
- Byte operations MUST preserve deterministic output and diagnostics.
- `slice` bounds policy MUST be deterministic: reject negative `start/end`, reject `start > end`, reject `end > len(value)`, and map each failure class to stable `BytesError` codes.
- `slice`/`to_string`/`from_hex` failures MUST map to deterministic `BytesError` codes with stable offsets when applicable.
- Hex canonical policy MUST be locked:
  - `to_hex` outputs lowercase ASCII (`0-9a-f`) with no prefix and no separators.
  - `from_hex` accepts lowercase and uppercase ASCII hex digits only, with no prefix, whitespace, or separators.
  - `from_hex` rejects odd-length inputs deterministically.
- `to_string` UTF-8 policy MUST be canonical and deterministic:
  - reject invalid UTF-8 byte sequences (including overlong encodings, surrogate code points, and out-of-range scalar encodings).
  - return stable `BytesError` code/offset for decode failures.

## Security Considerations
- Callers handling secrets SHOULD avoid `equals` and use `equals_ct`.
- Hex decoding MUST reject non-canonical or malformed input deterministically.
- `to_string` MUST reject non-UTF-8 byte sequences deterministically.
- Implementations MUST document maximum supported `Bytes` length and deterministic failure behavior at limits.
- `to_hex` output format MUST be canonical to prevent hash/signature mismatches caused by alternate textual encodings.

## Contract Conformance Checklist
First-production required:
- `len`, `is_empty`, `concat`, `equals`, `equals_ct`, `from_string` MUST be deterministic for identical inputs across platforms.
- `equals_ct` MUST have constant-time behavior with respect to byte content.
- `from_string` MUST preserve UTF-8 byte identity deterministically.

Deferred-method conformance (activate when method is release-enabled):
- `from_hex` MUST reject invalid alphabet/length/format with stable error codes.
- `to_hex` MUST emit canonical lowercase format with no prefix/separators.
- `to_string` MUST enforce canonical UTF-8 decoding and stable error offset mapping.
- `BytesError::offset` MUST point to a deterministic failure location when available.

## Summary
- Supports slicing, compare, and canonical byte manipulation helpers.
- Includes constant-time equality operations for security-sensitive use cases.
- Acts as the bridge between high-level types and canonical binary encodings.


