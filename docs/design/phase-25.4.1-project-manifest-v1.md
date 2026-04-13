# Phase 25.4.1 - `clg.project.json` Project Manifest v1

## Status
Design + implementation lock for `25.4.1` in `docs/TODO.md`.

## Goal
Define `clg.project.json` as the user-authored project/dependency manifest with Maven-like project metadata while keeping deterministic Gate E lockfile behavior.

## Release UX Direction (Pre-GA)
Before official GA release, project policy is manifest-first and compatibility-minimal:
- `clg.project.json` is the primary source for release defaults and dependency roots.
- Long non-secret release parameters are transitional only and should be retired once manifest coverage is complete.
- No long-term backward-compatibility commitment is required for pre-GA legacy parameter surfaces.

Target release UX after Gate E closure:
- canonical flow: `clg release --key <FILE> --pubkey <FILE> --root <DIR>`
- optional overrides only for exceptional/operator cases
- sensitive key material remains explicit by policy (`--key`, `--pubkey`)

## Design Principles Check
- Simple for users: one user-authored manifest now holds project identity, contact metadata, dependency requirements, and release defaults.
- AI-friendly: schema is explicit, versioned, and validated fail-closed with deterministic diagnostics.
- Provably correct: dependency requirements and compiler-version requirements are validated before resolver execution.
- Crypto-focused: lock generation stays deterministic and fail-closed; release defaults remain explicit policy inputs.

## Schema v1
`clg.project.json` schema v1:

```json
{
  "schema_version": 1,
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
    { "name": "std::core", "requirement": "^1.0.0" }
  ],
  "release_defaults": {
    "advisory_as_of": "REQUIRED_RFC3339_UTC",
    "key_id": "REQUIRED_KEY_ID",
    "out_dir": "out/release",
    "trust_policy": "trust-policy.json"
  }
}
```

## Validation Rules
1. `schema_version` accepts `0` (legacy release-defaults bootstrap) and `1` (project manifest v1).
2. `project.version` must be exact semver; `project.clg_version` must be a valid semver requirement.
3. `project.website` must be `http://` or `https://` with no whitespace.
4. `project.contact.email` must be email-like and non-empty.
5. `dependencies[]` entries must have valid package ids and semver requirements.
6. `dependencies[]` must not contain duplicate package names.
7. `project.entry`, `release_defaults.out_dir`, and `release_defaults.trust_policy` must be relative non-traversing paths.

## Resolver Contract
- `clg pkg lock --generate|--update` now reads dependency roots from `clg.project.json` schema v1 when present.
- The lockfile remains tool-generated (`clg.lock.json`) with exact versions/digests.
- If schema v1 is absent, current fallback behavior remains:
  - `--update` roots are taken from existing lockfile.
  - `--generate` roots are derived from canonical package metadata.

## Compatibility
- Schema v0 `clg.project.json` remains accepted for `release_defaults` compatibility.
- Schema v1 is the preferred format for Gate E project management.
- Pre-GA policy: legacy schema/flag compatibility is temporary migration debt and is scheduled for explicit fail-closed removal in pending `25.4.x` tasks.

## References
- `crates/cli/src/commands/release_defaults.rs`
- `crates/cli/src/commands/pkg/lock_command.rs`
- `crates/cli/tests/cli_it/pkg_lock/core.rs`
- `crates/cli/tests/cli_it/basic/build_release.rs`
- `docs/TODO.md`
