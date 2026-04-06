# Phase 25.3.33 - `assert_eq_bytes` Activation Policy

## Status
Policy lock for `25.3.33` in `docs/TODO.md`.

## Goal
Keep `std::unit::assert_eq_bytes` reserved until activation prerequisites are met, with deterministic pre-activation failure behavior.

## Locked Policy
1. `std::unit::assert_eq_bytes(actual: Bytes, expected: Bytes, msg: String) -> Bool` is reserved and not part of the Gate D shipped subset.
2. Activation is blocked until `Bytes` support for this assertion path is explicitly confirmed in std-core scope with deterministic typing/runtime coverage.
3. Pre-activation behavior is deterministic and fail-closed:
   - calls to `std::unit::assert_eq_bytes` must fail at type stage as unknown function (`T001`),
   - no implicit fallback/alias to other assertions is allowed.
4. Activation requires:
   - explicit policy update,
   - deterministic compatibility tests for typing/runtime/report contracts.

## Rationale
- Prevents accidental surface drift before runtime/type guarantees exist.
- Keeps Gate D contracts stable and predictable for users and tooling.
- Preserves release assurance boundaries (`release == proved`) by avoiding partially-specified assertion behavior.

## References
- `docs/TODO.md`
- `docs/testing.md`
- `docs/design/phase-25.3-evidence-index.md`
