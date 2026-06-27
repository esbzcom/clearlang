# Phase 29.0.0 - Post-M3 Expansion Planning Lock

Date: 2026-06-27
Status: Locked
Owner: roadmap-owner

## Purpose

Milestones 1-3 close the currently tracked foundation, go-live, proof/release, and shared-std
distribution roadmap.

Phase 29 begins only after the post-production backlog is re-established explicitly.

The purpose of this lock is to prevent unfocused post-milestone work from degrading the current
product contract.

## Current Project Status

The project now has:

1. a complete tracked roadmap through Milestone 3
2. `embedded` as a supported production default
3. `shared` as a supported explicit opt-in production mode for the current bundled package set
4. documented and gated proof, release, provenance, and distribution workflows

That means the next planning cycle must optimize for disciplined prioritization rather than broad
feature accumulation.

## Prioritization Rules

Any Phase 29 execution slice must satisfy the repository design principles:

### 1. Simple For Users

New work should reduce workflow complexity, clarify product contracts, or remove user-visible
surprises. Feature additions that primarily increase surface area without simplifying adoption are
lower priority.

### 2. AI-Friendly

New work should prefer canonical schemas, deterministic diagnostics, and explicit state transitions
over heuristic or hidden behavior. If AI tools cannot safely generate, repair, or reason about the
workflow, the slice is not ready.

### 3. Provably Correct

Phase 29 must not weaken the current proof/release contract. New slices should either preserve
the existing formal guarantees or make those guarantees easier to validate and operate.

### 4. Crypto-Focused

Priority should go to deterministic, audit-grade workflows that matter for smart contracts,
package trust, release integrity, runtime safety, or chain-facing correctness boundaries.

## Non-Goals

This lock does not authorize:

1. reopening completed Milestone 3 rollout decisions without a new design lock
2. adding broad new language surface area without a scoped success target
3. weakening fail-closed release, trust, or proof behavior to accelerate feature velocity
4. feature selection driven only by convenience rather than product contract impact

## Success Target

Phase 29 planning is successful only when:

1. the first post-Milestone-3 execution slice is explicitly selected
2. that slice has a bounded scope and acceptance gate
3. deferred work is named explicitly
4. the selected slice is justified against the design principles above

## Next Task

The next task after this planning lock is `29.1.0`:

- select the first post-Milestone-3 implementation slice and lock its execution contract

