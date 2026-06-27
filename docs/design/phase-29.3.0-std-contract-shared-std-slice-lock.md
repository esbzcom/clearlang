# Phase 29.3.0 - `std::contract` Shared-Std Slice Lock

Date: 2026-06-27
Status: Locked
Owner: std-packages-owner

## Purpose

Lock the bounded Wave 2 contract for promoting `std::contract` into the supported shared-std
package set.

This lock defines the exact contract-domain package boundary before implementation begins so
publication, lock generation, release evidence, runtime loading, and migration diagnostics remain
deterministic and fail closed.

## Supported Package Identity

The package id is:

- `std::contract`

This package is promoted after the current supported shared packages:

- `std::text`
- `std::int`
- `std::sequence`
- `std::codec`

No other package ids are added by this lock.

## Supported Module Set

The `std::contract` shared package contains exactly:

- `std::contract`
- `std::contract::address`
- `std::contract::amount`
- `std::contract::contract_error`
- `std::contract::event`

Anything outside this module set is not implicitly included by the `std::contract` promotion.

## Supported Symbol Set

The supported shared symbol set is exactly:

- `std::contract::address::equals`
- `std::contract::address::from_bytes`
- `std::contract::address::to_bytes`
- `std::contract::amount::add_checked`
- `std::contract::amount::from_u64`
- `std::contract::amount::is_zero`
- `std::contract::amount::sub_checked`
- `std::contract::amount::value`
- `std::contract::contract_error::code`
- `std::contract::contract_error::equals`
- `std::contract::event::new`
- `std::contract::event::payload`
- `std::contract::event::topic`

This symbol set is intentionally aligned with the first-production chain-agnostic `std::contract`
API lock.

Anything outside this set is not implicitly included by the `std::contract` promotion.

## Domain Boundary Rules

`std::contract` is a chain-agnostic domain-surface package.

The shared promotion must preserve these boundary rules:

1. address parsing/serialization stays chain-agnostic at the `std::contract` layer
2. amount arithmetic remains deterministic and fail closed on overflow/underflow
3. event topic/payload accessors preserve bytes exactly
4. chain-specific address, event, or ABI semantics stay out of `std::contract`

That means packageizing `std::contract` does not authorize moving chain-target behavior into this
package.

## Publication Shape

`std::contract` uses the same canonical publication contract as the current supported shared
packages:

1. package-specific publish stem: `std-contract-<version>`
2. package artifact path: `std-packages/std-contract-<version>.wasm`
3. canonical consumer metadata files:
   - `clg.package-metadata.json`
   - `clg.package-abi.json`
4. canonical signature, provenance, public-key, package-manifest, and publish-manifest outputs
5. deterministic registry layout under:
   - `std__contract/<version>/`

No package-specific special case is authorized.

## Activation And Compatibility Rules

`std::contract` follows the existing supported shared-std activation contract:

1. shared activation requires schema v2 plus explicit `std.delivery = "shared"`
2. projects must opt in explicitly through `std.packages[]`
3. no silent fallback from `shared` to `embedded` is allowed
4. unsupported package ids remain fail-closed errors
5. release, verify-bundle, and runtime loading must enforce the same trust, ABI, digest,
   provenance, and replay rules as the existing supported shared packages

Promoting `std::contract` does not change the default delivery mode:

- `embedded` remains a supported default production mode
- `shared` remains an explicit opt-in production mode

## Migration Diagnostics

When moved contract-domain surfaces are referenced through the bundled/shared package overlay,
diagnostics must name the replacement package deterministically:

- module migration target: `std::contract`
- symbol migration target: `std::contract`

The implementation must preserve the existing deterministic migration style already used for:

- `std::text`
- `std::int`
- `std::sequence`
- `std::codec`

and extend it to:

- `std::contract`
- `std::contract::address`
- `std::contract::amount`
- `std::contract::contract_error`
- `std::contract::event`

## Trust, ABI, And Provenance Rules

`std::contract` must satisfy the same shared-package trust chain as the current supported package
set:

1. publication signs the exact artifact and metadata emitted for the package version
2. lockfiles pin package identity, version, ABI range, digest, signer, and provenance inputs
3. release and verify-bundle evidence must carry the selected `std::contract` shared package under
   the same schema-v2 fail-closed rules as other supported packages
4. runtime loading must reject missing artifacts, digest mismatch, ABI mismatch, untrusted signers,
   and replay/provenance mismatch without fallback behavior

No weaker trust or verification path is authorized for `std::contract`.

## Explicit Deferrals

This lock intentionally defers all chain-adapter shared-package work to Phase `29.5+`.

In particular, this lock does not authorize:

- `std::eth`
- `std::solana`
- `std::cosmos`

It also does not authorize mixing chain-adapter symbols into the `std::contract` package boundary.

## Non-Goals

This lock does not authorize:

1. changing the shared-std default behavior
2. broadening `std::contract` beyond the current first-production symbol set
3. promoting any chain-adapter package in the same slice
4. weakening fail-closed release, trust, ABI, provenance, or replay behavior to simplify rollout

## Implementation Target

After this lock, the next implementation task is:

- `29.4.0` extend allowlists, publication, and manifest/lock validation to support
  `std::contract`

## References

- `docs/design/phase-26.1.4.7-std-contract-first-production-lock.md`
- `docs/design/phase-27.2-external-std-package-plan.v1.json`
- `docs/design/phase-28.9.2-shared-std-product-contract-lock.md`
- `docs/design/phase-29.1.0-first-post-m3-slice-selection.md`
