# Phase 25.4.4 - Version-Free Imports in `.clear`

## Status
Design lock + implementation record for `25.4.4` in `docs/TODO.md`.

## Goal
Keep `.clear` import/module paths purely logical (`pkg::module`), with all version selection centralized in:
- `clg.project.json` (declared requirements)
- `clg.lock.json` (resolved exact versions/digests)

## Contract
1. Source import paths are version-free only.
2. Versioned forms such as `import vendor::crypto@1.2.3::hash;` are rejected with deterministic diagnostics.
3. Resolver/version decisions must come from manifest + lockfile flows (`clg pkg lock --generate|--update`), not from source text.

## Diagnostics
- Parser emits `P015` for versioned imports:
  - "versioned imports are not supported in source; keep import paths version-free and declare versions in `clg.project.json`/`clg.lock.json`"

## Evidence
- parser heuristic and structured error mapping:
  - `crates/parser/src/program/heuristics.rs`
  - `crates/parser/src/program.rs`
- regression test:
  - `crates/parser/tests/parse_structured_errors.rs` (`parse_errors_reports_versioned_import_as_p015`)
- diagnostics table update:
  - `docs/diagnostics.md`
