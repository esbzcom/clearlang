# Phase 27.0 - Verified Std ABI Decoupling Lock

Date: 2026-05-31
Status: Locked for Phase 27 planning
Owner: std-arch-owner

## Purpose

Phase 26 intentionally shipped a compiler-governed, embedded-first standard library model so first production could be deterministic, fail-closed, and fully auditable. That model is now complete enough to support the next step: split the small set of compiler-trusted std surfaces from the much larger set of libraries that should evolve independently.

The Phase 27 goal is not "make std fully dynamic." The goal is narrower and stricter:

1. Keep a minimal compiler-known verified std ABI.
2. Move non-proof-critical and non-host-policy-critical std functionality into separately versioned packages.
3. Preserve deterministic release behavior and fail closed if package resolution, ABI compatibility, or trust guarantees drift.

## Locked Layering Model

Phase 27 adopts a three-layer standard library architecture:

1. Language kernel
   - Owned by the compiler.
   - Includes syntax, type/effect rules, proof rules, deterministic diagnostics, and lowering semantics.
   - Must not depend on package resolution for core correctness.

2. Verified std ABI
   - Small compiler-known surface that remains trusted by typer, lowering, codegen, release gates, and proof policy.
   - Exists to model runtime/host boundaries and other semantics that cannot become "just library code" without weakening soundness or determinism.
   - Is versioned, compatibility-checked, and fail-closed.

3. External std packages
   - Separately versioned package surfaces implemented outside the compiler-trusted ABI boundary.
   - May evolve on a different cadence from the language/compiler once they satisfy locked compatibility and trust rules.
   - Must not silently change compiler assumptions.

## What Must Remain Compiler-Known

A std symbol stays inside the verified std ABI if changing or externalizing it would risk any of the following:

- proof soundness
- effect classification correctness
- host-capability policy enforcement
- deterministic lowering/codegen contracts
- strict release gating or theorem-grade no-assumption rules

Examples that remain compiler-known until an equivalent locked ABI exists:

- host/runtime boundary surfaces
- effect-bearing capability wrappers
- proof-critical collection semantics that participate in release-enabled theorem gates
- panic/error/runtime contracts that the compiler or verifier assumes directly

## What Should Move Out

A std symbol should move to external package space when it is primarily reusable functionality whose behavior can be specified through stable public contracts without compiler special handling.

Candidate early externalization targets:

- convenience helpers over `std::bytes`, `std::str`, and `std::int`
- codec composition helpers above the canonical wire ABI
- contract-domain convenience helpers that do not define the trusted host boundary
- chain-target adapters and domain packages above the stable ABI

## Non-Goals

Phase 27 does not authorize the following by default:

- removing the verified std ABI entirely
- shared/dynamic std runtime linking for production by default
- weakening strict release checks to permit unresolved package or ABI drift
- moving host-capability policy into untrusted package code

Dynamic/shared linking remains a later activation path, not the first deliverable of Phase 27.

## Versioning and Compatibility Policy

The compatibility model is locked as follows:

1. The verified std ABI has its own version line and compatibility contract.
2. External std packages version independently from the compiler once they no longer require compiler-known symbol tables.
3. Compiler releases must declare which verified std ABI versions they accept.
4. Package resolution must fail closed when an external std package requires an unsupported ABI version.
5. Alias/deprecation handling must remain explicit, cataloged, and diagnosable.

The concrete Phase 27 versioning policy is:

1. The verified std ABI uses `major.minor` compatibility semantics.
2. A compiler release must declare an accepted ABI range as:
   - exact major match required
   - minimum supported minor
   - maximum supported minor
3. External std packages must declare the verified std ABI range they require.
4. Resolution fails closed when:
   - no ABI declaration is present
   - majors do not match
   - the package requires a minor newer than the compiler accepts
   - the package requires a minor older than the compiler still supports only through removed/deprecated compatibility shims
5. ABI-major changes are never silently tolerated and require an explicit compiler upgrade path.
6. External std package version numbers are independent from compiler versions and must not be interpreted as ABI compatibility by themselves.

The Phase 27 compatibility decision is therefore based on ABI declarations first, package versions second.

## Migration Rule

A symbol may move from compiler-governed std to external package space only when all of the following are true:

1. Its type/effect/lowering behavior is fully expressible through the verified std ABI contract.
2. No proof or strict-release rule depends on hidden compiler knowledge of that symbol.
3. Metadata, import resolution, conformance, and trust checks can validate it without hard-coded duplicate symbol logic.
4. Migration diagnostics and compatibility shims are documented and fail closed on unsupported usage.

## Migration Diagnostics and Deprecation Policy

When a symbol moves from compiler-governed std to external package space, migration behavior is locked as follows:

1. The old compiler-known symbol path must either:
   - resolve through an explicit cataloged compatibility shim, or
   - fail with a deterministic migration diagnostic
2. Diagnostics must name:
   - the moved symbol
   - the replacement package or symbol path
   - the ABI/package requirement needed to restore compatibility
3. Silent fallback from compiler-known semantics to package-provided semantics is forbidden.
4. Compatibility shims must be:
   - explicitly cataloged
   - version-bounded
   - removable on a documented schedule
5. Once a symbol is classified as external-package-only, compiler conformance tooling must fail closed on any new direct compiler dependency on that symbol.
6. Release-precheck must reject builds that rely on expired shims or unresolved moved-symbol dependencies.

Deprecation policy for moved symbols is:

1. Introduce deprecation with deterministic warnings and migration guidance.
2. Keep the shim only for the locked compatibility window.
3. Remove the shim once the cutover window closes and upgrade diagnostics are in place.

## Execution Order

Phase 27 must proceed in this order:

1. Lock the architecture and migration criteria.
2. Extract the verified std ABI as an explicit compiler-consumed contract.
3. Split catalog and conformance tooling between ABI surfaces and external package surfaces.
4. Externalize pure/helper std modules first.
5. Revisit shared/precompiled std linking only after the ABI/package split is stable and audited.

## Completion Criteria

Phase 27 is complete only when all of the following are true:

1. The compiler consumes a minimal verified std ABI contract instead of treating the full std catalog as compiler-known.
2. Non-ABI std packages can evolve independently without modifying compiler symbol tables.
3. Conformance gates fail closed on ABI drift, package drift, and unauthorized compiler/package coupling.
4. Release workflows remain deterministic under embedded linking while mixed ABI/package mode exists.
5. Migration and deprecation paths for moved symbols are documented and testable.

## Initial Classification Artifact

The initial symbol-by-symbol Phase 27 classification is recorded in:

- `docs/design/phase-27.0-std-symbol-classification.v1.json`
- `docs/design/phase-27.1-verified-std-abi.manifest.v1.json`

That artifact is generated from the locked Phase 26 catalog and is the source of truth for the first decoupling split:

- `verified_std_abi`: 80 symbols
- `external_std_package_candidate`: 97 symbols

This classification is intentionally conservative. It keeps host boundaries, proof-critical collections, core runtime contracts, and test-harness-assumed surfaces inside the verified ABI while marking pure/helper, codec, contract-domain, and chain-target adapters as early externalization candidates.

## Initial Externalization Plan Artifact

The first packageization scaffold for external std candidates is recorded in:

- `docs/design/phase-27.2-external-std-package-plan.v1.json`

That artifact is generated from the locked catalog and Phase 27 classification. It groups external-package candidates into deterministic first-wave and follow-up package targets so `27.2.0` can proceed from a canonical plan instead of ad hoc manual lists.

The first live Gate C cut routes `std::text` module imports (`std::bytes`, `std::str`, `std::str_pattern`), `std::int` module imports (`std::u64`, `std::u128`, `std::u256`), and `std::codec` module imports (`std::encoder`, `std::decoder`, `std::encode_error`, `std::decode_error`) through a bundled package overlay at the CLI module-loader boundary. This intentionally changes ownership at import/module resolution first while keeping runtime/codegen compatibility shims in place until the remaining Wave 1 cutover work is complete.

That bundled overlay is intentionally restricted to external-package-candidate Wave 1 modules. Verified-ABI boundaries such as `std::core`, `std::env`, `std::wasi`, `std::host`, `std::crypto`, and proof-critical collection modules remain excluded from the bundled package overlay and stay compiler-governed until an equivalent explicit ABI contract exists.

The active `27.2.2` migration contract is therefore:

1. The bundled package overlay itself is the explicit compatibility shim for moved Wave 1 module paths.
2. Loader/import diagnostics for moved Wave 1 symbols stay on stable existing build codes (`C021`, `C027`) but must name the bundled replacement package deterministically.
3. Legacy helper alias handling for moved Wave 1 text/int surfaces is centralized in one compiler shim catalog rather than duplicated across typer and conformance code paths.
