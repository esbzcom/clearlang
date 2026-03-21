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
   - migration completed in `22.1.5.1`: standard/permissive compiled-package indexing reads only canonical metadata/ABI inputs.
3. Legacy loader retirement rule:
   - legacy `clg-packages.json` is rejected in standard/permissive and strict flows (strict remains `C101`; standard/permissive reports deterministic build diagnostic `C027`).
4. Final production target:
   - canonical-only package metadata/ABI model is the sole accepted source-of-truth for package indexing.

## Implementation Notes (22.0.1)
- Module metadata loader enforces deterministic coexistence rejection.
- Integration coverage asserts coexistence conflict reports `C027`.

## References
- `docs/TODO.md` (`22.0.1`)
- `docs/design/phase-22.0.2-package-metadata-v1.md`
- `docs/design/phase-20.1.0-package-metadata-abi-v0.md`
- `docs/design/phase-18.4-compiled-package-imports.md`
