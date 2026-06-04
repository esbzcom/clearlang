# Phase 28.1.1 - Shared Std Resolver and Lock Validation

Date: 2026-06-04
Status: Locked
Owner: std-arch-owner

## Purpose

Define the deterministic validation rules for shared std manifest intent and lockfile identity before runtime loading is implemented.

This phase covers validation only:

1. manifest intent shape
2. lockfile identity shape
3. manifest-to-lock consistency
4. deterministic diagnostics for violations

It does not yet implement runtime activation.

## Validation Scope

Validation applies whenever either of the following is true:

1. `clg.project.json` uses schema v2 and includes a `std` section
2. `clg.lock.json` uses schema v2 and includes a `std` section

Embedded-only schema v1 files remain valid outside this scope.

## Manifest Validation Rules

For `clg.project.json` schema v2:

1. `std.delivery` is required.
2. `std.delivery` must be exactly `embedded` or `shared`.
3. `std.packages[]` entries must be unique by `package_id`.
4. every `std.packages[].package_id` must be a Phase-27 external-package-capable std package id.
5. every `std.packages[].version_requirement` must be a valid semver requirement.
6. every `std.packages[].verified_std_abi` must be present and valid:
   - `major` integer >= 1
   - `minor_min` integer >= 0
   - `minor_max` integer >= `minor_min`
7. optional `registry` must be non-empty if present.
8. optional `signer_policy` must be non-empty if present.
9. `allow_compat_shims` defaults to `false`; if present it must be a boolean.
10. `std.delivery = embedded` must not declare non-empty `std.packages[]`.
11. `std.delivery = shared` must declare at least one `std.packages[]` entry.

## Lockfile Validation Rules

For `clg.lock.json` schema v2:

1. `std.delivery` is required.
2. `std.delivery` must be exactly `embedded` or `shared`.
3. `std.packages[]` entries must be unique by exact `package_id@version`.
4. `std.packages[]` ordering must be deterministic by `package_id@version`.
5. every locked shared std package must satisfy the `shared_std_package_lock` contract from `28.0.1`.
6. `artifact.digest` must use canonical `sha256:<hex>` form.
7. `artifact.path` must be relative and non-traversing.
8. `artifact.size_bytes` must be a positive integer.
9. `signature.key_id`, `signature.algorithm`, `signature.signed_at`, and `signature.signature` are required and non-empty.
10. `provenance.statement_digest` and `provenance.statement_format` are required and non-empty.
11. `symbols[]` must be deterministic and sorted.
12. `dependencies[]` must be deterministic and sorted.
13. `std.delivery = embedded` must not carry shared std package lock entries.
14. `std.delivery = shared` must carry at least one shared std package lock entry.

## Manifest-to-Lock Consistency Rules

When both manifest schema v2 and lockfile schema v2 are present:

1. `std.delivery` must match exactly.
2. manifest `std.packages[].package_id` set must match lockfile `std.packages[].package_id` set.
3. every locked shared std package version must satisfy the manifest `version_requirement`.
4. every locked shared std package ABI claim must satisfy the manifest `verified_std_abi` requirement.
5. `allow_compat_shims = false` in the manifest forbids lock entries that require compatibility-shim dependence.
6. manifest shared std packages must not appear in normal lockfile `packages[]`.
7. lockfile shared std packages must not appear in normal manifest `dependencies[]`.

## Determinism Rules

Validation output must be deterministic across identical inputs.

That means:

1. duplicate detection reports the lexicographically first offending package id
2. ordering drift reports the first offending position in canonical order
3. manifest/lock mismatch reports canonical sorted sets, not discovery-order sets
4. repeated runs over identical inputs must produce identical error ordering and bytes under `--json-errors`

## Diagnostic Mapping

Shared std validation reuses the existing deterministic metadata/lock error families:

1. `C027`
   - malformed shared std manifest/metadata shape in permissive/standard indexing flows
2. `C109`
   - shared std manifest/lock coexistence or model mismatch
3. `C111`
   - shared std lockfile input contract failure during generate/update flows
4. `C104`
   - unsupported schema version in strict shared std validation paths, if/when those paths consume schema-gated strict inputs

The implementation must not introduce ambiguous generic messages when one of these existing deterministic families applies.

## Canonical Failure Cases

The following cases are explicitly locked as validation failures:

1. manifest declares `std.delivery = shared` but omits `std.packages[]`
2. manifest declares the same shared std package twice
3. manifest declares a non-externalizable std package id under `std.packages[]`
4. manifest declares invalid ABI range (`minor_max < minor_min`)
5. lockfile carries `std.delivery = embedded` but also contains `std.packages[]`
6. lockfile carries unsorted or duplicate shared std package entries
7. lockfile shared std version does not satisfy manifest requirement
8. lockfile shared std ABI claim does not satisfy manifest ABI requirement
9. shared std package is present in both normal package dependency space and std package space
10. manifest and lockfile disagree on shared-vs-embedded delivery mode

## Follow-on Work

This validation lock feeds directly into:

1. `28.1.2` coexistence diagnostics
2. `28.2.0` artifact discovery
3. `28.2.1` ABI validation before activation

## References

- `docs/design/phase-25.4.1-project-manifest-v1.md`
- `docs/design/phase-25.4.2-lockfile-tool-generated-contract.md`
- `docs/design/phase-28.0-shared-std-distribution-design-lock.md`
- `docs/design/phase-28.1.0-shared-std-manifest-lock-extension.md`
