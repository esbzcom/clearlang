# Phase 25.0.4 - Fail-Closed Release Enforcement

## Status
Design lock for `25.0.4` in `docs/TODO.md`.

## Goal
Enforce fail-closed release gating so production release is blocked on any non-proved proof outcome.

## Locked Rule
Release/publish gates MUST reject when any proof outcome is in:
- `failed`
- `unknown`
- `timeout`
- `assumed`

Accepted release outcome set:
- only fully proved status (`proved_all`) for the release surface.

## Enforcement Contract
1. Enforcement point:
   - release verification gate before publish/tag.
2. Input sources:
   - signed assurance manifest + signed module/proof hashes,
   - proof artifacts used to compute deterministic proof outcome summary.
3. Evaluation:
   - fail if any disallowed outcome appears in summary,
   - fail if required summary fields are missing/ambiguous (fail closed).
4. Output:
   - deterministic policy failure (`V005`) with explicit reason text.

## Deterministic Rejection Matrix
1. Any VC `failed` -> reject.
2. Any VC `unknown` -> reject.
3. Any VC `timeout` -> reject.
4. Any assumption boundary present (`assumed`) -> reject.
5. Any missing/partial proof summary -> reject.

No warning-only mode is allowed for production release gates.

## Scope Boundary
- This slice locks fail-closed policy semantics.
- CLI field wiring and command flags are tracked in:
  - `25.0.5` (`proof_status` emission),
  - `25.0.6` (`clg verify --require-assurance proved_all`),
  - `25.0.7` (release compile/publish profile gate).

## Non-Goals
1. No change to language syntax.
2. No solver-model feature expansion in this slice.
3. No downgrade path for production release policy.

## Exit Criteria for 25.0.4
1. Fail-closed rejection conditions are documented and locked.
2. `docs/TODO.md` marks `25.0.4` complete with this reference.
3. Release-process docs state the same rejection set.

## References
- `docs/TODO.md`
- `docs/design/phase-25.0.1-milestone-3-design-lock.md`
- `docs/design/phase-25.0.2-theorem-grade-certification-policy.md`
- `docs/design/phase-25.0.3-release-equals-proved-policy.md`
- `docs/release-process.md`
