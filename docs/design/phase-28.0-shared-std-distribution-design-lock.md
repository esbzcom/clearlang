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

## 28.0.1 Canonical Shared Std Artifact and Manifest Shape

The canonical shared-std identity contract is locked as follows.

### Design Rule

Shared std package identity must be expressed through explicit artifact and manifest records.

The compiler, lockfile, runtime loader, release manifest, and verify flows must all refer to the same logical fields rather than inventing stage-specific variants.

### Logical Records

Phase 28 defines three logical records:

1. `shared_std_package_requirement`
   - user/resolver-facing requirement record
2. `shared_std_package_lock`
   - exact-pinned lock/runtime identity record
3. `shared_std_package_provenance`
   - release/verification evidence record

### 1. Requirement Record

The requirement record is the minimal declarative input used by manifests/resolution.

Required fields:

1. `package_id`
2. `version_requirement`
3. `verified_std_abi`
   - required range expression over `major.minor`
4. `delivery`
   - `embedded` or `shared`

Optional fields:

1. `registry`
2. `signer_policy`
3. `allow_compat_shims`
   - explicit boolean, default `false`

Locked rules:

1. `package_id` is the canonical package name such as `std::text`.
2. `version_requirement` uses normal semver requirement syntax.
3. `verified_std_abi` is mandatory for all shared std package requirements.
4. `delivery=shared` is invalid unless the package is classified as external-package-capable under the Phase 27 split.
5. Requirements never carry artifact paths or digests directly.

### 2. Lock Record

The lock record is the exact-pinned identity consumed by runtime/link activation.

Required fields:

1. `package_id`
2. `version`
3. `verified_std_abi`
   - exact accepted ABI range/claim recorded in the lock output
4. `artifact`
   - object with:
   - `format`
   - `path`
   - `digest`
   - `size_bytes`
5. `signature`
   - object with:
   - `key_id`
   - `algorithm`
   - `signed_at`
   - `signature`
6. `provenance`
   - object with:
   - `statement_digest`
   - `statement_format`
7. `symbols`
   - canonical exported std module/symbol summary for deterministic identity reporting
8. `dependencies`
   - exact locked shared-std dependency ids, if any

Locked rules:

1. `artifact.path` must be relative and non-traversing.
2. `artifact.digest` is mandatory and canonicalized as `sha256:<hex>`.
3. `artifact.format` is explicit and must not be inferred from filename.
4. Signature data is mandatory for any `delivery=shared` locked entry.
5. `verified_std_abi` in the lock record is exact evidence, not a loose requirement.
6. The lock record is the only source of truth for runtime/link-time artifact identity.

### 3. Provenance Record

The provenance record is the release/verification evidence projection of the shared std lock record.

Required fields:

1. `package_id`
2. `version`
3. `verified_std_abi`
4. `artifact_digest`
5. `signature_key_id`
6. `provenance_digest`

Locked rules:

1. Release manifests and verify-bundle evidence must carry these fields for every shared std artifact selected for a release.
2. Provenance projection must be derivable from the lock record without hidden runtime-only fields.
3. Verify flows must fail closed if any shared std artifact used at release time lacks a matching provenance record.

### Canonical JSON Example

The canonical locked shared std package entry is:

```json
{
  "package_id": "std::text",
  "version": "1.2.0",
  "verified_std_abi": {
    "major": 1,
    "minor_min": 3,
    "minor_max": 3
  },
  "artifact": {
    "format": "wasm",
    "path": "std-packages/std-text-1.2.0.wasm",
    "digest": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    "size_bytes": 24576
  },
  "signature": {
    "key_id": "std-publisher-ed25519-2026q2",
    "algorithm": "ed25519",
    "signed_at": "2026-06-02T00:00:00Z",
    "signature": "BASE64_SIGNATURE"
  },
  "provenance": {
    "statement_digest": "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
    "statement_format": "in-toto-v1"
  },
  "symbols": [
    "std::bytes",
    "std::str",
    "std::str_pattern"
  ],
  "dependencies": []
}
```

### Integration Boundary

This lock does not yet choose the exact file names or final embedding location inside:

1. `clg.project.json`
2. `clg.lock.json`
3. release manifest payloads

That integration work belongs to `28.1.x` and `28.3.x`.

What is locked here is the canonical field model those later artifacts must reuse.

### Failure Policy

Any future implementation must fail closed when:

1. a shared std package requirement omits `verified_std_abi`
2. a shared std lock entry omits signature or artifact digest
3. provenance cannot be matched back to a locked shared std artifact
4. artifact identity fields disagree across manifest, lock, loader, and release evidence

## 28.0.2 Embedded-vs-Shared Activation Semantics

The activation and fallback contract for std delivery is locked as follows.

### Delivery Modes

Phase 28 defines two canonical delivery modes:

1. `embedded`
2. `shared`

No third implicit mode such as `auto` or `best_effort` is allowed in the initial activation design.

### Default Behavior

The default delivery mode remains:

1. `embedded`

This preserves the Milestone 26 production safety baseline and keeps existing projects deterministic unless they explicitly opt into shared std distribution.

### Explicit Selection Rule

Shared std activation must be an explicit user/project choice.

Allowed selection surfaces in later implementation phases are:

1. project manifest default
2. lockfile-resolved delivery mode
3. explicit CLI override

Selection precedence is locked as:

1. explicit CLI override
2. resolved lockfile mode
3. project manifest default
4. compiler default (`embedded`)

### Fallback Policy

Fallback behavior is locked conservatively:

1. `embedded -> shared` fallback is forbidden
2. `shared -> embedded` silent fallback is forbidden
3. if `shared` is requested and cannot be satisfied, the command fails closed
4. if `embedded` is requested, shared std artifacts must not be consulted for activation

This means ClearLang never changes std delivery mode implicitly at build, run, release, or verify time.

### Mode-Satisfaction Rules

`embedded` mode is satisfied only when:

1. all required std surfaces are linked into the artifact
2. no shared std runtime artifact is required for execution
3. release evidence does not claim shared std package identity

`shared` mode is satisfied only when:

1. all required shared std package lock entries are present
2. ABI compatibility checks pass before activation
3. signature and provenance requirements pass before activation
4. runtime/link-time artifact identity exactly matches the lock/evidence contract

### Diagnostic Contract

Requested delivery mode failures must be deterministic and fail closed.

The diagnostic message must always name:

1. the requested delivery mode
2. the reason it could not be satisfied
3. the exact missing or conflicting artifact/policy input
4. the required corrective action

The implementation may map those failures to existing deterministic build/runtime code families, but it must not collapse them into generic ambiguous errors.

Locked failure categories are:

1. requested `shared` mode but no locked shared std artifact set exists
2. requested `shared` mode but artifact identity/trust/ABI/provenance validation fails
3. requested `embedded` mode but inputs simultaneously require shared-only std delivery
4. conflicting delivery-mode declarations across manifest, lockfile, and explicit CLI inputs

### Release Workflow Rule

For production release workflows:

1. `embedded` remains the default-safe mode
2. `shared` is allowed only when explicitly selected and fully evidenced
3. any unresolved delivery-mode mismatch fails closed before release artifact generation

### Verify Workflow Rule

Verification must respect the claimed delivery mode.

That means:

1. a release bundle claiming `embedded` mode must not require shared std artifact evidence
2. a release bundle claiming `shared` mode must fail verification unless shared std provenance is present and matches the locked identities

### Non-Goal Clarification

This activation lock does not yet decide whether `shared` mode will become:

1. experimental-only
2. opt-in production
3. future default

That decision belongs to `28.4.1`.

## References

- `docs/design/phase-25.4.1-project-manifest-v1.md`
- `docs/design/phase-25.4.2-lockfile-tool-generated-contract.md`
- `docs/design/phase-27.0-verified-std-abi-decoupling-lock.md`
- `docs/design/phase-27.3.0-post-split-linking-reevaluation.md`
- `docs/design/phase-27.3.1-shared-std-trust-and-provenance-lock.md`
