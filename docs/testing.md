# Unit Testing Guide (Gate D Contract)

This guide documents the Phase `25.3` unit-testing contract for ClearLang.

Status:
- `clg test` runner core is shipped for deterministic serial execution (`25.3.2`, `25.3.10`, `25.3.11`, `25.3.14.1`, `25.3.20`).
- Deterministic mock execution is shipped with explicit per-test bindings and fail-closed path safety (`25.3.4`, `25.3.15`, `25.3.16`, `25.3.17`, `25.3.24`).
- CI/release-precheck gates are shipped for `clg test` schema validation and cross-platform parity (`25.3.7`, `25.3.22`).
- Closeout quality/runtime/docs gates are shipped for matrix coverage, balanced mock confidence, runtime worker safety, and contract drift detection (`25.3.6`, `25.3.8`, `25.3.18`, `25.3.25`, `25.3.26`).
- Gate D policy/design locks are published for overall scope, test proof-mode default, and deferred-flag contracts (`25.3.0`, `25.3.5`, `25.3.19.1`).

## Design Constraints

- Deterministic by default: serial execution, stable ordering, stable report payloads.
- Machine-readable by default: JSON errors/events follow the primary CLI contract.
- Fail-closed safety: invalid mock binding or policy mismatch must fail deterministically.
- Release-grade isolation: `tests/` and `tests/mocks/` are test-only and must never enter production release artifacts.
- Assurance alignment: testing workflow must stay compatible with `release == proved`.
- Proof scope separation: theorem-grade assurance (`proved_all`) is required for production source/release artifacts, not for `tests/` sources executed by `clg test`.

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
- When `default_mock_sets` is non-empty, each selected test must have an explicit `cases[]` entry.
- Symlink/out-of-root mock paths are rejected with deterministic `C136` diagnostics.

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
- Effective mock-set execution is per-test and isolated (fresh compile + fresh runtime context per case).
- Signature/effect mismatch is a deterministic failure.

## CLI Shape (Minimal, Shipped)

Minimal `clg test` surface:
- `<path?>`
- `--filter`
- `--report human|json|junit`

Non-essential flags are deferred; policy remains controlled via `tests/test-plan.json` and deterministic runner defaults.

## Proof-Mode Policy (25.3.5)

- `clg test` uses one deterministic default policy for Gate D unit-test workflows.
- `clg test` does not expose a proof-mode selector in the minimal production CLI contract.
- Unit-test pass/fail does not weaken production release assurance policy.
- `release == proved` remains enforced by release gates; theorem-grade release artifacts still require `proved_all`.
- `clg test` is a deterministic quality runner for test behavior; it is not a theorem-grade certification gate for test sources.

## Unit Assertions Package (25.3.27)

Canonical package name:
- Use `std::unit` as the standard assertion package namespace.
- Do not use top-level `unit::` as the canonical std package name.
- If concise call sites are preferred, import alias is allowed (`import std::unit as unit`) and tests may call `unit::...`.

Locked production surface (v1 target API):
- `pass() -> Bool`
- `fail(msg: String) -> Bool`
- `assert_true(cond: Bool, msg: String) -> Bool`
- `assert_false(cond: Bool, msg: String) -> Bool`
- `assert_eq_int(actual: Int, expected: Int, msg: String) -> Bool`
- `assert_eq_bool(actual: Bool, expected: Bool, msg: String) -> Bool`
- `assert_eq_u64(actual: U64, expected: U64, msg: String) -> Bool`
- `assert_eq_u128(actual: U128, expected: U128, msg: String) -> Bool`
- `assert_eq_u256(actual: U256, expected: U256, msg: String) -> Bool`
- `assert_eq_string(actual: String, expected: String, msg: String) -> Bool`
- `assert_ne_int(actual: Int, expected: Int, msg: String) -> Bool`
- `assert_ne_string(actual: String, expected: String, msg: String) -> Bool`
- `assert_lt_int(actual: Int, expected: Int, msg: String) -> Bool`
- `assert_le_int(actual: Int, expected: Int, msg: String) -> Bool`
- `assert_gt_int(actual: Int, expected: Int, msg: String) -> Bool`
- `assert_ge_int(actual: Int, expected: Int, msg: String) -> Bool`
- `assert_contains_string(haystack: String, needle: String, msg: String) -> Bool`
- `assert_starts_with_string(actual: String, prefix: String, msg: String) -> Bool`
- `assert_ends_with_string(actual: String, suffix: String, msg: String) -> Bool`
- `assert_eq_bytes(actual: Bytes, expected: Bytes, msg: String) -> Bool` (enabled when `Bytes` is in std-core scope)

Gate D implementation subset (initial ship target):
- `assert_true(cond: Bool, msg: String) -> Bool`
- `assert_eq_int(actual: Int, expected: Int, msg: String) -> Bool`
- `assert_eq_bool(actual: Bool, expected: Bool, msg: String) -> Bool`
- `fail(msg: String) -> Bool`

Semantics:
- Assertion success returns `true`.
- Assertion mismatch returns `false` (deterministic `C139` mapping in `clg test`).
- Assertion helpers are deterministic and test-quality focused; they do not change production proof policy (`release == proved` remains enforced by release gates).

Phased rollout governance (25.3.32):
- Gate D ships only the minimal subset (`assert_true`, `assert_eq_int`, `assert_eq_bool`, `fail`).
- Remaining v1 methods listed above are roadmap targets and must be added additively only.
- No breaking changes are allowed to already-shipped `std::unit` signatures or baseline Bool-return semantics.

`assert_eq_bytes` activation policy (25.3.33):
- `assert_eq_bytes` remains reserved until `Bytes` is confirmed in std-core scope with deterministic typing/runtime support for this assertion path.
- Pre-activation behavior is fail-closed and deterministic: calls to `std::unit::assert_eq_bytes` are rejected as unknown function (`T001` at type stage).
- Activation requires explicit policy update plus deterministic compatibility tests before moving from reserved to shipped.

Example:

```clear
import std::unit as unit
import services::discount

function test_discount_uses_promo_rate() -> Bool {
    unit::assert_eq_int(discount::compute_discount(200), 50, "promo discount should be 50")
}
```

## Deferred Flags Policy (25.3.19.1)

Deferred non-essential `clg test` flags:
- `--plan`
- `--mock-set`
- `--timeout-ms`
- `--fail-fast`
- `--list`

Deterministic alternatives remain:
- `tests/test-plan.json` for per-case policy (`mock_sets`, `timeout_ms`)
- serial run-all default with explicit rerun for selection/replay

## Execution and Reliability Policy

- Serial-by-default execution.
- Single-process policy: Gate D does not support concurrent `clg test` processes targeting the same project/workspace; run one active invocation per project root.
- Per-test timeout default: `120000ms` (2 minutes), policy override via test plan.
- No automatic retries in default/CI/release-precheck paths.
- Shared-state leak across tests is rejected by contract (fresh context or deterministic reset).

## CI Gate Contract

- `cargo run -p xtask -- release-precheck` includes fail-closed `clg test examples/projects/testing --report json` schema validation.
- Milestone parity gate runs `milestone3_test_parity` on Windows and Linux and compares emitted summaries byte-for-byte.
- `milestone3-release-train-gate` depends on both proof parity and `clg test` parity compare jobs before milestone tag release gating.

## Coverage Matrix Contract (25.3.6)

- "Enough tests" is defined by scenario completeness, not numeric thresholds.
- Canonical matrix: `docs/testing-quality-matrix.md`.
- Gate D completion requires positive + negative deterministic evidence across parser/type/contracts/runtime/mock/release-parity scenarios.

## Migration Policy (25.3.8)

- Canonical unit-test source-of-truth for `clg test` is `tests/` (`tests/unit`, `tests/mocks`, `tests/test-plan.json`).
- `clearlang-tests/` remains a temporary legacy sample/e2e corpus for non-`clg test` compile/run coverage while migration finalization is deferred.
- Cutover trigger criteria:
  - `clg test` contract and mock behavior are stable across replay/parity gates.
  - canonical `tests/` coverage matrix rows are complete and green in CI.
  - release-precheck and release-train gates no longer depend on legacy-only fixtures.

## Balanced Mock Confidence Gate (25.3.18)

- Gate D quality policy requires critical-path evidence to include both:
  - non-mocked execution, and
  - mocked execution.
- `xtask release-precheck` enforces this in the canonical testing example via fail-closed schema/quality checks on `clg test --report json`.
- Purpose: avoid mock-only confidence for release-critical behavior.

## Runtime Worker Safety Policy (25.3.25)

- `clg test` workers apply deterministic runtime guardrails beyond timeout:
  - fuel limit,
  - memory-growth limit,
  - crash capture (`worker_crash`) with deterministic runtime status mapping.
- Failures from these safety guardrails remain `failure_kind=runtime` with `failure_code=C138` and stable reason prefixes:
  - `fuel_exhausted: ...`
  - `memory_limit: ...`
  - `worker_crash: ...`

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

Shipped Gate D release-safety diagnostics:
- `C128`: production build/release module-graph isolation failure (`tests/` or `tests/mocks/` reference).
- `C129`: release artifact scan failure (strict import-map/release bundle evidence invalid or includes test/mock source paths).
- `V003`: signature-bound hash mismatch (including mock/test substitution tamper attempts).

## Example

Reference example project:
- `examples/projects/testing/`
- `examples/projects/testing/README.md`
