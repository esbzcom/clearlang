# Unit Testing Guide (Gate D Contract)

This guide documents the Phase `25.3` unit-testing contract for ClearLang.

Status:
- `clg test` runner core is shipped for deterministic serial execution (`25.3.2`, `25.3.10`, `25.3.11`, `25.3.14.1`, `25.3.20`).
- Deterministic mock execution remains fail-closed until the mock runner slices land (`25.3.4` family).

## Design Constraints

- Deterministic by default: serial execution, stable ordering, stable report payloads.
- Machine-readable by default: JSON errors/events follow the primary CLI contract.
- Fail-closed safety: invalid mock binding or policy mismatch must fail deterministically.
- Release-grade isolation: `tests/` and `tests/mocks/` are test-only and must never enter production release artifacts.
- Assurance alignment: testing workflow must stay compatible with `release == proved`.

## Canonical Layout

```
<project-root>/
  main.clear
  domain/...
  services/...
  tests/
    unit/
      *_tests.clear
    mocks/
      common/
        ...
      <set-name>/
        ...
    test-plan.json
```

Rules:
- Unit test functions use `test_*` names.
- Mock files mirror production module paths under `tests/mocks/<set>/...`.
- Missing/invalid mock bindings fail closed.

## Deterministic Mock Binding

`tests/test-plan.json` (schema v1) maps tests to mock sets:

```json
{
  "schema_version": 1,
  "default_mock_sets": ["common"],
  "cases": [
    {
      "test_id": "tests/unit/discount_tests.clear::test_discount_uses_common_rate",
      "mock_sets": ["common"]
    },
    {
      "test_id": "tests/unit/discount_tests.clear::test_discount_uses_promo_rate",
      "mock_sets": ["common", "promo"],
      "timeout_ms": 120000
    }
  ]
}
```

Binding policy:
- Start from `default_mock_sets`.
- Apply case `mock_sets` in listed order.
- Later sets override earlier sets deterministically.
- Signature/effect mismatch is a deterministic failure.

## Planned CLI Shape (Minimal)

Minimal `clg test` surface:
- `<path?>`
- `--filter`
- `--report human|json|junit`

Non-essential flags are deferred; policy remains controlled via `tests/test-plan.json` and deterministic runner defaults.

## Execution and Reliability Policy

- Serial-by-default execution.
- Per-test timeout default: `120000ms` (2 minutes), policy override via test plan.
- No automatic retries in default/CI/release-precheck paths.
- Shared-state leak across tests is rejected by contract (fresh context or deterministic reset).

## Report Contract (v1)

JSON report (`--report json`) has pinned `schema_version: 1` and stable core fields:
- summary: `status`, `discovered`, `selected`, `executed`, `passed`, `failed`
- per-case: `id`, `file`, `function`, `timeout_ms`, `mock_sets`, `status`
- per-case failure metadata:
  - `failure_kind` (`timeout|runtime|assertion_false`)
  - `failure_code` (`C137|C138|C139`)
  - `reason`
- per-case replay/capture contract:
  - `captured_stdout`, `captured_stderr` (currently empty-string by default)
  - `replay.argv` (single-test replay flow, canonical tokenized command)

Stability policy:
- additive-only changes for optional fields in `schema_version: 1`
- breaking/removal changes require a schema-version bump

Code-to-failure mapping (test stage):
- plan/schema governance: `C135`
- mock binding/policy: `C136`
- timeout execution failure: `C137`
- runtime/harness execution failure: `C138`
- assertion false (`Bool` false) execution failure: `C139`

## JSON Events Contract

With `--json-events`, `clg test` emits NDJSON on `stderr` with `schema_version: 1`.
Stable stage set includes:
- pipeline stages (`test_contract`, `test_discover`, `test_plan`, `test_compile`, `test_execute`)
- per-test stage (`test_case`) with deterministic `start`/`finish` events and structured fields (`test_id`, `timeout_ms`, `status`, `failure_code`).

## Release Safety

Release workflows must enforce:
- no module-graph references to `tests/` or `tests/mocks/`,
- no test/mock paths in release bundle/import-map,
- tamper evidence via signature/hash verification gates.

## Example

Reference example project:
- `examples/projects/testing/`
- `examples/projects/testing/README.md`
