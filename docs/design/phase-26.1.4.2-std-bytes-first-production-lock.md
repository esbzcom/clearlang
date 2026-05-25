# Phase 26.1.4.2 - `std::bytes` First-Production API Lock

## Scope
This lock defines the first-production `std::bytes` surface for Gate B execution.

## First-Production API (Release-Enabled)
- `std::bytes::len(value: Bytes) -> Int`
- `std::bytes::is_empty(value: Bytes) -> Bool`
- `std::bytes::concat(lhs: Bytes, other: Bytes) -> Bytes`
- `std::bytes::equals(lhs: Bytes, other: Bytes) -> Bool`
- `std::bytes::equals_ct(lhs: Bytes, other: Bytes) -> Bool`
- `std::bytes::from_string(value: String) -> Bytes`

## Determinism and Security Contracts
- `len`/`is_empty`/`concat`/`equals` MUST be deterministic for identical inputs across supported targets.
- `equals_ct` MUST avoid secret-dependent branches and memory access patterns.
- `equals_ct` MUST return `false` deterministically for length mismatch; length short-circuit is allowed because length is public metadata.
- `from_string` MUST preserve UTF-8 byte identity deterministically.

## Deferred (Non-Release) Symbols
- `std::bytes::slice`
- `std::bytes::to_string`
- `std::bytes::to_hex`
- `std::bytes::from_hex`
- `std::bytes_error::{code, offset, equals}` until at least one deferred error-producing helper is release-enabled.

## Deferred-Method Contract Locks (Pre-Activation)
- `to_hex` canonical output: lowercase ASCII hex, no `0x` prefix, no separators.
- `from_hex` acceptance: ASCII hex digits only (`0-9a-fA-F`), no prefix/whitespace/separators; odd length rejected deterministically.
- `slice` bounds: reject negative indices, reject `start > end`, reject `end > len(value)`, stable deterministic `BytesError` mapping.
- `to_string` UTF-8: reject invalid/non-canonical UTF-8 deterministically with stable error offset mapping.

## Gate B Exit for `std::bytes`
`std::bytes` slice is complete when:
1. API/doc/metadata surface is consistent with this lock.
2. `typed|runtime|proved` statuses are recorded in `docs/std/coverage-matrix.md`.
3. Release profile fails closed for deferred methods.
4. CI has deterministic contract tests for all release-enabled symbols.
