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
- `docs/design/phase-18.4-compiled-package-imports.md`
- `docs/runtime/host-imports.md`
- `docs/runtime/chain-packages.md`
- `docs/proofs/crypto-limitations.md`
