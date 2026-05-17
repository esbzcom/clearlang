# Package: `std::unit`

## Purpose
Minimal deterministic testing/assertion helpers for `clg test`, with crypto-safe testing behavior.

## Key Types
- `Assert`
- `TestFail`
- `AssertError`

## Class/Method Draft

### `Assert`
Methods:
- `assert_true(value: Bool) -> Bool`
- `assert_eq_int(left: Int, right: Int) -> Bool`
- `assert_eq_bool(left: Bool, right: Bool) -> Bool`
- `fail(code: ErrorCode) -> Bool`

### `Assert` (crypto-oriented deferred methods)
Methods:
- `assert_eq_bytes_ct(left: Bytes, right: Bytes) -> Bool` (constant-time compare assertion; deferred)
- `assert_not_zero_u64(value: U64) -> Bool` (common nonce/amount guard assertion; deferred)

### `TestFail`
Methods:
- `code(self) -> ErrorCode`
- `equals(self, other: TestFail) -> Bool`

### `AssertError`
Methods:
- `code(self) -> ErrorCode`
- `equals(self, other: AssertError) -> Bool`

## First-Production Cut (recommended)
- Keep `Assert`: `assert_true`, `assert_eq_int`, `assert_eq_bool`, `fail`.
- Keep deterministic failure mapping via `TestFail`.
- Defer richer assertions (including `assert_false`, bytes/generic/error-shape asserts) to follow-up phases.
- Defer crypto-oriented assertions (`assert_eq_bytes_ct`, `assert_not_zero_u64`) until their policy/typing/runtime gates are explicitly locked.
- Defer `AssertError` typed-return usage until advanced assertion APIs are enabled.

## Notes
- Assertion APIs must preserve deterministic test failure IDs/report shape.
- No expansion that requires new CLI flags in first production cut.
- `AssertError` is reserved for post-first-production assertion APIs; first-production assertions return `Bool` and fail deterministically via runner mapping.
- Crypto-oriented assertions must avoid secret-dependent branch/report data; failure payloads should expose stable error codes only.
- Constant-time comparison assertions must reuse the same constant-time primitives as production code paths.

## Summary
- Supports baseline assertion ergonomics without widening core CLI/test-plan surface.
- Keeps failure mapping deterministic for CI/replay and diagnostics stability.
- Advanced assertion features remain additive follow-up work.
