# Phase 22.0.4 - Package Metadata and ABI Compatibility Policy

## Status
Design lock target for `22.0.4` in `docs/TODO.md`.

## Goal
Define deterministic compatibility rules for package metadata, package ABI, and lockfile schema evolution.

## Current Compatibility Matrix (Locked)
| Artifact | Accepted schema versions | Rejected schema versions | Rejection diagnostic |
|---|---|---|---|
| `clg.lock.json` | `0`, `1` | anything else | `C104` |
| `clg.package-metadata.json` | `0`, `1` | anything else | `C104` |
| `clg.package-abi.json` | `0` | anything else | `C104` |

## Schema Evolution Policy
1. New schema versions must be introduced with explicit design lock documentation and regression tests before runtime use.
2. Unknown/unsupported versions fail closed with deterministic diagnostics (`C104`).
3. Compatibility windows are additive-first:
   - add support for new schema version,
   - keep previous accepted version(s) during migration window,
   - remove old version only in a later, explicit policy update with regression coverage updates.

## Migration and Deprecation Guarantees
1. Existing accepted schemas do not silently change behavior in strict mode; changes require a documented phase lock update.
2. When a schema is deprecated, diagnostics remain deterministic and machine-stable.
3. Build acceptance ordering remains deterministic regardless of file field ordering.

## Regression Gate (22.0.4)
- `crates/cli/tests/schema_compatibility_policy.rs` locks strict-mode schema compatibility behavior:
  - accepts lockfile schema `1`,
  - accepts package metadata schema `0`,
  - accepts package metadata schema `1`,
  - rejects package metadata schema `2` with `C104`,
  - rejects package ABI schema `1` with `C104`.

## References
- `docs/TODO.md` (`22.0.4`)
- `docs/design/phase-22.0.1-canonical-package-metadata-migration.md`
- `docs/design/phase-22.0.2-package-metadata-v1.md`
- `docs/design/phase-22.0.3-lockfile-v1.md`
