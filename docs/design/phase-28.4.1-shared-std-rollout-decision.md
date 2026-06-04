# Phase 28.4.1 - Shared Std Rollout Decision

Date: 2026-06-04
Status: Locked
Owner: std-arch-owner

## Decision

Shared std distribution remains `experimental` and `disabled by default`.

The project does **not** switch the default delivery mode away from `embedded` at the end of Milestone 3.

## Default Behavior

The default behavior after Milestone 3 is:

1. `embedded` std remains the default compiler/runtime/release mode
2. `shared` std requires explicit selection through the Phase 28 manifest/lock/runtime inputs
3. production release flows must continue to fail closed when shared std evidence is missing, tampered, unsigned, or ABI-incompatible
4. no silent fallback from `shared` to `embedded` is allowed

## Rationale

This is the correct rollout choice for three reasons.

### 1. Safety Is Proven, Adoption Is Not

Phase 28 now has:

1. explicit shared std artifact/manifest contracts
2. lockfile/runtime identity and ABI verification
3. release manifest, provenance, and verify-bundle parity
4. `xtask release-precheck` and CI wiring for the full shared std evidence chain

That is enough to make the mode auditable.

It is not, by itself, a reason to change the product default.

### 2. Embedded Still Has the Better Operational Profile

For the current milestone goals, embedded std still provides:

1. simpler deployment
2. smaller trust surface during rollout
3. fewer packaging and activation branches for end users
4. lower support risk while the new distribution path accumulates real usage

### 3. The Project Wanted Optional Separation, Not Forced Separation

The Milestone 3 objective was to make std separable and governable without coupling the language forever to one delivery model.

That objective is satisfied by:

1. preserving `embedded` as the safe default
2. enabling `shared` as an explicitly governed experimental path
3. keeping activation reversible until the ecosystem and tooling prove out

## Supported Rollout Shape

For now, the allowed rollout shape is:

1. internal validation
2. targeted profile experiments
3. explicit opt-in only

The following are not authorized by this decision:

1. default-on shared std for all users
2. warning-only downgrade behavior
3. undocumented profile-specific activation
4. implicit migration of existing embedded projects

## Exit Criteria For A Future Default-On Decision

The project may revisit this decision only when all of the following are true:

1. shared std package generation/resolution is routine in normal workflows
2. release and verify-bundle evidence paths are stable across real projects
3. CI coverage includes representative shared std profile matrices
4. no unresolved fail-closed diagnostics or provenance gaps remain
5. operational support cost is understood

## Milestone 3 Outcome

Milestone 3 closes with:

1. a usable language/toolchain
2. a production-safe embedded default
3. an auditable experimental shared std path

That is the right stopping point.
