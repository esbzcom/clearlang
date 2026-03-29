# Phase 25.1.2 - Solver-Era Diagnostics Reservation

## Status
Design lock for `25.1.2` in `docs/TODO.md`.

## Goal
Reserve stable diagnostics before solver implementation so build/verify tooling can fail closed with deterministic machine-readable codes.

## Reserved Codes

### Build
| Code | Trigger | Meaning |
| --- | --- | --- |
| `C124` | solver unavailable | Strict proof execution cannot start because configured theorem prover is unavailable. |
| `C125` | solver timeout | Strict proof execution exceeds configured timeout budget. |
| `C126` | proof artifact mismatch | Proof artifact missing/malformed, schema-invalid, or hash/binding mismatch against VC/proof claims. |
| `C127` | solver replay mismatch | Identical strict inputs produce different solver outcomes or proof artifact bytes. |

### Verify
| Code | Trigger | Meaning |
| --- | --- | --- |
| `V006` | proof artifact consistency failure | Verify gate rejects missing/malformed proof artifact or mismatch with signed solver/profile/proof claims. |

## Determinism Rules
1. Diagnostic code selection must not depend on nondeterministic ordering.
2. For multi-error situations, first-failure precedence is:
   - input unreadable/invalid schema,
   - unavailable solver/profile,
   - timeout,
   - artifact mismatch,
   - replay mismatch.
3. JSON error payload ordering remains stable under identical inputs.

## Non-Goals
1. No solver algorithm policy in this slice.
2. No diagnostics wording localization policy.
3. No runtime (`Rxxx`) code additions.

## References
- `docs/diagnostics.md`
- `crates/cli/tests/phase25_diagnostics_reservation.rs`
- `docs/design/phase-25.1.1-gate-b-design-lock.md`
