# Phase 28.0 - Shared Std Distribution Design Lock

Date: 2026-06-02
Status: Locked for planning
Owner: std-arch-owner

## Purpose

Phase 27 finished the verified-ABI/package split and the policy gates for any future non-embedded std delivery mode.

Phase 28 is the activation phase for that work, but only under fail-closed distribution rules.

The purpose of this lock is to define the execution order and non-goals for implementing separately versioned std distribution without weakening release determinism.

## Phase 28 Goal

Enable a production-auditable non-embedded std delivery path that:

1. uses the verified std ABI as the compatibility boundary,
2. loads only signed and trusted std package artifacts,
3. preserves deterministic diagnostics and replay behavior, and
4. keeps embedded std as the default-safe fallback until the new path is fully proven.

## Execution Order

Phase 28 must proceed in this order:

1. lock artifact/package manifest shapes for separately versioned std packages
2. implement runtime/linker identity and ABI verification gates
3. implement deterministic loader diagnostics and replay evidence
4. wire release/verify-bundle provenance to shared std package identities
5. enable a gated non-embedded std mode only after CI/release-precheck coverage is green

## Non-Goals

Phase 28 does not authorize:

1. replacing embedded std as the default
2. warning-only downgrade paths for trust, ABI, or provenance failures
3. package-name-based trust inference
4. silent fallback from shared std to embedded std in production workflows

## Required Safety Invariants

Any Phase 28 implementation must preserve all of the following:

1. deterministic ABI acceptance/rejection
2. deterministic signer/trust resolution
3. deterministic runtime package set selection
4. deterministic release-manifest evidence for selected std artifacts
5. fail-closed runtime behavior on missing/tampered/incompatible std artifacts

## Milestone Breakdown

The first execution plan for Phase 28 is:

1. `28.0` distribution architecture + artifact contract lock
2. `28.1` signed std package manifest and lockfile integration
3. `28.2` runtime/link-time loader and ABI verification
4. `28.3` release evidence, verify-bundle, and provenance parity
5. `28.4` CI/release-precheck activation gate and rollout decision

## Entry Criteria

Phase 28 starts only because all of these are now true:

1. verified std ABI extraction is complete
2. Wave 1 external std packageization is complete
3. moved-symbol migration shims/diagnostics are complete
4. post-split shared std trust/signing/provenance policy is locked

## Exit Criteria

Phase 28 is complete only when:

1. non-embedded std delivery is fully gated and deterministic
2. release-precheck and verify-bundle validate the full shared std evidence chain
3. runtime/package diagnostics are stable and fail closed
4. the project explicitly decides whether the feature is enabled, experimental, or still deferred by default

## References

- `docs/design/phase-27.0-verified-std-abi-decoupling-lock.md`
- `docs/design/phase-27.3.0-post-split-linking-reevaluation.md`
- `docs/design/phase-27.3.1-shared-std-trust-and-provenance-lock.md`
