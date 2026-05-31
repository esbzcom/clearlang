# Phase 26.5 - Test Assertion Extensions Lock (Std Gate F)

## Scope
This lock enables deterministic advanced assertion behavior for `clg test` without changing the minimal CLI flag surface.

## 26.5.1 Expected-Outcome Semantics
- `tests/test-plan.json` `schema_version: 1` now supports optional per-case `expected_outcome`:
  - `kind`: `pass | assertion_false | runtime | timeout`
  - optional `failure_code` (format `Cddd`)
  - optional `reason_contains` (single-line non-empty substring)
- If `expected_outcome` is omitted, behavior remains `pass`.
- Matching expected failed outcomes are treated as passing test cases.
- Expected-outcome mismatches fail deterministically with:
  - `failure_code = C141`
  - `failure_kind = assertion_mismatch`
  - stable `failure_id` taxonomy (`expectation.kind_mismatch`, `expectation.code_mismatch`, `expectation.reason_mismatch`)

## 26.5.2 Deterministic Assertion Diff + Failure IDs
- Per-test report payload now includes optional:
  - `failure_id` (stable string taxonomy)
  - `assertion_diff` (deterministic shape, `schema_version: 1`)
- Current deterministic IDs:
  - timeout: `timeout.elapsed`
  - runtime: `runtime.fuel_exhausted | runtime.memory_limit | runtime.worker_crash | runtime.trap`
  - baseline Bool assertion false: `assertion.bool_false`
  - expected-outcome mismatch: `expectation.kind_mismatch | expectation.code_mismatch | expectation.reason_mismatch`
- `assertion_diff` shape:
  - `schema_version`, `kind`, `expected`, `actual`
  - optional `expected_failure_code`, `actual_failure_code`, `reason_contains`, `actual_reason`

## 26.5.3 Equality-Capability Policy for `assert_eq<T>`
- Generic `assert_eq<T>` remains deferred until an explicit equality capability contract is locked in the type system.
- Gate F enables only explicit typed methods with deterministic behavior:
  - `assert_eq_int`
  - `assert_eq_bool`
  - `assert_eq_u64`
- Policy is fail-closed for unsupported generic equality paths; no implicit fallback generic assertion API is introduced.

## Determinism/Compatibility Rules
- `tests/test-plan.json` remains `schema_version: 1`; extensions are additive and optional.
- Existing `C137/C138/C139` behavior remains unchanged for baseline cases.
- `C141` is reserved for expected-outcome contract mismatches only.
- JSON report remains `schema_version: 1` with additive optional fields (`failure_id`, `expected_outcome`, `assertion_diff`).
