# Phase 30.0.0 - Milestone 4 Planning Lock

Date: 2026-06-27
Status: Locked
Owner: roadmap-owner

## Purpose

Milestone 3 closes the current production contract.

Milestone 4 exists for additive post-release work only. Its purpose is to extend the product
carefully without reopening Milestone 3 closure decisions or weakening the current fail-closed
release, proof, provenance, and shared-std contract.

## Current Starting Point

After `29.8.1`, the project has:

1. a closed Milestone 3 production roadmap
2. `embedded` as the default supported production delivery mode
3. `shared` as a supported explicit opt-in production mode for the locked bundled package set
4. GA binary operations for Windows and Linux, with macOS still explicitly preview-only
5. deterministic release, proof, runtime, and provenance workflows backed by documentation and CI

That means Milestone 4 should optimize for disciplined expansion, not backlog accumulation.

## Locked Invariants

All Milestone 4 work must preserve the following:

1. Milestone 3 closure documents remain authoritative unless a new design lock explicitly revises a
   contract
2. fail-closed proof, trust, ABI, digest, provenance, replay, rollback, and signer-rotation
   behavior may not be weakened for convenience
3. unsupported workflows must continue to reject deterministically rather than silently degrade
4. user-facing behavior must stay documented in the same patch as the behavior change

## Prioritization Rules

Milestone 4 prioritization must continue to follow the README principles:

- simple for users: prefer work that reduces support-matrix confusion, installation friction, or
  activation ambiguity
- AI-friendly: prefer canonical matrices, explicit policies, and machine-checkable gates over
  heuristic workflows
- provably correct: prefer slices that preserve or extend the current release and proof guarantees
- crypto-focused: prefer audit-grade runtime, release, supply-chain, and chain-facing correctness
  improvements before broad new surface area

## Non-Goals

This lock does not authorize:

1. reopening Milestone 3 production-closure decisions as unfinished work
2. broad new language features without a separate milestone and bounded success target
3. defaulting from `embedded` to `shared` without a separate product decision
4. broadening chain helpers or shared package surfaces in the same slice as platform/distribution
   policy changes

## Success Target

Milestone 4 planning is successful only when:

1. the post-release candidate set is inventoried explicitly
2. the first bounded Milestone 4 execution slice is selected and justified
3. deferred tracks are named explicitly so future work does not drift

## Next Task

The next task after this planning lock is `30.1.0`:

- inventory the post-release candidate tracks that remain intentionally deferred after Milestone 3

## References

- `README.md`
- `docs/design/phase-29.8.1-milestone-3-closure-lock.md`
- `docs/design/phase-25.6.0-binary-ga-and-provenance-policy-lock.md`
- `docs/design/phase-25.6.13-binary-publication-policy-lock.md`
- `docs/release/milestone_3-binary-operations.md`
- `docs/todo/milestone_4_roadmap.md`
