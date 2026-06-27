# Phase 29.9.0 - Post-29.8 Hardening Planning Lock

Date: 2026-06-27
Status: Locked
Owner: roadmap-owner

## Purpose

`29.8` closed the shared-std breadth-complete contract, but Milestone 3 still retains remaining
production-hardening and distribution-completion work.

This lock keeps that remaining work inside Milestone 3 and prevents later slices from drifting into
ad hoc feature selection or reopening already-locked shared-package decisions.

## Current Starting Point

After `29.8.1`, the project has:

1. a locked shared-std production contract for the supported bundled package set
2. `embedded` as the default supported production delivery mode
3. `shared` as a supported explicit opt-in production mode for the locked bundled package set
4. GA binary operations for Windows and Linux, with macOS still explicitly preview-only
5. deterministic release, proof, runtime, and provenance workflows backed by documentation and CI

That means the remaining Milestone 3 work should optimize for disciplined hardening, not backlog
accumulation.

## Locked Invariants

All remaining Milestone 3 work must preserve the following:

1. the `29.8.1` shared-surface closure document remains authoritative unless a new design lock
   explicitly revises that contract
2. fail-closed proof, trust, ABI, digest, provenance, replay, rollback, and signer-rotation
   behavior may not be weakened for convenience
3. unsupported workflows must continue to reject deterministically rather than silently degrade
4. user-facing behavior must stay documented in the same patch as the behavior change

## Prioritization Rules

The remaining Milestone 3 prioritization must continue to follow the README principles:

- simple for users: prefer work that reduces support-matrix confusion, installation friction, or
  activation ambiguity
- AI-friendly: prefer canonical matrices, explicit policies, and machine-checkable gates over
  heuristic workflows
- provably correct: prefer slices that preserve or extend the current release and proof guarantees
- crypto-focused: prefer audit-grade runtime, release, supply-chain, and chain-facing correctness
  improvements before broad new surface area

## Non-Goals

This lock does not authorize:

1. reopening shared-package breadth-closure decisions as unfinished work
2. broad new language features without a separate bounded success target
3. defaulting from `embedded` to `shared` without a separate product decision
4. broadening chain helpers or shared package surfaces in the same slice as platform/distribution
   policy changes

## Success Target

This planning slice is successful only when:

1. the remaining production candidate set is inventoried explicitly
2. the first bounded remaining Milestone 3 execution slice is selected and justified
3. deferred tracks are named explicitly so future work does not drift

## Next Task

The next task after this planning lock is `29.10.0`:

- inventory the remaining production tracks that stay intentionally deferred after `29.8`

## References

- `README.md`
- `docs/design/phase-29.8.1-milestone-3-closure-lock.md`
- `docs/design/phase-25.6.0-binary-ga-and-provenance-policy-lock.md`
- `docs/design/phase-25.6.13-binary-publication-policy-lock.md`
- `docs/release/milestone_3-binary-operations.md`
- `docs/todo/milestone_3_roadmap.md`
