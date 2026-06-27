# Phase 29.14.0 - Final Additive Product-Surface Selection

Date: 2026-06-27
Status: Locked
Owner: roadmap-owner

## Purpose

Re-rank the remaining additive Milestone 3 product tracks after `29.12` and `29.13`, then select
the next bounded surface decision from the only remaining authorized pool:

1. additional shared-package surface expansion
2. richer chain-helper surface decisions
3. delivery-mode simplification

## Current Starting Point

After `29.13`, Milestone 3 now has:

1. a locked breadth-complete shared-package production contract for the current eight-package set
2. GA release parity across `windows`, `linux`, and `macos`
3. supported Homebrew and winget publication metadata layered over canonical GitHub release
   artifacts
4. `embedded` as the default production mode
5. `shared` as a supported explicit opt-in production mode

That means the biggest remaining Milestone 3 questions are no longer platform or installer gaps.

The remaining open work is now purely additive product-surface prioritization.

## Re-Ranked Remaining Tracks

### 1. Delivery-Mode Simplification

This is now the highest-priority remaining additive decision.

Why it ranks first:

- simple for users: after platform and installer hardening, the largest remaining user-facing
  complexity is understanding when to choose `embedded` vs `shared`
- AI-friendly: one explicit delivery-mode contract is easier for tools to generate around than more
  package variants or broader helper APIs
- provably correct: this can be decided without broadening the supported package set or chain API
  surface
- crypto-focused: it keeps attention on deterministic activation, release evidence, and runtime
  trust boundaries

Why it is still bounded:

- this ranking does not pre-authorize changing the default mode
- it authorizes a decision lock about whether simplification should happen at all, and under what
  fail-closed conditions

### 2. Additional Shared-Package Surface Expansion

This now ranks second.

Why it remains behind delivery-mode simplification:

1. `29.8.1` already closed the planned breadth-complete shared-package set
2. no concrete new package candidate is currently locked as common enough to justify expansion
3. package-set growth increases long-term maintenance and documentation surface immediately

This stays valid future work, but not the next Milestone 3 decision.

### 3. Richer Chain-Helper Surface Decisions

This remains the lowest-priority remaining additive track.

Why it stays last:

1. it expands chain-facing API surface the most
2. it is the easiest track to weaken explicit correctness boundaries accidentally
3. it adds convenience before resolving the more fundamental delivery-mode product story

## Selection Decision

The next bounded Milestone 3 surface decision is:

- delivery-mode simplification

This creates the next planning gate:

- `29.15` Delivery-mode simplification decision

## Why This Selection Is Correct

### 1. It Best Matches The README Principles Now

After `29.12` and `29.13`, the product already made the two most important release-hardening
improvements:

1. the GA support matrix is explicit and release-gated
2. installer/publication-channel friction is reduced without weakening trust

The next highest-value question is therefore not “what more can be added,” but “can the delivery
story be made simpler without sacrificing the locked fail-closed contract?”

### 2. It Avoids Reopening Breadth Closure Prematurely

Selecting delivery-mode simplification before new package expansion respects the closure locked in
`29.8.1`.

That avoids treating the newly completed eight-package shared surface as if it were still
unfinished backlog.

### 3. It Keeps Chain-Helper Growth Appropriately Conservative

Broader chain-helper surfaces remain the easiest place to accidentally hide runtime assumptions or
blur correctness boundaries.

They should stay behind a clearer final answer on delivery-mode semantics.

## What Remains Deferred

This selection explicitly defers:

1. expanding the supported shared package-id set beyond the current locked eight-package boundary
2. adding broader chain-specific helpers, serializers, event surfaces, or runtime IO helpers
3. changing delivery-mode defaults without a dedicated decision lock

## Success Target For `29.15`

`29.15` succeeds only when:

1. the product explicitly decides whether Milestone 3 should keep `embedded` as the default
   supported mode or authorize a later simplification path
2. migration, activation, and rollback semantics remain explicit and fail closed
3. the decision does not implicitly broaden shared-package membership or chain-helper scope

## Next Task

The next task after this selection lock is `29.15.0`:

- publish the bounded delivery-mode simplification decision lock, including evaluation criteria,
  migration constraints, fail-closed activation rules, and explicit non-goals

## References

- `README.md`
- `docs/TODO.md`
- `docs/design/phase-28.9.2-shared-std-product-contract-lock.md`
- `docs/design/phase-29.8.1-milestone-3-closure-lock.md`
- `docs/design/phase-29.10.0-remaining-production-candidate-inventory-lock.md`
- `docs/design/phase-29.13.0-installer-publication-channel-policy-lock.md`
- `docs/todo/milestone_3_roadmap.md`
