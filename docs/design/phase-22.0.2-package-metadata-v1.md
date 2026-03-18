# Phase 22.0.2 - Package Metadata v1

## Status
Design lock target for `22.0.2` in `docs/TODO.md`.

## Goal
Define one canonical production package metadata schema for build/run resolution and strict trust checks.

## Canonical File
- Filename: `clg.package-metadata.json`
- Schema version: `1`
- Unknown keys: rejected (`deny_unknown_fields` policy)

## Schema (v1)
```json
{
  "schema_version": 1,
  "packages": [
    {
      "name": "std::core",
      "version": "1.0.0",
      "digest": "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
      "artifact": {
        "format": "wasm",
        "path": "packages/std-core-1.0.0.wasm"
      },
      "abi_id": "std-core-abi-v1",
      "signature": {
        "format": "ed25519",
        "key_id": "std-core-release",
        "signed_at": "2026-01-15T00:00:00Z",
        "signature": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
      },
      "trust": {
        "trusted_anchor_ids": ["core-root-2026"]
      },
      "dependencies": [
        {
          "name": "std::host",
          "requirement": "=1.0.0"
        }
      ]
    }
  ]
}
```

## Validation Lock
1. `name` must be valid package id (same lexical rules as strict lockfile package names).
2. `version` must be exact semver `MAJOR.MINOR.PATCH` for resolved package entries.
3. `digest` must be `sha256:<64 lowercase hex>`.
4. `artifact.format` must be `wasm` in Phase 22.
5. `artifact.path` must be relative (no absolute path, no traversal outside configured root).
6. `abi_id` must be non-empty and unique across packages.
7. `signature.format` must be `ed25519`; `signed_at` must be RFC3339 UTC; `signature` must be 64-byte lowercase hex (128 chars).
8. `trusted_anchor_ids` must be non-empty and each id must exist in trust policy.
9. `dependencies` entries are package requirements (not resolved pins); they feed resolver input.

## Determinism Lock
1. Internal evaluation ordering is canonical:
   - package order by `name`, then `version`.
   - dependency order by `name`.
2. Duplicate detection reports lexicographically smallest offending key first.
3. Canonical serialization for emitted artifacts uses sorted keys and sorted arrays by the ordering above.

## Migration Policy (22.0.1 linkage)
- `clg-packages.json` is legacy and becomes deprecated once metadata v1 is implemented.
- During transition, if both legacy and canonical files are present, build fails closed with a deterministic build diagnostic.
- Final production path: only canonical metadata v1 is accepted in strict and standard package flows.

## References
- `docs/TODO.md` (`22.0.1`, `22.0.2`)
- `docs/design/phase-20.1.0-package-metadata-abi-v0.md`
- `docs/design/phase-22.0.3-lockfile-v1.md`
