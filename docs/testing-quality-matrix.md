# Gate D Quality Matrix (25.3.6)

This matrix defines "enough test cases" for Gate D by scenario/risk coverage, not by a numeric quota.

Policy:
- No fixed numeric thresholds (no minimum test-count target).
- Coverage is complete when each scenario below has deterministic positive and negative evidence.
- Gaps are tracked by missing scenario rows, not percentage math.

## Scenario Matrix

| Scenario | Positive Evidence | Negative Evidence |
| --- | --- | --- |
| Parser + discovery contract | deterministic discovery of `test_*` in canonical `tests/unit` layout | invalid layout/signature/duplicate-id fails with deterministic `C134` |
| Test-plan governance | canonical sorted `tests/test-plan.json` with stable bindings | malformed schema/unsorted/duplicate/unknown IDs fail with deterministic `C135` |
| Mock binding + safety | explicit per-test mock-set execution with deterministic override order | missing/unknown/unsafe mock path or binding-policy breach fails with deterministic `C136` |
| Runtime execution + assertion semantics | passing tests return stable `ok` report with replay fields; expected-outcome policy can deterministically classify known negative paths as pass | assertion false/runtime/timeout failures map to deterministic `C139/C138/C137`; expected-outcome mismatch maps to deterministic `C141` |
| Runtime worker safety limits | deterministic fuel + memory limits and crash handling policy under test runner | `clg test` integration covers fail-closed `C138` runtime mapping; fuel/memory reason prefixes are enforced by deterministic unit contracts |
| Report/event contracts | stable `human|json|junit` reports and NDJSON event stages | schema drift or missing required fields fails Gate D CI/precheck checks |
| Release isolation/tamper safety | release graph and artifacts include production modules only | any test/mock reference or substitution fails deterministic `C128/C129/V003` gates |
| Cross-platform determinism | Windows + Linux parity summaries match for canonical `clg test` fixture | parity mismatch fails release-train parity compare gate |
| Balanced mock confidence | canonical critical-path suite includes non-mocked and mocked executions | mock-only critical-path suite fails deterministic quality gate |

## Completion Rule

Gate D quality completeness is satisfied when:
1. All matrix rows above have live evidence in tests/docs/gates.
2. No row is marked "partial" or "deferred" for milestone release scope.
3. CI and `xtask release-precheck` keep the same deterministic pass/fail contract.
