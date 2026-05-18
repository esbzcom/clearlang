# Namespace: `std::str`

## Purpose
Deterministic UTF-8 string handling for language/runtime safe operations.

## Sub-Namespaces
- `string_view`
- `utf8_error`
- `str_pattern`

## Types
- `String` (built-in)
- `StringView`
- `Utf8Error`
- `StrPattern`

## Type/Function Draft

### `string_view`
Functions:
- `len(value: StringView) -> Int`
- `is_empty(value: StringView) -> Bool`
- `equals(lhs: StringView, other: StringView) -> Bool`
- `concat(lhs: StringView, other: StringView) -> String`
- `starts_with(value: StringView, prefix: StringView) -> Bool`
- `ends_with(value: StringView, suffix: StringView) -> Bool`
- `contains(value: StringView, needle: StringView) -> Bool`
- `slice(value: StringView, start: Int, end: Int) -> Result<StringView, Utf8Error>`
- `trim(value: StringView) -> StringView`
- `to_bytes(value: StringView) -> Bytes`

### `utf8_error`
Functions:
- `code(err: Utf8Error) -> ErrorCode`
- `offset(err: Utf8Error) -> Option<Int>`
- `equals(a: Utf8Error, other: Utf8Error) -> Bool`

### `str_pattern`
Functions:
- `new(literal: StringView) -> StrPattern`
- `matches(pattern: StrPattern, input: StringView) -> Bool`

## First-Production Cut (recommended)
- Keep `StringView`: `len`, `is_empty`, `equals`, `concat`, `starts_with`, `ends_with`, `contains`.
- Keep `Utf8Error`: `code`, `offset`, `equals`.
- Defer slicing/trim/pattern helpers if they slow initial release.

## Notes
- Locale-sensitive transforms (case mapping, collation) are out of scope for first production release.
- String behavior MUST be deterministic across platforms.
- UTF-8 boundary checks for `slice` MUST be deterministic and MUST return stable `Utf8Error` offsets.
- `StringView` operations MUST NOT depend on host locale or timezone state.

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


