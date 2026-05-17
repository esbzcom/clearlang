# Package: `std::unit`

## Purpose
Minimal deterministic testing/assertion helpers for `clg test`.

## Key Types
- `Assert`
- `TestFail`
- `AssertError`

## Class/Method Draft

### `Assert`
Methods:
- `true(value: Bool) -> Bool`
- `false(value: Bool) -> Bool`
- `eq_int(left: Int, right: Int) -> Bool`
- `eq_bool(left: Bool, right: Bool) -> Bool`
- `fail(code: ErrorCode) -> Bool`

### `TestFail`
Methods:
- `code(self) -> ErrorCode`
- `equals(self, other: TestFail) -> Bool`

### `AssertError`
Methods:
- `code(self) -> ErrorCode`
- `equals(self, other: AssertError) -> Bool`

## First-Production Cut (recommended)
- Keep `Assert`: `true`, `eq_int`, `eq_bool`, `fail`.
- Keep deterministic failure mapping via `TestFail`.
- Defer richer assertions (bytes/generic/error-shape asserts) to follow-up phases.

## Notes
- Assertion APIs must preserve deterministic test failure IDs/report shape.
- No expansion that requires new CLI flags in first production cut.

## Summary
- Supports baseline assertion ergonomics without widening core CLI/test-plan surface.
- Keeps failure mapping deterministic for CI/replay and diagnostics stability.
- Advanced assertion features remain additive follow-up work.
