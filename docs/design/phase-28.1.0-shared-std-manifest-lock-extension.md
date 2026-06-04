# Phase 28.1.0 - Shared Std Manifest and Lock Extension

Date: 2026-06-04
Status: Locked
Owner: std-arch-owner

## Purpose

Extend the canonical ClearLang manifest and lockfile model to represent separately versioned shared std packages and verified-ABI requirements without overloading normal user package semantics.

## Design Decision

Shared std metadata must live in a dedicated `std` section.

It must **not** be merged into:

1. `clg.project.json.dependencies[]`
2. `clg.lock.json.roots[].dependencies[]`
3. `clg.lock.json.packages[]`

Reason:

- normal project/package dependencies and shared std distribution have different trust, ABI, and activation rules
- mixing them would make manifest intent ambiguous and weaken deterministic diagnostics
- Phase 28 needs a distinct std delivery contract, not a disguised reuse of user package roots

## Manifest Contract

### Schema

`clg.project.json` adds shared std support through schema v2.

Schema v1 remains valid for embedded-only projects.

Shared std inputs are invalid under schema v1.

### Canonical Shape

```json
{
  "schema_version": 2,
  "project": {
    "name": "app",
    "description": "ClearLang project",
    "version": "0.1.0",
    "clg_version": "^0.1.0",
    "entry": "main.clear",
    "website": "https://example.com",
    "contact": {
      "name": "Project Maintainer",
      "email": "maintainer@example.com"
    }
  },
  "dependencies": [
    { "name": "acme::payments", "requirement": "^1.4.0" }
  ],
  "std": {
    "delivery": "shared",
    "packages": [
      {
        "package_id": "std::text",
        "version_requirement": "^1.2.0",
        "verified_std_abi": {
          "major": 1,
          "minor_min": 3,
          "minor_max": 3
        },
        "registry": "default",
        "signer_policy": "std-publisher-prod",
        "allow_compat_shims": false
      }
    ]
  },
  "release_defaults": {
    "advisory_as_of": "REQUIRED_RFC3339_UTC",
    "key_id": "REQUIRED_KEY_ID",
    "out_dir": "out/release",
    "trust_policy": "trust-policy.json"
  }
}
```

### Locked Rules

1. `dependencies[]` remains user/package dependency intent only.
2. `std.delivery` is required when `std` is present.
3. `std.delivery` must be `embedded` or `shared`.
4. `std.packages[]` entries use the `shared_std_package_requirement` shape locked in `28.0.1`.
5. `std.packages[]` is:
   - optional and usually empty for `embedded`
   - required for `shared`
6. `std.packages[].package_id` must be Phase-27 external-package-capable std packages only.
7. `std.packages[]` must not contain duplicates by `package_id`.
8. Shared std package requirements must not appear in `dependencies[]`.

## Lockfile Contract

### Schema

`clg.lock.json` adds shared std support through schema v2.

Schema v1 remains valid for embedded-only lockfiles.

Shared std lock entries are invalid under schema v1.

### Canonical Shape

```json
{
  "schema_version": 2,
  "resolver_version": 1,
  "roots": [
    {
      "name": "app",
      "dependencies": [
        { "name": "acme::payments", "requirement": "^1.4.0" }
      ]
    }
  ],
  "packages": [
    {
      "id": "acme::payments@1.4.2",
      "name": "acme::payments",
      "version": "1.4.2",
      "digest": "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
      "abi_id": "abi:acme:payments:1.4.2",
      "dependencies": []
    }
  ],
  "std": {
    "delivery": "shared",
    "packages": [
      {
        "package_id": "std::text",
        "version": "1.2.0",
        "verified_std_abi": {
          "major": 1,
          "minor_min": 3,
          "minor_max": 3
        },
        "artifact": {
          "format": "wasm",
          "path": "std-packages/std-text-1.2.0.wasm",
          "digest": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
          "size_bytes": 24576
        },
        "signature": {
          "key_id": "std-publisher-ed25519-2026q2",
          "algorithm": "ed25519",
          "signed_at": "2026-06-04T00:00:00Z",
          "signature": "BASE64_SIGNATURE"
        },
        "provenance": {
          "statement_digest": "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
          "statement_format": "in-toto-v1"
        },
        "symbols": [
          "std::bytes",
          "std::str",
          "std::str_pattern"
        ],
        "dependencies": []
      }
    ]
  }
}
```

### Locked Rules

1. `roots[]` and `packages[]` remain the canonical non-std dependency graph.
2. Shared std lock entries live only under `std.packages[]`.
3. `std.delivery` is required in schema v2 lockfiles.
4. `std.packages[]` entries use the `shared_std_package_lock` shape locked in `28.0.1`.
5. `std.packages[]` ordering is deterministic by `package_id@version`.
6. `std.packages[].dependencies[]` ordering is deterministic by exact locked id.
7. Shared std lock entries do not participate in normal package id collision space under `packages[]`.

## Embedded-Only Compatibility

The extension is intentionally additive for embedded-only projects.

Locked compatibility rules:

1. A project using only embedded std may stay on manifest schema v1 and lockfile schema v1.
2. A project that declares `std.delivery = shared` must use manifest schema v2.
3. A lockfile that records shared std identities must use lockfile schema v2.
4. Schema v1 plus shared std intent is invalid and must fail closed.

## Drift and Resolver Intent

Shared std intent is part of canonical root state once schema v2 is used.

That means later `28.1.1` validation must treat all of the following as lock drift:

1. `std.delivery` mismatch between manifest and lockfile
2. manifest `std.packages[]` vs lockfile `std.packages[]` root-intent mismatch
3. shared std ABI requirement mismatch
4. shared std package duplication or ordering drift

## Release and Runtime Boundary

This task only locks manifest/lock representation.

It does not yet:

1. resolve shared std packages
2. validate shared std trust/signature data
3. load shared std artifacts at runtime
4. project shared std identities into release manifests

Those belong to:

1. `28.1.1`
2. `28.2.x`
3. `28.3.x`

## References

- `docs/design/phase-25.4.1-project-manifest-v1.md`
- `docs/design/phase-25.4.2-lockfile-tool-generated-contract.md`
- `docs/design/phase-25.4.5-manifest-lock-schema-migration-and-drift-gate.md`
- `docs/design/phase-28.0-shared-std-distribution-design-lock.md`
