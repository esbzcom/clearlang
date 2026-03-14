# Phase 20.1.0 - Package Metadata v0 + ABI Contract v0

## Goal
Define the minimal package metadata schema and ABI contract schema required by
strict preflight in Phase 20.1.

Scope for this bootstrap slice is intentionally narrow:
- direct dependencies only,
- exact package identity and artifact linkage,
- deterministic ABI surface checks for imported symbols.

This v0 strict-preflight schema is separate from the existing Phase 18.4
`clg-packages.json` resolver metadata (`schema_version = 1`).

## Files and Scope
- Canonical metadata file: `clg.package-metadata.json`
- Canonical ABI file: `clg.package-abi.json`
- Location: module root for strict build input.
- Applicability: `clg build --compiler-mode strict` preflight only.
- Non-goal: transitive dependency metadata and ABI evolution policy (tracked in `22.0.3`).

## Metadata Schema (v0)

```json
{
  "schema_version": 0,
  "packages": [
    {
      "name": "std::core",
      "version": "1.0.0",
      "digest": "sha256:9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08",
      "artifact": {
        "format": "wasm",
        "path": "store/std-core-1.0.0.wasm"
      },
      "abi_id": "abi:std::core:1.0.0"
    }
  ]
}
```

### Required fields
- `schema_version`: integer, must equal `0`.
- `packages`: array of direct package entries (may be empty).
- Per package entry:
  - `name`: package identifier.
  - `version`: exact `MAJOR.MINOR.PATCH`.
  - `digest`: `sha256:<64 lowercase hex>`.
  - `artifact.format`: must be `wasm`.
  - `artifact.path`: non-empty relative path.
  - `abi_id`: non-empty string; must match an ABI contract entry in `clg.package-abi.json`.

### Rejected in v0
- Unknown top-level or package keys.
- Duplicate package identities (`name` duplicates).
- Non-exact semver versions.
- Non-`sha256` digests.
- Absolute artifact paths.

## ABI Contract Schema (v0)

```json
{
  "schema_version": 0,
  "contracts": [
    {
      "abi_id": "abi:std::core:1.0.0",
      "package": "std::core",
      "version": "1.0.0",
      "imports": [
        {
          "symbol": "std::core::math::add",
          "effect": "pure",
          "params": ["Int", "Int"],
          "ret": "Int",
          "capability": null
        },
        {
          "symbol": "std::core::wasi::print",
          "effect": "io",
          "params": ["Bytes"],
          "ret": "Int",
          "capability": "std::wasi::print"
        }
      ]
    }
  ]
}
```

### Required fields
- `schema_version`: integer, must equal `0`.
- `contracts`: array of ABI contracts (may be empty).
- Per contract:
  - `abi_id`: unique id referenced by metadata.
  - `package`: package name.
  - `version`: exact package version.
  - `imports`: import signature list.
- Per import signature:
  - `symbol`: fully qualified symbol name.
  - `effect`: one of `pure`, `mut`, `io`.
  - `params`: ordered list of type names.
  - `ret`: return type name.
  - `capability`: string or `null` (required host capability id when host-backed).

### Rejected in v0
- Unknown keys at any level.
- Duplicate `abi_id` or duplicate `(abi_id, symbol)`.
- `abi_id`/package/version mismatch between metadata and ABI file.
- Unsupported effect names.
- Empty symbol or type names.

## Determinism Rules
1. Internal ordering:
   - packages sorted by `name`,
   - contracts sorted by `abi_id`,
   - imports sorted by `symbol`.
2. Duplicate detection reports lexicographically first conflicting id.
3. Strict preflight checks are pure and input-driven:
   - metadata file bytes,
   - ABI file bytes,
   - lockfile/trust-policy/host-profile inputs.

## Integration With 20.1 Gates
- Metadata/schema gate (`20.1.2.4`):
  - missing/malformed/unsupported metadata or ABI schema fails with `C104`.
- ABI/link gate (`20.1.2.5`):
  - 20.1.0 bootstrap implementation enforces metadata <-> ABI contract identity consistency (`abi_id` presence + package/version alignment) and reports mismatches as `C105`.
  - full source import vs ABI symbol/effect/params/ret/capability matching remains in `20.1.2.5`.

## Diagnostics Mapping (Design-Locked Proposal)
- Metadata/ABI file missing or unreadable: `C104`.
- Metadata/ABI schema malformed or unsupported: `C104`.
- ABI contract mismatch for linked symbol: `C105`.

Final canonical code registration remains tracked under TODO `20.1.4.3`.

## Remediation (Bootstrap)
When strict preflight fails for metadata/ABI input:
1. ensure both files exist and are readable at module root:
   - `clg.package-metadata.json`
   - `clg.package-abi.json`
2. use schema v0 exactly (no unknown fields),
3. ensure metadata package entries and ABI contracts align by `abi_id` + `(package, version)`,
4. in bootstrap scope, fix metadata/ABI identity mismatches first; full per-symbol linker checks land in `20.1.2.5`.

## Non-Goals
- Transitive package metadata.
- Semver compatibility ranges.
- Automated ABI migration windows.
- Runtime loader resolution behavior.
