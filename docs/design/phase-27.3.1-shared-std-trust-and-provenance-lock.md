# Phase 27.3.1 - Shared Std Trust, Signing, and Provenance Lock

Date: 2026-06-02
Status: Locked
Owner: std-arch-owner

## Purpose

Define the minimum trust/signing/provenance policy that must exist before ClearLang can enable any non-embedded std distribution mode.

This lock does **not** enable shared/precompiled std delivery by itself.
It defines the fail-closed policy that such a mode would have to satisfy first.

## Scope

This lock applies to any std package that is:

1. versioned independently from the compiler, and
2. loaded or linked as an external artifact instead of being embedded into the final release artifact.

## Locked Policy

### 1. Signed Artifact Requirement

Every separately versioned std package artifact must be signed.

Required signed identity fields are:

1. package id
2. package version
3. verified std ABI range
4. artifact digest
5. artifact format
6. signing key id
7. signing timestamp

Unsigned shared/precompiled std artifacts are never acceptable in production mode.

### 2. Trusted Key Resolution

Shared/precompiled std packages must resolve signer trust through an explicit trusted key set.

That trust set must be:

1. versioned
2. auditable
3. fail-closed on unknown key ids
4. fail-closed on revoked key ids
5. fail-closed on invalid signing windows

Trust must never be inferred from package names, package versions, or repository location alone.

### 3. Provenance Binding

The release artifact must carry provenance that binds:

1. the selected std package identities
2. the exact artifact digests
3. the accepted verified std ABI contract
4. the signer identities used to authorize those artifacts

Any bundle/manifest verification path must be able to prove which shared std artifact set was accepted at release time.

### 4. ABI Compatibility Is Checked Before Loading

Shared/precompiled std packages must pass verified std ABI compatibility checks before any runtime loading or link activation.

Required fail-closed checks are:

1. package declares an accepted verified std ABI range
2. ABI major matches exactly
3. ABI minor is within compiler-supported range
4. no dependency on removed compatibility shims
5. no unresolved moved-symbol dependency outside the locked shim window

### 5. Runtime Loader Fail-Closed Contract

If non-embedded std is ever enabled, the runtime loader contract must fail closed using the existing deterministic runtime package error family:

1. `R012` for missing/unavailable runtime std artifact
2. `R013` for digest mismatch
3. `R014` for signature/trust verification failure
4. `R015` for ABI/link binding mismatch
5. `R016` for capability/profile mismatch
6. `R017` for determinism replay mismatch

No warning-only downgrade is permitted for production release workflows.

### 6. Release Evidence Parity

Shared/precompiled std delivery is only acceptable if release evidence remains on par with embedded linking.

That means:

1. release-precheck must validate the shared std trust inputs
2. release manifests must record shared std package identities and digests
3. verify/verify-bundle flows must validate shared std provenance without manual ad hoc flags
4. reproducibility and rollback evidence must remain deterministic

### 7. Activation Guard

Even with this policy locked, non-embedded std remains disabled until an explicit follow-up activation phase says otherwise.

This lock authorizes only the policy boundary, not the product switch.

## Immediate Decision

With this lock in place, the project now has the required policy answer for shared/precompiled std trust.

The remaining question for any future activation phase is implementation readiness, not policy ambiguity.

## References

- `docs/design/phase-26.0.0-std-embedded-first-policy-lock.md`
- `docs/design/phase-26.0.1-std-scope-and-governance-lock.md`
- `docs/design/phase-27.0-verified-std-abi-decoupling-lock.md`
- `docs/design/phase-27.3.0-post-split-linking-reevaluation.md`
