# Namespace: `std::unit`

## Purpose
Deterministic assertion helpers for `clg test`.

## First-Production Baseline (Gate D)
- `assert_true(cond: Bool, msg: String) -> Bool`
- `assert_eq_int(actual: Int, expected: Int, msg: String) -> Bool`
- `assert_eq_bool(actual: Bool, expected: Bool, msg: String) -> Bool`
- `fail(msg: String) -> Bool`

## Gate F Extensions (Phase 26.5)
- `assert_false(cond: Bool, msg: String) -> Bool`
- `assert_eq_u64(actual: U64, expected: U64, msg: String) -> Bool`

## Determinism Contract
- Assertion helpers return `true` on success and `false` on mismatch.
- `clg test` maps `false` to deterministic assertion failure contracts (`C139`, `failure_id=assertion.bool_false`).
- Expected-outcome mismatches in test-plan policy map to `C141` with deterministic diff payload.

## Deferred APIs
- `assert_eq_bytes`, `assert_eq_bytes_ct`, `assert_not_zero_u64`
- `assert_error::{code,equals}`
- generic `assert_eq<T>`
- `assert_eq_string`

## Generic Equality Policy
- `assert_eq<T>` is intentionally deferred.
- Generic activation requires an explicit equality capability contract in typing plus deterministic diagnostics policy.
- Until then, use typed assertion methods only.
