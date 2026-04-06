# Phase 25.3.32 - `std::unit` Rollout Governance Policy

## Status
Policy lock for `25.3.32` in `docs/TODO.md`.

## Goal
Lock a non-breaking rollout policy for `std::unit` so Gate D ships the minimal assertion subset now, while preserving stable compatibility for later additions.

## Locked Policy
1. Gate D shipped subset is fixed as:
   - `assert_true(cond: Bool, msg: String) -> Bool`
   - `assert_eq_int(actual: Int, expected: Int, msg: String) -> Bool`
   - `assert_eq_bool(actual: Bool, expected: Bool, msg: String) -> Bool`
   - `fail(msg: String) -> Bool`
2. The broader `std::unit` v1 list in `docs/testing.md` is roadmap scope, not shipped-by-default scope for Gate D.
3. Post-Gate-D expansion must be additive only:
   - no signature breaking changes for shipped methods,
   - no behavior breaking changes to baseline Bool-return assertion semantics.
4. `clg test` CLI/test-plan surface stays unchanged by this policy lock.

## Design Principle Alignment
- Simple for users: minimal shipped subset is explicit and stable.
- AI-friendly: deterministic, versioned assertion surface avoids hidden contract drift.
- Provably correct: production proof policy remains unchanged (`release == proved`).
- Crypto-focused safety: no test assertion expansion is allowed to relax fail-closed release behavior.

## References
- `docs/TODO.md`
- `docs/testing.md`
- `docs/design/phase-25.3-evidence-index.md`
