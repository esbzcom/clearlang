# Phase 30.3.0 - Multi-Platform GA Support Lock

Date: 2026-06-27
Status: Locked
Owner: release-owner

## Purpose

Lock the platform-support contract for the first Milestone 4 execution slice before CI, release,
or documentation changes land.

Milestone 3 closed with:

- `windows` and `linux` as GA baseline binary targets
- `macos` as preview and non-blocking for GA closure
- proof parity gating narrower than the binary/test support matrix

This lock defines the target post-Milestone-3 hardening contract so the next implementation slice
has an exact success condition.

## Locked Target State

The intended supported GA binary target matrix after `30.3` is:

- `windows`
- `linux`
- `macos`

That means the target end state for this slice is:

1. `macos` is promoted from preview to GA
2. GA release-train blocking gates apply to all three targets
3. deterministic proof/release parity expectations are aligned with the GA matrix

## Transition Rule

This document locks the target state for the `30.3` slice.

It does not itself activate the new GA matrix immediately.

Until `30.3.1` and `30.3.2` land, the active published release contract remains the Milestone 3
contract:

- GA: `windows`, `linux`
- preview: `macos`

The policy changes in this lock become active only when the required implementation and
documentation updates are complete.

## Required GA Blocking Contract

When `30.3` is complete, GA release-train blocking must require all of the following for every GA
target:

1. proof-parity execution on identical strict inputs
2. proof-parity comparison across all GA targets for deterministic outcome and artifact agreement
3. `clg test` parity comparison across all GA targets where the workflow claims parity support
4. binary smoke coverage for the shipped binary on each GA target
5. per-target binary reproducibility evidence under the existing witness policy
6. signed release-bundle and provenance evidence parity for every published GA artifact

No GA target may remain documentation-only or best-effort.

## Why This Lock Is Correct

### 1. It Simplifies The Supported Product Story

Users should not need to infer support from scattered workflow details.

One explicit GA matrix is simpler than:

- one platform set for proof gates
- a different platform set for binaries
- a third status for preview packaging

### 2. It Keeps The Product AI-Friendly

AI tooling benefits from one canonical support matrix with one deterministic release-blocking
policy.

That is materially better than platform-specific exceptions hidden in release checklists or CI job
shape.

### 3. It Extends Correctness Instead Of Surface Area

This slice strengthens release determinism and support guarantees without introducing new language,
runtime, or std-package semantics.

### 4. It Fits The Crypto-Focused Priority

For a smart-contract toolchain, stronger support guarantees and consistent provenance-bearing
release gates across shipped platforms are higher-value than broader convenience surfaces.

## What This Lock Does Not Authorize

This lock does not authorize:

1. installer/publication-channel expansion
2. new shared-package promotions
3. broader chain-helper or chain-runtime helper surfaces
4. defaulting from `embedded` to `shared`
5. weakening existing provenance, checksum, signer, rollback, or fail-closed proof rules

## Required Documentation State

When this slice lands, the following documentation must agree on the exact same GA matrix:

- `docs/release/milestone_3-binary-operations.md`
- `docs/release/milestone_3-release-train-checklist.md`
- `docs/release-process.md`
- `docs/design/phase-25.6.0-binary-ga-and-provenance-policy-lock.md`
- `docs/design/phase-25.6.13-binary-publication-policy-lock.md`

If those documents disagree, the slice is incomplete.

## Next Task

The next task after this lock is `30.3.1`:

- expand the trusted self-contained solver support matrix first, because full GA proof parity
  cannot execute on `linux` and `macos` until those bundled solver assets exist

## References

- `README.md`
- `.github/workflows/ci.yml`
- `docs/design/phase-25.0.10-cross-platform-proof-parity-gate.md`
- `docs/design/phase-25.1.13-ci-replay-gates-release-target-platforms.md`
- `docs/design/phase-25.6.0-binary-ga-and-provenance-policy-lock.md`
- `docs/design/phase-25.6.13-binary-publication-policy-lock.md`
- `docs/release/milestone_3-binary-operations.md`
- `docs/release/milestone_3-release-train-checklist.md`
- `docs/todo/milestone_4_roadmap.md`
