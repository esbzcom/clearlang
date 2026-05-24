# Namespace: `std::str`

## Purpose
Deterministic UTF-8 string handling for language/runtime safe operations.

## Sub-Namespaces
- `utf8_error`
- `str_pattern`

## Types
- `String` (built-in)
- `Utf8Error`
- `StrPattern`

## Type/Function Draft

### `str`
Functions:
- `len(value: String) -> Int`
- `is_empty(value: String) -> Bool`
- `equals(lhs: String, other: String) -> Bool`
- `concat(lhs: String, other: String) -> String`
- `starts_with(value: String, prefix: String) -> Bool`
- `ends_with(value: String, suffix: String) -> Bool`
- `contains(value: String, needle: String) -> Bool`
- `slice(value: String, start: Int, end: Int) -> Result<String, Utf8Error>`
- `trim(value: String) -> String`
- `to_bytes(value: String) -> Bytes`

### `utf8_error`
Functions:
- `code(err: Utf8Error) -> ErrorCode`
- `offset(err: Utf8Error) -> Option<Int>`
- `equals(a: Utf8Error, other: Utf8Error) -> Bool`

### `str_pattern`
Functions:
- `new(literal: String) -> StrPattern`
- `matches(pattern: StrPattern, input: String) -> Bool`

## First-Production Cut (recommended)
- Keep `std::str`: `len`, `is_empty`, `equals`, `concat`, `starts_with`, `ends_with`, `contains`, `to_bytes`.
- Keep `str_pattern`: stable `matches` contract for first-production API planning.
- Keep `Utf8Error`: `code`, `offset`, `equals`.
- Defer slicing/trim and `str_pattern::new` if they slow initial release.

## Notes
- Locale-sensitive transforms (case mapping, collation) are out of scope for first production release.
- String behavior MUST be deterministic across platforms.
- UTF-8 boundary checks for `slice` MUST be deterministic and MUST return stable `Utf8Error` offsets.
- String operations MUST NOT depend on host locale or timezone state.

## Security Considerations
- `contains`/`starts_with`/`ends_with` are general-purpose helpers and are not constant-time contracts.
- Secret-bearing string material SHOULD be converted to `Bytes` and compared via `std::bytes::equals_ct`.

## Contract Conformance Checklist
- `len` semantics MUST be clearly locked (byte length vs scalar length) and remain stable once finalized.
- `slice` MUST validate bounds and UTF-8 correctness deterministically.
- `Utf8Error` mapping (`code`, `offset`) MUST be stable across platforms and releases.

## Summary
- Provides core operations like length, equality, concat, and validation.
- Enforces deterministic behavior and clear diagnostics for invalid UTF-8 paths.
- Avoids locale-dependent or host-dependent behavior in production profiles.


