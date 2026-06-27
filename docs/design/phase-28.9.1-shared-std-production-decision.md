# Phase 28.9.1 - Shared Std Production Decision

Date: 2026-06-27
Status: Locked
Owner: std-arch-owner

## Decision

`shared` std becomes a **supported production option**.

`embedded` remains a **supported production option** and stays the **default** delivery mode.

This is a promotion of support status, not a default flip.

## Supported Product Contract

After this decision:

1. `embedded` remains the default compiler/runtime/release mode
2. `shared` is supported only through the explicit schema-v2 manifest/lock/runtime flow
3. `shared` remains fail-closed for missing or mismatched identity, trust, ABI, digest, signature,
   provenance, or replay evidence
4. no silent fallback from `shared` to `embedded` is allowed
5. current supported bundled shared-std package ids are:
   - `std::text`
   - `std::int`
   - `std::sequence`
   - `std::codec`

## Why This Promotion Is Correct

### 1. The Supported Workflow Exists End To End

The product now has one normal supported workflow for shared std:

1. explicit shared-std manifest intent
2. normal `clg pkg lock` generation/update
3. normal `clg release`
4. normal `clg verify-bundle`
5. deterministic runtime loading

That workflow is no longer dependent on side-channel artifacts, hand-edited lockfiles, or
implementation-only operator knowledge.

### 2. The Safety Contract Is Enforced, Not Implied

The shared-std path now has:

1. canonical schema-v2 lock modeling across command/runtime/release paths
2. deterministic diagnostics for activation, trust, provenance, and ABI failures
3. command-level gates for happy-path and tamper-path behavior
4. documented publishing, rotation, rollback, and incident response playbooks

That is sufficient for supported production use as an explicit opt-in mode.

### 3. Preserving `embedded` As Default Still Fits The Design Principles

Keeping `embedded` as default preserves:

1. simpler deployment for the common case
2. familiar behavior for existing users
3. lower operational overhead where shared package governance is unnecessary

Promoting `shared` to supported production satisfies the product goal of offering deterministic,
auditable std distribution without forcing one delivery model on every project.

## What This Decision Does Not Authorize

This decision does not authorize:

1. changing the default from `embedded` to `shared`
2. implicit migration of embedded projects
3. warning-only downgrade behavior
4. undocumented activation shortcuts
5. unsupported shared package ids outside the current bundled package contract

## References

- `docs/evidence/phase-28.8-shared-std-support-readiness.md`
- `docs/evidence/phase-28.9.0-shared-std-rollout-reassessment.md`
- `docs/design/phase-28.5-shared-std-general-production-plan.md`
