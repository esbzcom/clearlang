# Phase 25.3.0 - Gate D Design Lock (Unit Test + Mock Runner)

## Status
Design lock for `25.3.0` in `docs/TODO.md`.

## Goal
Lock Gate D scope and fail-closed contracts for `clg test` before final closeout.

## Design-Principle Alignment
Gate D follows project design principles in `README.md`:
1. **Simple for users**: production CLI surface for tests stays minimal (`path`, `--filter`, `--report`).
2. **AI-friendly**: deterministic ordering, diagnostics, and machine-readable report/event contracts.
3. **Provably correct**: test workflows cannot weaken production policy (`release == proved` remains enforced by release gates).
4. **Crypto-focused safety**: deterministic, fail-closed behavior for mock/path/runtime/release isolation contracts.

## Scope (Gate D)
- Canonical test layout and discovery under `tests/`.
- Deterministic serial test runner and stable report schemas.
- Deterministic mock selection via `tests/test-plan.json`.
- Fail-closed safety gates for plan/schema, mock binding, path safety, runtime safety, and release isolation.
- CI + `xtask release-precheck` integration and cross-platform parity.

## Non-Goals
- No broad test-framework feature expansion beyond locked Gate D contract.
- No relaxation of release policy (`release == proved`).
- No compatibility-first fallback behavior that hides contract failures.

## Deterministic/Fall-Closed Contracts
- Discovery order, selected case order, and report shape are deterministic.
- Invalid test-plan schema/bindings fail closed with stable diagnostics.
- Mock loading rejects path traversal/symlink escape and out-of-root bindings.
- Test worker runtime failures map to deterministic failure reasons/codes.
- Release/build flows reject test/mock path references for production artifacts.

## Gate D Exit Criterion
`clg test` is considered shipped primary UX only when required `25.3.x` gates are green and TODO evidence is marked complete.

## Locked Implementation Order
Gate D was executed in dependency order:
1. contract/layout/schema foundations (`25.3.1`, `25.3.19`, `25.3.23`)
2. runner core (`25.3.2`, `25.3.10`, `25.3.11`, `25.3.14.1`)
3. machine-readable outputs (`25.3.3`, `25.3.9`, `25.3.21`)
4. deterministic mock system (`25.3.4`, `25.3.15`, `25.3.16`, `25.3.17`, `25.3.24`)
5. release/proved safety gates (`25.3.12`, `25.3.13`, `25.3.14`)
6. CI/release-precheck + platform determinism (`25.3.7`, `25.3.22`)
7. migration/quality/docs closeout (`25.3.6`, `25.3.8`, `25.3.18`, `25.3.25`, `25.3.26`)

## References
- `docs/TODO.md`
- `docs/testing.md`
- `docs/testing-quality-matrix.md`
- `README.md`
