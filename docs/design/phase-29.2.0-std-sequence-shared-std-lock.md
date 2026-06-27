# Phase 29.2.0 - `std::sequence` Shared-Std Lock

Date: 2026-06-27
Status: Locked
Owner: std-packages-owner

## Purpose

Lock the bounded contract for promoting `std::sequence` into the supported shared-std package set.

This lock defines the exact package boundary before implementation begins so publication, lock
generation, release evidence, runtime loading, and migration diagnostics can stay deterministic.

## Supported Package Identity

The package id is:

- `std::sequence`

This package is promoted alongside the existing supported shared packages:

- `std::text`
- `std::int`
- `std::codec`

No other package ids are added by this lock.

## Supported Module Set

The `std::sequence` shared package contains exactly:

- `std::array`
- `std::slice`

## Supported Symbol Set

The supported shared symbol set is exactly:

- `std::array::len`
- `std::slice::get`
- `std::slice::len`
- `std::slice::subslice`

Anything outside this set is not implicitly included by the `std::sequence` promotion.

In particular, this lock does not expand the package to cover broader collection/list/set/map
surfaces or additional array/slice helpers beyond the current Phase 27 plan.

## Publication Shape

`std::sequence` uses the same canonical publication contract as the current supported shared
packages:

1. package-specific publish stem: `std-sequence-<version>`
2. package artifact path: `std-packages/std-sequence-<version>.wasm`
3. canonical consumer metadata files:
   - `clg.package-metadata.json`
   - `clg.package-abi.json`
4. canonical signature, provenance, public-key, package-manifest, and publish-manifest outputs
5. deterministic registry layout under:
   - `std__sequence/<version>/`

No package-specific special case is authorized.

## Activation And Compatibility Rules

`std::sequence` follows the existing supported shared-std activation contract:

1. shared activation requires schema v2 plus explicit `std.delivery = "shared"`
2. projects must opt in explicitly through `std.packages[]`
3. no silent fallback from `shared` to `embedded` is allowed
4. unsupported package ids remain fail-closed errors
5. release, verify-bundle, and runtime loading must enforce the same trust, ABI, digest,
   provenance, and replay rules as the existing supported shared packages

## Migration Diagnostics

When moved helper surfaces are referenced through the bundled/shared package overlay, diagnostics
must name the replacement package deterministically:

- module migration target: `std::sequence`
- symbol migration target: `std::sequence`

The implementation must preserve the existing deterministic style used for:

- `std::text`
- `std::int`
- `std::codec`

and extend it to:

- `std::array`
- `std::slice`

## Non-Goals

This lock does not authorize:

1. packageizing `std::list`, `std::set`, or `std::map`
2. promoting `std::contract` or chain packages in the same slice
3. changing the shared-std default behavior
4. expanding `std::sequence` beyond the current Phase 27 module/symbol plan

## Implementation Target

After this lock, the next implementation task is:

- `29.2.1` extend allowlists, publication, and manifest/lock validation to support
  `std::sequence`

## References

- `docs/design/phase-27.2-external-std-package-plan.v1.json`
- `docs/design/phase-29.1.0-first-post-m3-slice-selection.md`
- `docs/design/phase-28.9.2-shared-std-product-contract-lock.md`
