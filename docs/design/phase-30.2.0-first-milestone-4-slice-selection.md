# Phase 30.2.0 - First Milestone 4 Slice Selection

Date: 2026-06-27
Status: Locked
Owner: roadmap-owner

## Decision

The first Milestone 4 execution slice is:

- multi-platform GA hardening

This creates the first bounded execution gate for Milestone 4:

- `30.3` Multi-platform GA hardening

## Why This Slice Comes First

### 1. It Improves The Supported Product Contract Without Expanding Surface Area

Milestone 3 already closed the currently planned language, release, and shared-std product
surface.

The next highest-value step is to make the shipped support matrix more explicit and more complete,
not to add more API surface immediately.

### 2. It Best Matches The README Principles

- simple for users: a clearer GA platform matrix reduces install and support ambiguity
- AI-friendly: a locked support matrix and parity gate are easier to automate than broader feature
  additions
- provably correct: extending deterministic proof/release expectations across supported platforms is
  a direct correctness improvement
- crypto-focused: hardened release and provenance guarantees on shipped binaries matter more than
  additive library convenience

### 3. It Keeps Milestone 4 Conservative

Choosing platform hardening first avoids two riskier early moves:

1. broadening the supported shared package set before the current set has longer operational soak
2. expanding chain-helper surfaces before the constructor-only contract has been proven routine

## What Is Deferred

This selection intentionally defers:

1. installer/publication channel expansion
2. additional shared-package promotion work
3. richer chain-helper or chain-runtime helper surfaces
4. any default-mode change from `embedded` to `shared`

Those stay valid candidate tracks, but not the first Milestone 4 slice.

## Success Target For `30.3`

`30.3` succeeds only when:

1. the GA platform matrix is re-locked explicitly
2. the proof/release parity contract matches that matrix
3. release-train blocking conditions match the locked support policy
4. operator documentation is updated to reflect the exact supported platform set

## Next Task

The next task after this slice-selection lock is `30.3.0`:

- publish the support-matrix and GA-promotion lock for the first Milestone 4 execution slice

## References

- `README.md`
- `docs/design/phase-25.1.13-ci-replay-gates-release-target-platforms.md`
- `docs/design/phase-25.6.0-binary-ga-and-provenance-policy-lock.md`
- `docs/design/phase-25.6.13-binary-publication-policy-lock.md`
- `docs/design/phase-30.1.0-post-release-candidate-inventory-lock.md`
- `docs/todo/milestone_4_roadmap.md`
