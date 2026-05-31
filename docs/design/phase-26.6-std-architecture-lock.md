# Phase 26.6 - Std Architecture Lock (Gate G)

## Scope
Lock a long-term std architecture that removes unmanaged symbol drift across docs, typer, CLI metadata, and codegen routing.

## Canonical Source-Of-Truth
- Canonical machine-readable catalog: `docs/design/phase-26.6-std-catalog.lock.json`.
- Catalog ownership:
  - module-level class: `pure_std` or `host_std`
  - export-level contract for values: `arity`, `effect`, `route`, optional `capability`, optional `deprecated_alias_of`
  - export-level contract for types: `layout`
- Generated artifacts:
  - `crates/cli/assets/std-metadata.json`
  - `docs/design/phase-26.6-std-builtin-signatures.v1.json`

## Generation + Gate Commands
- Sync/regenerate artifacts from the canonical catalog:
  - `cargo run -p xtask -- std-arch-sync --write`
- Refresh the canonical catalog from currently shipped runtime/typer sources (maintainer operation):
  - `cargo run -p xtask -- std-arch-sync --refresh-lock --write`
- Fail-closed conformance gate:
  - `cargo run -p xtask -- std-arch-conformance-check`
- Legacy gate status:
  - `std-surface-drift-check` remains as historical Phase 21 tooling and is superseded by Gate G checks.

## Conformance Guarantees
`std-arch-conformance-check` fails closed on drift for:
1. Canonical catalog vs emitted std metadata.
2. Canonical catalog vs typer builtin signature surface (`symbol`, `arity`, `effect`).
3. Canonical catalog vs codegen intrinsic/package routing.
4. Canonical catalog vs std coverage-matrix symbol coverage.
5. Canonical catalog host capabilities vs canonical host capability policy.

## Host Boundary Lock
- `host_std` ownership is explicitly locked to `std::crypto`, `std::env`, and `std::wasi`.
- `std::env::chain_id` remains an approved compatibility-only host-policy capability path while typed std metadata remains centered on `std::env::{time,random}`.
- Any new compatibility-only host capability path must be explicitly allowlisted and documented.

## Deprecation/Cutover Policy
- Compatibility aliases remain callable but are catalog-declared through `deprecated_alias_of`.
- Alias lifecycle requirements:
  1. Catalog entry + migration note.
  2. Deterministic diagnostics for removals/retargeting.
  3. Conformance gate remains green before release.

## Gate G Completion Criteria
Gate G is considered closed when all are true:
1. No unmanaged std value symbol exists outside catalog/governed generators.
2. `std-arch-conformance-check` is green in CI.
3. `std-metadata` + builtin-signature artifact are generated from canonical catalog and deterministic across repeated runs.
4. Host boundary lock and compatibility-only exceptions are explicit and versioned.
