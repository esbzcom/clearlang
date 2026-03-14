# Phase 20.0 - Std Packaging and Runtime Linking Lock

## Status
Design lock for post-Phase-19 execution.
Roadmap execution is split into Phases 20-24 in `docs/TODO.md` to keep milestone slices small and testable.

## Goal
Define the long-term, production-grade std architecture for crypto environments:
- small per-app Wasm artifacts,
- reusable audited std packages,
- deterministic fail-closed trust checks.

## Design Principles Check
- Simple for users: default path remains one-command build/run; package complexity is behind host policy and lockfiles.
- AI-friendly: deterministic metadata, stable diagnostics, and explicit trust boundaries.
- Provably correct: proof artifacts keep assumed/proved boundaries explicit and verifiable.
- Crypto-focused: package loading is hash/signature pinned and fails closed.

## Final Architecture (Locked)
1. `std::core` (precompiled package)
   - Pure deterministic helpers (e.g., bytes/str/collections/numeric helpers) are shipped as versioned compiled package artifacts.
   - User modules import these symbols through package metadata; app Wasm no longer duplicates all reusable std-core code.

2. `std::host` (host-backed capabilities)
   - `std::crypto`, `std::env`, and `std::wasi` remain host capabilities via imports.
   - Hosts must provide deterministic behavior profiles and explicit trap mapping.

3. `std::<chain>` (chain packages)
   - Chain-specific types/helpers stay outside core and are versioned independently.
   - Chain packages may wrap host capabilities through `io` surfaces only.

## Packaging and Trust Model
1. Compile-time package resolution:
   - Resolve imports from package metadata with exact versions and artifact digests.
   - Emit deterministic external import bindings for used symbols only.

2. Runtime package linking:
   - Runtime linker loads artifacts from a trusted local store/registry index configured by policy.
   - Every artifact must pass digest/signature verification before use.
   - Mismatch, missing signatures, or policy violations fail closed.

3. No ambient dynamic trust:
   - No implicit network fetch during execution in strict production profiles.
   - Runtime loading source is host-controlled and policy-pinned.

## Execution Profiles
1. Static/contract profile:
   - Prefer fully pinned artifacts and deterministic host bindings for consensus contexts.

2. Shared/app profile:
   - Allow reusable precompiled std packages for artifact size and startup wins.
   - Still requires digest/signature checks.

## Diagnostics (Planned)
- Add deterministic package/linker diagnostics for:
  - missing package artifact,
  - digest/signature mismatch,
  - untrusted signer/policy violation,
  - unresolved runtime import.

## Strict-Mode Acceptance Gates (20.1.2)
1. Source-of-truth gate
   - Strict mode resolves packages only from lockfile + trusted local store.
   - No implicit network fetch during build/run.

2. Artifact identity gate
   - Required exact `(name, version, digest)` match against lockfile entries.
   - Any digest mismatch fails closed.

3. Trust gate
   - Package signatures must verify against configured trust anchors.
   - Untrusted, revoked, or expired signers fail closed.

4. Metadata/schema gate
   - Package metadata schema version must be accepted by policy/toolchain.
   - Unsupported schema versions fail with deterministic diagnostics.

5. ABI/link gate
   - Linked imports must match expected symbol signature/effect/capability profile exactly.
   - ABI mismatch fails deterministically (no best-effort fallback).

6. Runtime capability gate
   - Host profile must explicitly provide required capabilities for linked imports.
   - Missing capability fails closed.

7. Determinism gate
   - Identical inputs (source, lockfile, package store, trust policy) must produce identical resolved direct-dependency import map and diagnostics ordering in 20.1 scope.

8. CI acceptance gate
   - CI must include positive and tamper negative suites that assert stable diagnostic codes for all gates above.

## 20.1 Implementation Blueprint
1. Scope boundary
   - Implement strict-gate preflight only (policy + diagnostics + deterministic ordering).
   - Keep transitive resolver/semver/runtime auto-loading in later phases (22/23).

2. Preflight input contract
   - Minimal strict lockfile v0 entries (name/version/digest) for direct dependencies,
   - package metadata/schema v0 and ABI contract v0 (direct dependencies),
   - package signature envelope,
   - trust-policy v0/trust anchors (bootstrap scope for 20.1),
   - host-profile v0 capabilities (bootstrap scope for 20.1).

3. Evaluator behavior
   - A pure deterministic evaluator computes gate violations from the preflight input.
   - Violations are emitted in stable order: gate id -> package id -> symbol id.
   - Strict mode fails closed when any gate is violated, while emitting the complete ordered violation list for that run.
   - Evaluator emits a canonical direct-dependency import-map artifact with deterministic serialization/hash.

4. Build integration
   - Invoke preflight evaluator inside strict-mode build flow before final link/output emission.
   - Do not fallback to permissive behavior if preflight fails.

## Proposed Diagnostic Matrix (20.1.4.1)
Codes use the existing 4-character diagnostics convention (`[P|T|C|V|R][0-9]{3}`), so package/linker strict gates reserve the `C101`-`C107` range.
These codes are design-locked for 20.1 and must be promoted to the canonical diagnostics registry (`docs/diagnostics.md`) during implementation.

| Gate | Proposed code | Failure trigger |
|---|---|---|
| 20.1.2.1 Source-of-truth | `C101` | Strict mode attempted non-lockfile/non-trusted-store source (including implicit network fetch). |
| 20.1.2.2 Artifact identity | `C102` | `(name, version, digest)` mismatch vs lockfile pin. |
| 20.1.2.3 Trust/signature | `C103` | Strict trust gate failure (missing/invalid package signature envelope, untrusted/revoked signer, signer validity-window mismatch at signature timestamp, or signature verification failure). |
| 20.1.2.4 Metadata/schema | `C104` | Unsupported package metadata schema version/policy mismatch. |
| 20.1.2.5 ABI/link | `C105` | Metadata <-> ABI identity mismatch (`abi_id` / package / version), unresolved ABI import symbol, or resolved symbol signature/effect mismatch versus strict ABI contract. |
| 20.1.2.6 Runtime capability | `C106` | Required host capability missing in selected host-profile v0. |
| 20.1.2.7 Determinism | `C107` | Repeated evaluation with identical inputs produced different direct-dependency import map/diagnostics ordering. |

## Canonical Fixture Matrix (20.1.4.2)
| Fixture class | Expected result |
|---|---|
| `strict_ok` | Passes all gates in strict mode. |
| `strict_untrusted_source` | Fails with `C101`. |
| `strict_digest_mismatch` | Fails with `C102`. |
| `strict_bad_signature_or_signer` | Fails with `C103`. |
| `strict_schema_mismatch` | Fails with `C104`. |
| `strict_abi_mismatch` | Fails with `C105`. |
| `strict_missing_capability` | Fails with `C106`. |
| `strict_determinism_replay` | Two identical runs produce identical direct-dependency import map and sorted diagnostics; mismatch fails with `C107`. |

## Non-Goals (20.0)
- No semver solver introduction in this lock.
- No implicit internet package registry behavior.
- No weakening of strict/profile assurance guarantees from Phase 19.

## Milestone 2 Production Target
Milestone 2 (final production target) must explicitly close the current package/runtime gaps:
1. Add transitive dependency resolution for compiled packages.
2. Add a deterministic semver solver with lockfile pinning.
3. Add automatic runtime package loading/linking under fail-closed trust policy.

## Exit Criteria for 20.0
1. Design lock is published and referenced from roadmap docs.
2. `docs/TODO.md` contains executable Phases 20-24 slices with acceptance gates.
3. First execution slice starts with a runnable namespaced baseline sample.

## References
- `docs/TODO.md`
- `docs/rollout/DEVPLAN.md`
- `docs/design/phase-20.1.0-strict-lockfile-v0.md`
- `docs/design/phase-20.1.0-trust-policy-v0.md`
- `docs/design/phase-20.1.0-host-profile-v0.md`
- `docs/design/phase-20.1.0-package-metadata-abi-v0.md`
- `docs/design/phase-18.4-compiled-package-imports.md`
- `docs/runtime/host-imports.md`
- `docs/runtime/chain-packages.md`
- `docs/proofs/crypto-limitations.md`
