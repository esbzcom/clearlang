# Phase 25.0.2 - Theorem-Grade Certification Policy (`proved_all`)

## Status
Design lock for `25.0.2` in `docs/TODO.md`.

## Goal
Define deterministic certification rules for theorem-grade status in Milestone 3.

## Design Principles Check
- Simple for users: one top-level status for release certification: `proved_all`.
- AI-friendly: machine-checkable predicates with deterministic pass/fail decision.
- Provably correct: theorem-grade status is granted only when proof closure is complete.
- Crypto-focused: assumption boundaries in crypto/bitwise/unsigned surfaces block theorem-grade status.

## Certification Unit
Certification is evaluated for a release candidate artifact set:
1. release-target package/module set,
2. strict policy inputs (`clg.lock.json`, trust policy, host profile, solver configuration),
3. emitted VC/proof artifacts for that exact input set.

## `proved_all` Definition (Locked)
`proved_all` is true iff all predicates below hold:

1. Strict mode predicate:
   - candidate is built/verified under strict mode policy.
2. Purity predicate:
   - all release-surface functions covered by theorem-grade claim are `pure`.
3. VC completeness predicate:
   - every expected VC for release-surface symbols is emitted and accounted for.
4. VC outcome predicate:
   - every accounted VC has status `proved`.
5. Zero assumptions predicate:
   - no assumption boundary is present on release-enabled surfaces, including:
     - `unsigned.int_model`
     - `bitwise.uninterpreted`
     - `crypto.uninterpreted`
6. Determinism predicate:
   - repeated verification on identical inputs yields identical aggregate certification result.

If any predicate fails, status is `not_proved_all`.

## Required Counters/Inputs for Deterministic Evaluation
The certification evaluator must consume stable counters from proof artifacts:
1. `expected_vc_count`
2. `proved_count`
3. `failed_count`
4. `unknown_count`
5. `timeout_count`
6. `assumed_count`
7. strict-mode claim marker (`release_grade_trust=true` equivalent strict claim)

Minimum deterministic checks:
- `expected_vc_count == proved_count`
- `failed_count == 0`
- `unknown_count == 0`
- `timeout_count == 0`
- `assumed_count == 0`

## Fail-Closed Evaluation Rules
Certification must fail (`not_proved_all`) when:
1. any required counter is missing or malformed,
2. proof artifacts do not match release target inputs,
3. strict policy inputs are missing or invalid,
4. verification output is partial/ambiguous.

No implicit downgrade to release-acceptable status is allowed.

## Scope Boundary
- This slice locks theorem-grade policy semantics only.
- Artifact field wiring and CLI switch behavior are handled in later tasks:
  - `25.0.5` (artifact/signature payload status),
  - `25.0.6` (verify policy gate),
  - `25.0.7` (release compile/publish gate).

## Non-Goals
1. No new language keyword (`theorem`) in Milestone 3.
2. No solver feature expansion in this slice.
3. No change to existing `L0-L3` tier schema in this slice.

## Exit Criteria for 25.0.2
1. Policy definition for `proved_all` is documented and locked.
2. `docs/TODO.md` marks `25.0.2` complete with reference to this doc.
3. Subsequent implementation tasks (`25.0.5`/`25.0.6`/`25.0.7`) treat this doc as source of truth.

## References
- `docs/TODO.md`
- `docs/design/phase-25.0.1-milestone-3-design-lock.md`
- `docs/design/phase-19.1.1-assurance-tiers.md`
- `docs/design/phase-19.1.3-strict-l3-fail-closed.md`
- `docs/proofs/proof-coverage-matrix.md`
