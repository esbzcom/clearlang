# Phase 26.1.4.8 - `std::unit` First-Production API Lock

## Scope
This lock aligns `std::unit` with the Gate D baseline assertion surface.

## First-Production API (Release-Enabled)
- `std::unit::assert_true(value: Bool) -> Bool`
- `std::unit::assert_eq_int(left: Int, right: Int) -> Bool`
- `std::unit::assert_eq_bool(left: Bool, right: Bool) -> Bool`
- `std::unit::fail(code: ErrorCode) -> Bool`

## Determinism and Failure-Mapping Contracts
- Assertion failure mapping MUST remain deterministic and reproducible across runs.
- Baseline method names/signatures are locked for first production and MUST NOT drift.
- Production profile MUST fail closed for non-baseline assertion names.

## Deferred (Non-Release) Symbols
- `assert_false`
- `assert_eq_bytes` and constant-time bytes assertion variants
- Generic/assertion-shape extensions and expected-failure assertions
- `assert_error::*` typed-return paths

## Gate B Exit for `std::unit`
`std::unit` slice is complete when:
1. API/doc/metadata surface is consistent with this lock.
2. `typed|runtime|proved` statuses are recorded in `docs/std/coverage-matrix.md`.
3. CI has deterministic contract tests for baseline method-name stability.
4. Release profile fails closed for deferred assertion names.
