# Phase 29.5.1 - Chain-Adapter Rollout Shape Lock

Date: 2026-06-27
Status: Locked
Owner: std-packages-owner

## Purpose

Lock one canonical rollout shape for promoting chain-adapter packages into the supported shared-std
set.

This lock prevents per-chain drift in package boundary, migration behavior, and release/runtime
requirements. Every chain-adapter promotion after `29.5.0` must use this same bounded template.

## Canonical Package Boundaries

The bounded chain-adapter packages are:

### `std::eth`

- package id: `std::eth`
- module set:
  - `std::eth`
- supported value symbols:
  - `std::eth::from_bytes`
  - `std::eth::from_array`

### `std::solana`

- package id: `std::solana`
- module set:
  - `std::solana`
- supported value symbols:
  - `std::solana::from_bytes`
  - `std::solana::from_array`

### `std::cosmos`

- package id: `std::cosmos`
- module set:
  - `std::cosmos`
- supported value symbols:
  - `std::cosmos::from_bytes`
  - `std::cosmos::from_array`

No chain-adapter promotion in Phase 29 may expand beyond these exact module and value-symbol
boundaries.

## Type Surface Expectation

Each bounded chain-adapter module continues to carry its chain-scoped value type through module
metadata:

- `std::eth::Address`
- `std::solana::Pubkey`
- `std::cosmos::Addr`

This lock does not authorize additional constructors, mutators, serializers, equality helpers, or
IO wrappers beyond the exact value-symbol set above.

## Canonical Migration Diagnostics

When a moved chain-adapter surface is resolved through the bundled/shared package overlay,
diagnostics must name the replacement package deterministically.

For every bounded chain-adapter package:

1. module migration target must equal the package id
2. symbol migration target must equal the package id
3. diagnostics must remain fail closed and must not suggest fallback to `embedded`

Examples of the locked diagnostic form:

- module `std::eth` moved to bundled std package `std::eth`
- symbol `std::solana::missing` moved to bundled std package `std::solana`

The exact package-specific wording should follow the same deterministic style already used by:

- `std::text`
- `std::int`
- `std::sequence`
- `std::codec`
- `std::contract`

## Publication Shape

Every chain-adapter package must use the canonical shared-package publication shape:

1. package-specific publish stem:
   - `std-eth-<version>`
   - `std-solana-<version>`
   - `std-cosmos-<version>`
2. artifact path:
   - `std-packages/<publish-stem>.wasm`
3. canonical consumer metadata files:
   - `clg.package-metadata.json`
   - `clg.package-abi.json`
4. canonical signature, provenance, public-key, package-manifest, and publish-manifest outputs
5. deterministic registry layout under:
   - `std__eth/<version>/`
   - `std__solana/<version>/`
   - `std__cosmos/<version>/`

No package-specific special case is authorized.

## Proof, Release, And Runtime Constraints

Each chain-adapter promotion must satisfy all of the following before it becomes supported in
shared form:

1. typed/runtime status stays aligned with the current coverage matrix for the bounded constructor
   surface
2. proof status does not silently upgrade beyond the currently bounded support contract
3. constructor semantics remain pure and deterministic
4. invalid constructor input remains fail-closed under the current runtime contract
5. release/verify-bundle/runtime loading enforce the same trust, ABI, digest, provenance, and
   replay rules used by the currently supported shared packages
6. shared activation still requires explicit schema-v2 manifest intent and explicit `std.packages[]`

This means a chain-adapter shared promotion must not weaken:

- release gating
- runtime loader verification
- manifest/lock validation
- deterministic runtime diagnostics

## Shared-Vs-Embedded Compatibility Rules

The bounded rollout shape must preserve the current delivery model:

1. `embedded` remains the default supported production mode
2. `shared` remains explicit opt-in only
3. enabling one chain-adapter shared package does not implicitly enable the others
4. embedded compatibility for non-promoted chain adapters remains valid
5. no chain-adapter promotion may require broad migration of embedded projects

## Non-Goals

This lock does not authorize:

1. chain-specific IO namespaces such as `std::<chain>::io::*`
2. additional chain-specific helpers outside `from_bytes` / `from_array`
3. multi-chain bundle shortcuts that skip per-package evidence parity
4. changing the default delivery mode
5. expanding proof claims for chain adapters without a separate explicit proof lock

## Next Task

After this lock, the next task is `29.6.0`:

- promote the first selected chain-adapter package from the `29.5.0` order under this bounded
  rollout shape

## References

- `docs/design/phase-26.1.4.9-std-chain-target-first-production-lock.md`
- `docs/design/phase-27.2-external-std-package-plan.v1.json`
- `docs/design/phase-28.9.2-shared-std-product-contract-lock.md`
- `docs/design/phase-29.5.0-chain-adapter-production-ordering-lock.md`
- `docs/runtime/chain-packages.md`
- `docs/std/coverage-matrix.md`
