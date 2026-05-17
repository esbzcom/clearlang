# Phase 25.0.1 - Milestone 3 Design Lock (`release == proved`)

## Status
Design lock for `25.0.1` in `docs/TODO.md`.

## Goal
Lock Milestone 3 release assurance policy before implementation so release behavior is deterministic, theorem-grade, and fail-closed.

## Design Principles Check
- Simple for users: one release rule for production artifacts: `release == proved`.
- AI-friendly: explicit proof thresholds, stable status vocabulary, deterministic gate outcomes.
- Provably correct: theorem-grade claims require complete proof closure for release-enabled surface.
- Crypto-focused: assurance claims remain bound to signed artifacts and explicit trust/trust-policy inputs.

## Locked Policy Baseline
1. Release policy:
   - production release is allowed only when theorem-grade threshold is satisfied.
2. Theorem-grade status token:
   - `proved_all` (passes all thresholds),
   - `not_proved_all` (any threshold miss).
3. Allowed proof outcomes for release:
   - only `proved`.
4. Disallowed proof outcomes for release:
   - `failed`, `unknown`, `timeout`, `assumed`.
5. Compiler mode for release:
   - strict mode only; no fallback to permissive/standard for release commands.

## Proof Completion Thresholds (Explicit)
For a release candidate, all thresholds below MUST be true:

1. VC completion threshold:
   - `proved_count == expected_vc_count`.
2. Zero-failure threshold:
   - `failed_count == 0`.
3. Zero-unknown threshold:
   - `unknown_count == 0`.
4. Zero-timeout threshold:
   - `timeout_count == 0`.
5. Zero-assumption threshold:
   - `assumed_count == 0` across release-enabled surfaces.
6. Strict-mode threshold:
   - verification and release packaging run under strict policy inputs.
7. Determinism threshold:
   - identical inputs produce identical proof-status summary and gate decision.

Definition:
- `expected_vc_count` is the deterministic VC total for the release-enabled symbol set under locked inputs (source, lockfiles, trust policy, host profile, solver config).

## Fail-Closed Policy (Locked)
Release MUST be blocked on any of the following:
1. Any threshold failure listed above.
2. Missing/unreadable proof artifact or malformed proof summary.
3. Missing/unreadable required strict inputs (`clg.lock.json`, trust/profile policy inputs, required signatures).
4. Signature/hash verification failure for assurance artifacts.
5. Any ambiguity in assurance status computation (for example, missing counters required for deterministic evaluation).

No soft-pass behavior:
- no warning-only downgrade,
- no automatic retry with weaker assurance profile,
- no implicit policy relaxation.

## Scope Boundary
- This lock defines policy and thresholds only.
- Implementation slices remain tracked in:
  - `25.0.2` through `25.0.15` (policy enforcement/details),
  - `25.1.x` (solver/proof engine closure),
  - `25.2.x` (release command orchestration),
  - `25.3.x` and later (test and workflow gates).

## Non-Goals (This Slice)
1. No new syntax keyword (`theorem`) in Milestone 3.
2. No solver replacement strategy changes beyond threshold definition.
3. No distribution/installer policy details (handled in `25.6.x`).

## Exit Criteria for 25.0.1
1. This design lock is published in `docs/design/`.
2. `docs/TODO.md` marks `25.0.1` complete and references this file.
3. Milestone 3 implementation tasks reference these thresholds as the authoritative baseline.

## References
- `docs/TODO.md`
- `docs/TODO.md`
- `docs/design/phase-19.5.3-release-policy-gates.md`
- `docs/design/phase-23.0-runtime-loader-linker-design-lock.md`
- `docs/proofs/proof-coverage-matrix.md`
