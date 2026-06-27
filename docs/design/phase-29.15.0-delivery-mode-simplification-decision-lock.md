# Phase 29.15.0 - Delivery-Mode Simplification Decision Lock

Date: 2026-06-27
Status: Locked
Owner: roadmap-owner

## Purpose

Close the remaining Milestone 3 delivery-mode simplification question with one explicit product
decision.

This lock evaluates whether Milestone 3 should simplify the current two-mode production story by
changing defaults or activation behavior after the `29.12` platform hardening and `29.13`
installer/publication-channel work.

## Decision

Milestone 3 does **not** change the delivery-mode default.

The final supported Milestone 3 delivery contract remains:

1. `embedded` is the default supported production mode
2. `shared` is a supported explicit opt-in production mode
3. no implicit migration or silent downgrade between modes is allowed

This closes the delivery-mode simplification track for Milestone 3.

## Evaluation Criteria

The simplification question was evaluated against the README principles using the following exact
criteria.

### 1. User Simplicity

A default-mode change is only justified if it makes the common production path simpler overall, not
just newer or more uniform on paper.

That bar is not met yet because `shared` still carries extra operator-facing concepts:

- explicit package publication/registry handling
- runtime artifact availability requirements
- shared-package trust/provenance inputs

For Milestone 3, the simplest general-purpose default remains the self-contained `embedded` path.

### 2. AI-Friendliness

Any simplification must reduce ambiguity for tools instead of moving it.

The current contract is already explicit and machine-friendly:

1. `embedded` is the default
2. `shared` requires schema v2 plus explicit manifest intent
3. unsupported mixed or partial workflows fail closed

Changing the default now would create migration ambiguity without enough offsetting reduction in
surface complexity.

### 3. Provable And Operational Correctness

A default flip is only acceptable if it preserves the fail-closed release/runtime evidence chain
with no new hidden assumptions for ordinary users.

That bar is intentionally kept high because `shared` adds more moving parts than `embedded`, even
though it is now supported.

Milestone 3 should keep the mode with the smaller operational surface as the default.

### 4. Crypto-Focused Release Discipline

For a smart-contract toolchain, simplification must not blur trust boundaries.

`embedded` remains the more conservative default because it avoids making external shared artifact
selection part of the common-path deployment story.

## Migration Constraints

Milestone 3 keeps the following migration constraints unchanged:

1. embedded projects are not required to migrate
2. shared activation remains explicit in `clg.project.json`
3. shared production use still requires the locked schema-v2 workflow
4. rollback from `shared` to `embedded` must stay explicit and documentation-backed
5. no compatibility pressure may be created by treating `embedded` as legacy or second-class

## Fail-Closed Activation Rules

The final Milestone 3 activation rules remain:

1. `shared` activates only when the project explicitly requests it
2. `shared` requires non-empty locked shared-package evidence under the current supported package
   set
3. missing/mismatched trust, ABI, digest, provenance, replay, or runtime-link evidence fails
   closed
4. `shared` never falls back silently to `embedded`
5. `embedded` remains valid without shared activation or shared evidence

## Explicit Non-Goals

This decision does not authorize:

1. defaulting all projects from `embedded` to `shared`
2. adding a heuristic or auto-selected delivery mode
3. introducing implicit fallback between `shared` and `embedded`
4. broadening the supported shared package-id set
5. coupling delivery-mode decisions to richer chain-helper expansion

## Final Milestone 3 Product Position

Milestone 3 now closes with one final delivery-mode story:

- `embedded` is the simple, default, self-contained production path
- `shared` is the supported, explicit, evidence-bearing opt-in production path for the locked
  eight-package set

That is the final supported delivery contract unless a future roadmap starts a new bounded planning
lock.

## Next Task

No further Milestone 3 additive-surface task is authorized by default after this decision.

Any later change to delivery-mode defaults, shared-package breadth, or chain-helper breadth must
start from a new explicit planning lock.

## References

- `README.md`
- `docs/TODO.md`
- `docs/design/phase-28.9.2-shared-std-product-contract-lock.md`
- `docs/design/phase-29.8.1-milestone-3-closure-lock.md`
- `docs/design/phase-29.14.0-final-additive-product-surface-selection.md`
- `docs/release/shared-std-user-guide.md`
- `docs/todo/milestone_3_roadmap.md`
