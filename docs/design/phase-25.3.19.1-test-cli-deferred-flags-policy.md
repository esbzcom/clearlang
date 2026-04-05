# Phase 25.3.19.1 - `clg test` Deferred Flags Policy Lock

## Status
Policy lock for `25.3.19.1` in `docs/TODO.md`.

## Goal
Keep `clg test` production CLI surface minimal and deterministic unless a concrete production workflow requires expansion.

## Locked Minimal Surface
`clg test` production contract:
- `<path?>`
- `--filter`
- `--report human|json|junit`

## Deferred Non-Essential Flags
The following test-command flags remain deferred:
- `--plan`
- `--mock-set`
- `--timeout-ms`
- `--fail-fast`
- `--list`

## Deterministic Replacement Policy
Equivalent behavior is controlled by:
- `tests/test-plan.json` (`schema_version`, `default_mock_sets`, per-case `mock_sets`, per-case `timeout_ms`)
- deterministic runner defaults (serial run-all, stable ordering, explicit rerun)

## Re-Open Criteria
Any deferred flag may be proposed only when:
1. a concrete production workflow cannot be expressed with current contracts,
2. deterministic schema/diagnostic compatibility impact is documented,
3. fail-closed tests are added for the new surface.

## References
- `docs/TODO.md`
- `docs/testing.md`
- `crates/cli/src/main.rs`
