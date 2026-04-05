# Phase 25.3.5 - `clg test` Proof-Mode Policy Lock

## Status
Policy lock for `25.3.5` in `docs/TODO.md`.

## Goal
Lock one deterministic proof-policy default for unit-test workflows, without weakening release assurance policy.

## Locked Policy
1. `clg test` has one deterministic default mode for Gate D workflows.
2. No `clg test` proof-mode selector flag is exposed in the minimal production CLI contract.
3. Unit tests validate deterministic compile/type/runtime behavior for test cases and mocks.
4. Production release assurance remains controlled by release gates (`release == proved`, `proved_all` required for release artifacts).
5. Test sources under `tests/` are quality-validation inputs; theorem-grade assurance requirements apply to production source/release artifacts, not test files.

## Rationale
- Keeps user and IDE surface simple and stable.
- Avoids mode-dependent drift in test behavior and report contracts.
- Prevents confusion between unit-test pass/fail and production release-proof acceptance.

## Follow-Up Policy
Advanced proof-mode selection for `clg test` is deferred until a concrete production workflow requires it and deterministic compatibility policy is defined.

## References
- `docs/TODO.md`
- `docs/testing.md`
- `docs/release-process.md`
