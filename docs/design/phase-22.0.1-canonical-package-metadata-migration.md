# Phase 22.0.1 - Canonical Package Metadata Migration

## Status
Design and migration lock for `22.0.1` in `docs/TODO.md`.

## Goal
Define one canonical production package metadata model and an explicit deprecation path for legacy `clg-packages.json`.

## Canonical Model (Production Target)
1. Identity/trust metadata:
   - `clg.package-metadata.json`
2. Symbol/effect/capability ABI contract:
   - `clg.package-abi.json`

Legacy model:
- `clg-packages.json` (Phase 18.4 resolver metadata) is compatibility-only.

## Migration Lock
1. Strict mode:
   - legacy `clg-packages.json` remains forbidden (`C101`) as source-of-truth violation.
2. Standard/permissive mode transition:
   - legacy metadata remains readable for compatibility while 22.x implementation converges.
3. Deterministic coexistence rule:
   - if both legacy `clg-packages.json` and canonical `clg.package-metadata.json` are present in the same module root, fail closed with deterministic build diagnostic (`C027` during transition).
4. Final production target:
   - remove legacy `clg-packages.json` resolver path after 22.x migration gates are complete.

## Implementation Notes (22.0.1)
- Module metadata loader enforces deterministic coexistence rejection.
- Integration coverage asserts coexistence conflict reports `C027`.

## References
- `docs/TODO.md` (`22.0.1`)
- `docs/design/phase-22.0.2-package-metadata-v1.md`
- `docs/design/phase-20.1.0-package-metadata-abi-v0.md`
- `docs/design/phase-18.4-compiled-package-imports.md`

