# Phase 27.3.0 - Post-Split Linking Re-evaluation

Date: 2026-06-02
Status: Complete
Owner: std-arch-owner

## Purpose

Re-evaluate whether ClearLang should reopen precompiled/shared std delivery now that:

1. the verified std ABI has been extracted,
2. Wave 1 external std packageization is complete, and
3. release-precheck already gates the std conformance boundary.

## Inputs Reviewed

The re-evaluation is based on the now-complete earlier Phase 27 gates:

1. `27.0` architecture lock
2. `27.1` verified std ABI extraction
3. `27.2` Wave 1 external std packageization

And on the active fail-closed gate set:

1. `cargo run -p xtask -- std-arch-conformance-check`
2. `cargo run -p xtask -- release-precheck`

`release-precheck` already includes the std-arch conformance gate, so any future shared/precompiled std activation would not start from an ungated state.

## Re-evaluation Result

Shared/precompiled std delivery remains **deferred**.

This re-evaluation does **not** authorize any non-embedded distribution mode yet.

## Why It Stays Deferred

The compiler/package ownership boundary is now stable enough to evaluate distribution, but the distribution trust model is not complete enough to enable it.

The remaining blockers are:

1. No locked trust/signing/provenance policy yet exists for separately versioned std packages.
2. No locked runtime policy yet exists for verifying shared std package identity, signer trust, and rollback behavior under production release workflows.
3. The deferred dynamic/shared std blueprint from Phase 26 still requires explicit activation gates for:
   - shared std artifact trust/signature policy
   - ABI compatibility/version negotiation
   - deterministic runtime loader diagnostics and replay
   - rollback and incident-response runbooks
   - provenance parity with embedded release artifacts

Those are architecture-level release blockers, not implementation nits. Enabling shared/precompiled std before they are locked would widen the runtime trust surface without an equivalent fail-closed release contract.

## Locked Decision

1. Embedded linking remains the only production-approved std delivery mode.
2. The verified std ABI and external std package split may continue to evolve under embedded linking.
3. Any future activation of shared/precompiled std must first complete `27.3.1`.
4. No experimental activation flag is authorized by this re-evaluation.

## Exit Condition For This Re-evaluation

`27.3.0` is complete once the project records an explicit answer to:

"Now that the ABI/package split is stable, should shared/precompiled std be reopened?"

The answer from this phase is:

"No. Re-evaluation is complete, but activation remains deferred until the trust/signing/provenance policy is locked."

## References

- `docs/design/phase-26.0.0-std-embedded-first-policy-lock.md`
- `docs/design/phase-26.0.1-std-scope-and-governance-lock.md`
- `docs/design/phase-27.0-verified-std-abi-decoupling-lock.md`
