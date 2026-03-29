# Phase 25.1.6 - Solver Runtime Safety and Isolation Policy

## Status
Design lock for `25.1.6` in `docs/TODO.md`.

## Goal
Define fail-closed solver runtime safety behavior before enabling theorem-prover execution in strict release workflows.

## Locked Runtime Policy
1. Execution envelope:
   - solver runs as an external process boundary with explicit input/output files or pipes,
   - solver process inherits no implicit network access requirement.
2. Resource limits:
   - per-VC timeout from locked solver profile,
   - total proof run timeout budget from locked solver profile,
   - bounded memory budget configured by policy (implementation may use platform-specific enforcement).
3. Kill semantics:
   - timeout forces process termination,
   - termination failure is a hard error.
4. Crash semantics:
   - crash/abnormal exit is fail-closed (`C124` or `C126` depending on stage).

## Diagnostic Mapping (Locked)
1. solver missing/unavailable -> `C124`
2. solver timeout -> `C125`
3. malformed/missing proof artifact or inconsistent hash binding -> `C126` / `V006`
4. deterministic replay mismatch across identical inputs -> `C127`

## Security Baseline
1. Inputs to solver must be deterministic and fully derived from strict build context.
2. Proof outputs consumed by release/verify gates must be hash-bound and signature-bound.
3. Unknown/partial solver outputs are treated as non-release evidence only.

## Non-Goals
1. This slice does not pick a sandbox technology per platform.
2. This slice does not define distributed/remote solver execution.

## References
- `docs/design/phase-25.1.2-solver-diagnostics-reservation.md`
- `docs/design/phase-25.1.4-deterministic-solver-profile-lock.md`
