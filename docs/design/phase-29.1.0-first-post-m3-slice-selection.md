# Phase 29.1.0 - First Post-Milestone-3 Slice Selection

Date: 2026-06-27
Status: Locked
Owner: roadmap-owner

## Decision

The first post-Milestone-3 implementation slice is:

- promote `std::sequence` to the supported shared-std package set

This creates the first bounded execution gate after Milestone 3:

- `29.2` `std::sequence` shared-std promotion

## Why `std::sequence` Comes First

### 1. It Closes The Remaining Wave 1 Helper-Surface Gap

Phase 27 already identified Wave 1 helper-surface packages:

- `std::text`
- `std::int`
- `std::codec`
- `std::sequence`

The current supported shared package set includes the first three but not `std::sequence`.

That makes `std::sequence` the cleanest next step because it completes an already-planned wave
instead of starting a new architectural branch.

### 2. The Slice Is Small And Bounded

`std::sequence` currently covers only:

- modules: `std::array`, `std::slice`
- symbols:
  - `std::array::len`
  - `std::slice::get`
  - `std::slice::len`
  - `std::slice::subslice`

This is materially smaller than:

- `std::contract` domain-surface promotion
- chain-adapter package promotion
- any broader collection/list/set/map externalization work

That makes it a good fit for the first post-M3 slice because it is easy to validate end to end.

### 3. It Fits The Design Principles

It aligns with the README principles directly:

- simple for users: small helper surface, explicit opt-in, no new activation model
- AI-friendly: deterministic package id, small symbol set, stable migration target
- provably correct: no weakening of the current fail-closed release/proof contract
- crypto-focused: keeps package trust, provenance, and deterministic distribution machinery on the
  highest-value path instead of expanding into unrelated surface area

## What Is Deferred

This selection intentionally defers:

1. `std::contract` shared-package promotion
2. chain-adapter shared-package promotion (`std::eth`, `std::solana`, `std::cosmos`)
3. broader collection packageization beyond the bounded `std::array` / `std::slice` helper cut
4. any change to the default delivery mode

Those can be revisited only after `std::sequence` proves the next promotion path is routine.

## Success Target For The Slice

`29.2` succeeds only when:

1. `std::sequence` is part of the supported shared-package set
2. xtask publication supports `std::sequence`
3. manifest/lock/release/verify/runtime flows accept `std::sequence` under the same fail-closed
   rules as the existing shared packages
4. migration diagnostics name `std::sequence` deterministically for moved `std::array` /
   `std::slice` helpers

## References

- `docs/design/phase-27.0-verified-std-abi-decoupling-lock.md`
- `docs/design/phase-27.2-external-std-package-plan.v1.json`
- `docs/design/phase-29.0.0-milestone-4-planning-lock.md`

