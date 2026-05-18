# Namespace: `std::unit`

## Purpose
Minimal deterministic testing/assertion helpers for `clg test`, with crypto-safe testing behavior.

## Sub-Namespaces
- `assert`
- `test_fail`
- `assert_error`

## Types
- `TestFail`
- `AssertError`

## Type/Function Draft

### `assert`
Functions:
- `assert_true(value: Bool) -> Bool`
- `assert_eq_int(left: Int, right: Int) -> Bool`
- `assert_eq_bool(left: Bool, right: Bool) -> Bool`
- `fail(code: ErrorCode) -> Bool`

### `assert` (crypto-oriented deferred functions)
Functions:
- `assert_eq_bytes_ct(left: Bytes, right: Bytes) -> Bool` (constant-time compare assertion; deferred)
- `assert_not_zero_u64(value: U64) -> Bool` (common nonce/amount guard assertion; deferred)

### `test_fail`
Functions:
- `code(err: TestFail) -> ErrorCode`
- `equals(a: TestFail, other: TestFail) -> Bool`

### `assert_error`
Functions:
- `code(err: AssertError) -> ErrorCode`
- `equals(a: AssertError, other: AssertError) -> Bool`

## First-Production Cut (recommended)
- Keep `assert`: `assert_true`, `assert_eq_int`, `assert_eq_bool`, `fail`.
- Keep deterministic failure mapping via `TestFail`.
- Defer richer assertions (including `assert_false`, bytes/generic/error-shape asserts) to follow-up phases.
- Defer crypto-oriented assertions (`assert_eq_bytes_ct`, `assert_not_zero_u64`) until their policy/typing/runtime gates are explicitly locked.
- Defer `AssertError` typed-return usage until advanced assertion APIs are enabled.

## Notes
- Assertion APIs MUST preserve deterministic test failure IDs/report shape.
- No expansion that requires new CLI flags in first production cut.
- `AssertError` is reserved for post-first-production assertion APIs; first-production assertions return `Bool` and fail deterministically via runner mapping.
- Crypto-oriented assertions MUST avoid secret-dependent branch/report data; failure payloads SHOULD expose stable error codes only.
- Constant-time comparison assertions MUST reuse the same constant-time primitives as production code paths.

## Security Considerations
- Test-only APIs MUST NOT weaken production runtime/typing guarantees.
- Constant-time assertions SHOULD be used for cryptographic equality checks in tests to catch misuse early.

## Contract Conformance Checklist
- Baseline assertion names and signatures MUST remain stable in first production cut.
- Failure mapping (`TestFail`) MUST be deterministic and reproducible across environments.
- Deferred assertion APIs MUST remain feature-gated until contracts are explicitly locked.

## Summary
- Supports baseline assertion ergonomics without widening core CLI/test-plan surface.
- Keeps failure mapping deterministic for CI/replay and diagnostics stability.
- Advanced assertion features remain additive follow-up work.


