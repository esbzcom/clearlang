# Phase 26.1.4.1 - `std::str` First-Production API Lock

## Scope
This lock defines the first-production `std::str` surface for Gate B execution.

## First-Production API (Release-Enabled)
- `std::str::len(value: String) -> Int`
- `std::str::is_empty(value: String) -> Bool`
- `std::str::equals(lhs: String, other: String) -> Bool`
- `std::str::concat(lhs: String, other: String) -> String`
- `std::str::starts_with(value: String, prefix: String) -> Bool`
- `std::str::ends_with(value: String, suffix: String) -> Bool`
- `std::str::contains(value: String, needle: String) -> Bool`
- `std::str::to_bytes(value: String) -> Bytes`
- `std::str_pattern::matches(pattern: String, input: String) -> Bool`

## Determinism Contracts
- `std::str` behavior MUST be locale-independent and timezone-independent.
- `len` semantics MUST remain stable after lock (byte length vs scalar length must not drift silently).
- `starts_with`/`ends_with`/`contains` MUST be deterministic for identical inputs across supported targets.
- `to_bytes` MUST preserve UTF-8 byte representation deterministically.

## Diagnostics and Error Surface
- Unsupported/deferred `std::str` symbols in production profile MUST fail closed with deterministic diagnostics.
- Any future `Utf8Error` path (`slice`, decode-related surfaces) MUST use stable code/offset mapping before release enablement.

## Deferred (Non-Release) Symbols
- `std::str::slice`
- `std::str::trim`
- `std::utf8_error::*`
- `std::str_pattern::new`

## Compatibility Note
- `std::str_pattern::matches` is locked for first production with `String` pattern input to keep runtime deterministic while `StrPattern` construction remains deferred.

## Gate B Exit for `std::str`
`std::str` slice is complete when:
1. API/doc/metadata surface is consistent.
2. `typed|runtime|proved` statuses are recorded in `docs/std/coverage-matrix.md`.
3. Release profile behavior is fail-closed for deferred symbols.
4. CI has deterministic contract tests for release-enabled symbols.
