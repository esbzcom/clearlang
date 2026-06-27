# Phase 29.8.0 - Breadth-Complete Production Decision

Date: 2026-06-27
Status: Locked
Owner: std-packages-owner

## Purpose

Re-run the Milestone 3 shared-std production decision after the remaining Wave 2 package work is
complete.

This decision answers one question only:

- is the planned Wave 2 shared-package surface now complete enough to call Milestone 3
  production-complete in breadth as well as baseline safety?

## Decision

Yes.

Milestone 3 is now breadth-complete for the planned shared-std surface.

The supported production contract now covers:

- `embedded` as the default supported production mode
- `shared` as an explicit opt-in supported production mode
- the full planned bundled shared-package set for this milestone:
  - `std::text`
  - `std::int`
  - `std::sequence`
  - `std::codec`
  - `std::contract`
  - `std::eth`
  - `std::solana`
  - `std::cosmos`

This is a breadth-completion decision, not a default-mode change.

## Why This Decision Is Correct

### 1. The Planned Wave 2 Surface Is Fully Promoted

The bounded post-Phase-28 package plan for Milestone 3 is now implemented:

1. `std::contract` is supported in shared form
2. `std::eth` is supported in shared form
3. `std::solana` is supported in shared form
4. `std::cosmos` is supported in shared form

No planned Wave 2 package from the locked ordering remains partially promoted.

### 2. The 29.5.0 Closure Criteria Are Now Satisfied

The chain-adapter closure criteria locked in `29.5.0` are now met:

1. each package has a bounded module/symbol lock
2. each package is promoted under the same trust, ABI, digest, provenance, and replay rules as
   the earlier supported shared packages
3. each package has deterministic manifest, lock, release, verify-bundle, runtime, and migration
   coverage
4. user and operator documentation now names the exact supported chain-adapter set

That means the remaining work is no longer package promotion work. It is closure-contract work.

### 3. The Product Contract Still Matches The Design Principles

This breadth-complete state still preserves the README principles:

- simple for users: `embedded` remains the default and the shared package set is explicit
- AI-friendly: the supported package boundary is exact and the diagnostics stay fail-closed
- provably correct: the rollout remains bounded to constructor/type surfaces with deterministic
  trust and ABI checks
- crypto-focused: the supported chain packages stay narrow, deterministic, and audit-grade

## What This Decision Does Not Change

This decision does not authorize:

1. changing the default from `embedded` to `shared`
2. expanding beyond the currently locked shared package-id set
3. broadening chain packages beyond the bounded constructor/type surfaces already locked
4. relaxing fail-closed trust, provenance, ABI, digest, or replay behavior

Those remain out of scope for this decision.

## Next Task

The remaining Milestone 3 closure work is `29.8.1`:

- publish the final Milestone 3 closure lock naming the supported shared-package set,
  intentionally deferred non-goals, and steady-state maintenance expectations

## References

- `docs/design/phase-28.9.1-shared-std-production-decision.md`
- `docs/design/phase-28.9.2-shared-std-product-contract-lock.md`
- `docs/design/phase-29.5.0-chain-adapter-production-ordering-lock.md`
- `docs/design/phase-29.5.1-chain-adapter-rollout-shape-lock.md`
- `docs/todo/milestone_3_roadmap.md`
